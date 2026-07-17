# SPEC-013 — Lifecycle, Ingest & Dumps

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | Phase 11 · phase11_lifecycle-ingest-dumps §0–§6 ([tasks](../../.rulebook/tasks/phase11_lifecycle-ingest-dumps/tasks.md)) |
| **Planning source** | [Elastic plan P1 #9 ILM-lite (F-021, F-050 #8), P1 #10 ingest (F-024)](../analysis/elastic/08-execution-plan.md) · [Meilisearch plan R-09 dumps (F-021), R-11 webhooks (F-020), A-06](../analysis/meilisearch/08-execution-plan.md) · phase11 code audit (rollover stats fabricated `num_docs * 1024` / `age_in_millis = 0`; `RolloverService` never started; `is_write_index` stored but unenforced; `datastream.rs` dead; snapshot repository writes mock data at `snapshot/repository.rs:854,903`) |

Requirement IDs `LCM-xxx`. RFC 2119 keywords are normative. Errors use SPEC-003. Dumps, dump imports, and rollovers run as SPEC-002 tasks; webhooks fire on SPEC-002 task completion. Rollover size conditions consume the real `IndexStats.size_in_bytes` of SPEC-008 OPS-003. `defaultPipeline` and template stamping come from SPEC-006. Per-item ingest failures follow SPEC-005 `_bulk` semantics.

## 1. Model

Four capabilities that make Lexum operable unattended:

1. **ILM-lite** — named lifecycle policies: rollover conditions + a delete phase, evaluated by a background service. Rollover is *the* growth story (1 primary shard + rollover, anti-goal F-050 #8).
2. **Write aliases / data streams** — an alias with exactly one write index; rollover atomically advances the write index through generations.
3. **Ingest-pipeline-lite** — a small processor chain applied on the write path.
4. **Logical dumps** — a version-portable export/import, distinct from binary snapshots.
5. **Task webhooks** — push notification on task completion.

## 2. ILM-lite policies

### 2.1 Policy resource

`PUT/GET/DELETE /_ilm/policy/{name}` — persisted. The policy object is deliberately tiny:

```json
{
  "policy": {
    "rollover": { "max_age": "7d", "max_size": "10gb", "max_docs": 10000000 },
    "delete": { "min_age": "30d" }
  }
}
```

- **LCM-001** `rollover` accepts any non-empty subset of `max_age` (ES duration string: `ms|s|m|h|d`), `max_size` (ES size string: `b|kb|mb|gb|tb`), `max_docs` (integer). Conditions are OR-ed: any one satisfied triggers rollover. `delete.min_age` is the time since an index was *rolled over* (ceased being the write index) after which it is deleted.
- **LCM-002** Both sections are optional but a policy MUST contain at least one. Unknown keys → SPEC-003 400. `PUT` of an existing name replaces it (affects future evaluations only); `DELETE` of a policy still attached to indices MUST fail 409 naming the indices, unless `?force=true` detaches it.
- **LCM-003** Policies attach to indices either directly (index setting `lifecycle.name`) or stamped via index templates (SPEC-006 SET-082) so every rolled-over generation inherits the policy automatically.

### 2.2 Condition evaluation — real inputs only

- **LCM-010** Condition inputs MUST be real measurements: `max_docs` from the live Tantivy doc count, `max_size` from `IndexStats.size_in_bytes` (SPEC-008 OPS-003), `max_age` from a **persisted index-creation timestamp**. The audited fabrications (`size_in_bytes = num_docs * 1024`, `age_in_millis = 0`, `num_segments * 1_000_000`) are non-conformant and MUST be removed.
- **LCM-011** Rollover conditions and policy attachments MUST be persisted and survive restart (today's `get/update_rollover_conditions` no-ops are non-conformant).
- **LCM-012** Manual `POST /{alias}/_rollover` (with optional inline conditions and `?dry_run=true`) remains supported and MUST share one code path with the background evaluator — exactly one rollover implementation may exist.

### 2.3 Background evaluator

- **LCM-020** A background lifecycle service MUST be started at server startup (the existing never-started `RolloverService` is non-conformant until wired), evaluating every attached policy on a configurable interval, default **60 s**, with graceful shutdown.
- **LCM-021** Each evaluation tick, per managed index: (a) if it is a write index and any rollover condition holds → execute rollover (§3); (b) if it was rolled over ≥ `delete.min_age` ago → delete the index and atomically remove its alias entries. Delete supports a config-level dry-run/log-only mode.
- **LCM-022** Evaluator actions run as SPEC-002 tasks (visible in `/tasks`, webhook-eligible per §7). A failed action MUST be retried on the next tick; failures never stop the evaluator loop.
- **LCM-023** `GET /{index}/_ilm/explain` MUST report: attached policy name, current phase (`hot` = is/was never rolled over and is the write index; `rolled_over` with the rollover timestamp; `delete_pending`), measured age/size/docs, and the next action the evaluator would take.

## 3. Write aliases & data-stream semantics

- **LCM-030** An alias MAY designate exactly one member as `is_write_index: true`. Writes addressed to an alias MUST resolve to that single write index. Zero or multiple write indices → SPEC-003 400 error naming the alias (the audit found the flag stored but never resolved — writes to aliases were ambiguous).
- **LCM-031** Reads via an alias fan out over **all** member indices (all generations).
- **LCM-032** Generation naming: rollover of write index `<name>-000001` creates `<name>-000002` (zero-padded 6-digit suffix, incremented). A target name MAY be supplied to manual rollover; auto-rollover always uses generation naming.
- **LCM-033** Rollover MUST be atomic with respect to writers: create the new index (stamped by matching templates, SPEC-006 §8), then in one atomic alias operation (`execute_atomic_operations`) move `is_write_index` from old to new. Every concurrent write lands in exactly one generation — never lost, never duplicated across generations.
- **LCM-034** The rollover response records `old_index`, `new_index`, `rolled_over: bool`, `dry_run: bool`, and the per-condition evaluation map (`{"[max_docs: 10000000]": true, ...}`), ES-shaped. The rollover timestamp MUST be persisted on the old index (input to `delete.min_age`, LCM-021).

## 4. Ingest-pipeline-lite

### 4.1 Pipeline resource

`PUT/GET/DELETE /_ingest/pipeline/{id}` — persisted. Shape:

```json
{
  "description": "parse web logs",
  "processors": [
    { "dissect": { "field": "message", "pattern": "%{ip} - - [%{ts}] \"%{verb} %{path}\"" } },
    { "date": { "field": "ts", "formats": ["dd/MMM/yyyy:HH:mm:ss Z"], "target_field": "@timestamp" } },
    { "set": { "field": "pipeline", "value": "weblog" } },
    { "rename": { "field": "verb", "target_field": "http.method" } },
    { "drop": { "if": "ctx.path == '/healthz'" } }
  ]
}
```

- **LCM-040** A pipeline is an ordered list of processors, executed in order against the document. Each processor MAY carry `if` (condition — the existing `script/` engine's condition grammar), `ignore_failure: bool` (default `false`), and `on_failure: [processor]`.

### 4.2 Initial processor set (normative)

| Processor | Parameters | Semantics |
|---|---|---|
| `set` | `field`, `value`, `override` (default `true`) | Set a field to a literal value. |
| `rename` | `field`, `target_field` | Move a field; fails if `field` missing or `target_field` exists. |
| `date` | `field`, `formats` (ordered list), `target_field` (default `@timestamp`), `timezone` | Try formats in order; first parse wins; fails if none parse. |
| `dissect` | `field`, `pattern`, `append_separator` | Delimiter-based extraction (`%{key}` tokens); fails on pattern mismatch. |
| `grok` | `field`, `patterns` | OPTIONAL — only if a vetted crate covers it; `dissect` is the required 80% case. |
| `drop` | `if` | Silently discard the document (not an error); `result: "noop"` reported per item. |

- **LCM-041** Processor failure handling, in precedence order: (1) if the processor has `on_failure`, run those processors and continue the pipeline; (2) else if `ignore_failure: true`, skip and continue; (3) else the **document** fails with a per-item SPEC-003-shaped error naming the pipeline, processor index/type, and reason. A document failure MUST NOT fail the batch (SPEC-005 `_bulk` per-item semantics) and MUST NOT partially index the document.
- **LCM-042** Execution points: `?pipeline=<id>` on document create and `_bulk`, else the index's `defaultPipeline` setting (SPEC-006 §2.1), else no pipeline. Unknown pipeline id → 400 at request time, not per item.
- **LCM-043** `POST /_ingest/pipeline/{id}/_simulate` with `{"docs": [{"_source": {...}}]}` MUST return the transformed documents (and per-doc errors) without indexing anything; simulate output MUST equal real ingestion output for the same input.

## 5. Logical dumps (R-09) — distinct from binary snapshots

Binary snapshots (`/_snapshot/*`) copy Tantivy segments and are only valid within one segment-format version — and the current repository writes mock data (`snapshot/repository.rs:854,903`; filed with phase14, out of scope here). Dumps are the trustworthy, portable path.

### 5.1 Format

- **LCM-050** A dump is a single `tar.gz` archive:

```
dump-<timestamp>/
  manifest.json          # { "dumpVersion": 1, "lexumVersion": "x.y.z", "createdAt": RFC3339 }
  indices/<name>/
    documents.ndjson     # one JSON document per line, full _source (+ _id)
    settings.json        # SPEC-006 §2 settings object (full, defaults filled)
    mapping.json         # ES mapping
  templates.ndjson       # index templates
  ilm-policies.ndjson    # §2 policies (+ attachments)
  ingest-pipelines.ndjson
  keys.ndjson            # API keys, hashed form (empty with a manifest warning until the key store lands)
  tasks.ndjson           # task history (SPEC-002 shape)
```

- **LCM-051** **Version portability guarantee**: a dump produced by any Lexum version MUST import into any later Lexum version, across Lexum *and* Tantivy version bumps — because import re-indexes documents through the normal write path rather than copying segments. Binary snapshots carry no such guarantee. `dumpVersion` is bumped only for archive-layout changes; importers MUST reject a `dumpVersion` newer than they understand with a clear SPEC-003 error, and MUST accept all older versions they document.
- **LCM-052** Every payload in the archive is JSON/NDJSON — no binary segment data, no bincode. Documents are the indexed `_source`, not Tantivy internal representations.

### 5.2 Dump and import as tasks

- **LCM-053** `POST /dumps` enqueues a SPEC-002 task (`202` + `taskUid`); the archive lands in a configurable `path.dumps` directory; task `details` report the file path and per-section counts. `GET /dumps/{taskUid}` reads status via the task API.
- **LCM-054** `POST /dumps/import` (and the `--import-dump <path>` startup flag) rebuilds state by replaying the archive through the normal write path: create indices with dumped settings + mappings, re-index documents, restore templates/policies/pipelines/keys. Import runs as a task; a corrupt archive or manifest mismatch fails the task with a SPEC-003 error before any partial index is exposed.
- **LCM-055** Round-trip invariant: dump → import into a fresh data directory → identical search results and byte-equivalent settings objects (SPEC-016 conformance test).

## 6. Task webhooks (R-11)

- **LCM-060** Webhook registration is a persisted config resource: `{ "url": "...", "headers": {"Authorization": "..."}, "events": ["dumpCreation", "indexRollover", ...] }` — multiple targets; empty/absent `events` means all task kinds.
- **LCM-061** On SPEC-002 task completion (terminal states `succeeded`, `failed`, `canceled`), Lexum POSTs to each matching target. Payload: the task object exactly as `GET /tasks/{uid}` returns it (SPEC-002 shape); completions occurring within a debounce window MAY be batched as NDJSON (one task object per line), `Content-Type: application/x-ndjson`.
- **LCM-062** Each request carries an HMAC-SHA256 signature header (`X-Lexum-Signature: sha256=<hex>`) computed over the raw body with the per-webhook secret, so receivers can authenticate the sender.
- **LCM-063** Delivery semantics are **at-least-once**: a 2xx response acknowledges; any other outcome (non-2xx, timeout, connection failure) is retried with exponential backoff (base 1 s, factor 2, max interval 5 min, max attempts ≥ 8, then dropped with a WARN log). Receivers MUST deduplicate by task `uid` + status.
- **LCM-064** **No ordering guarantee**: deliveries may arrive out of order across tasks and across retries. Delivery MUST be fully asynchronous — an unreachable or slow target never blocks, delays, or fails task processing or the evaluator loop (bounded outbound queue; overflow drops oldest with a WARN, mirroring A-06 cap-and-error over silent degradation).

## 7. Acceptance criteria

1. **Rollover reality** (LCM-010/011/020): each condition fires on real data — `max_docs` by indexing N docs, `max_size` by real bytes on disk, `max_age` via injectable clock; conditions survive restart; the background evaluator rolls over with no API call; no fabricated-stat code paths remain.
2. **ILM end-to-end** (LCM-021/023): template + policy → write through alias → auto-rollover at `max_docs` → old index deleted after `min_age`; `_ilm/explain` reports the correct phase at every step.
3. **Write-alias atomicity** (LCM-030/033): writes to the alias land only in the write index; under concurrent writes during rollover every document lands in exactly one generation; alias with no write index rejects writes with the named error.
4. **Ingest** (LCM-040–043): a pipeline of `set`+`rename`+`date`+`dissect`+conditional `drop` transforms documents on create and `_bulk`; `_simulate` output equals real ingestion; a failing processor yields a per-item error (with `on_failure` and `ignore_failure` variants tested), never a batch failure; `defaultPipeline` applies when no param is given.
5. **Dump round-trip** (LCM-051/055): populate 2 indices + settings + template + policy + pipeline → dump → import into a fresh data dir → identical search results and settings; newer `dumpVersion` and corrupted archive produce clear task errors with no partial state.
6. **Webhooks** (LCM-061–064): a local receiver gets a signed POST for a completed dump task within the debounce window; signature verifies; 5xx target is retried with backoff; an unreachable target never stalls the task queue.
