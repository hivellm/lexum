# Proposal: phase11_lifecycle-ingest-dumps

## Why

Four planned capabilities converge here, all P1-grade in the analyses:

- **ILM-lite** (elastic plan P1 item 9, F-021): "rollover by size/age +
  delete phase + data-stream-style aliases — the 80% of ILM that log
  retention needs. Lexum already has templates and snapshots, the two
  prerequisites." Anti-goal F-050 #8 makes rollover *the* growth story
  (1 primary shard + rollover, as ES concluded in 7.0).
- **Ingest-pipeline-lite** (elastic plan P1 item 10, F-024):
  `set`/`rename`/`date`/`grok|dissect`/`drop` processors — what log
  shippers need before Lexum can sit behind them.
- **Dumps** (meilisearch plan **R-09**, F-021): a version-portable logical
  export (documents + settings + keys + tasks as JSON/NDJSON), distinct
  from binary snapshots — "Tantivy segment formats change between versions,
  so Lexum will need this the first time it bumps Tantivy."
- **Task webhooks** (meilisearch plan **R-11**, F-020): webhook-on-task-
  completion, the minimal event-driven integration Lexum's MCP/UMICP plans
  imply.

A code audit (2026-07) shows the prerequisites are closer to scaffolding
than the parity matrices assumed, so this task is equal parts gap-closing
and honesty repair:

- **Rollover is manual-and-mocked.** Handlers exist (`handlers/rollover.rs`,
  ES-style POST/GET/PUT `_rollover` with dry_run), but the stats feeding
  condition checks are fabricated (`size_in_bytes = num_docs * 1024`,
  `age_in_millis = 0`) — so `max_age`/`max_size` can never fire; only
  `max_docs` works. Conditions are not persisted (`get/update_rollover_
  conditions` are no-ops), and the background evaluator
  (`services/rollover_service.rs`, complete with a 60s interval loop) is
  **dead code — never started by anyone**.
- **Write-alias semantics are stored but not enforced**: `AliasConfig.
  is_write_index` exists and `AliasManager` supports atomic operations with
  rollback, but nothing resolves an alias to its single write index for
  writes. `index/datastream.rs` and `timeseries.rs` model exactly the needed
  metadata (write index tracking, `should_rollover`, retention cleanup) but
  are library-only — zero references from lexum-server.
- **No ILM policy concept, no delete phase** anywhere (the only retention
  code, `TimeSeriesMetadata::cleanup_old_partitions()`, is unused).
- **No ingest pipelines** — but `crates/lexum-core/src/script/` is a real
  document-transformation engine (SetField/RemoveField/AddField/If/ForEach/
  Math/StringOp with regex conditions), currently reachable only via
  `_reindex` (`handlers/reindex.rs`). It is the natural substrate for
  set/rename/drop processors.
- **No logical dump/export exists** (no route, no NDJSON, no CLI command) —
  and critically, the binary snapshot repository **writes mock data**:
  `create_index_snapshot_data()` (snapshot/repository.rs:854) emits a fixed
  `total_documents: 1000` payload and a hardcoded schema regardless of the
  actual index. Until that is fixed, Lexum has *no* trustworthy
  backup/migration path — a logical dump is the fastest honest one.
- **No webhooks; no general task queue.** Only a reindex-scoped in-memory
  `TaskManager` exists. Phase1 (write-path task queue, R-01) is scaffolded
  but not started — this task's dump-as-task and webhook items *require*
  phase1's `taskUid` machinery to land first.

## What Changes

1. **Make rollover real.** Feed condition checks with real numbers: index
   age from a persisted creation timestamp, size from the real
   `IndexStats.size_in_bytes` (phase6 §1 plumbing), docs from Tantivy.
   Persist rollover conditions per alias/index (today's GET/PUT are no-ops).
   Wire the existing-but-never-started `RolloverService` into server startup
   as the background evaluator (configurable interval, default 60s), and
   collapse the duplicate rollover implementations (`handlers/rollover.rs`
   vs `index/rollover.rs` + the legacy `index::rollover_index` route) into
   one code path.
2. **ILM-lite policies with a delete phase.** A named lifecycle policy
   resource (`PUT/GET/DELETE /_ilm/policy/{name}`, deliberately tiny:
   `rollover: {max_size|max_age|max_docs}` + `delete: {min_age}`), attached
   to indices via index templates (phase4 settings-stamping chassis) or
   directly. The background evaluator executes rollover and, after
   `min_age` past rollover, deletes rolled-over indices. Explain endpoint
   (`GET /{index}/_ilm/explain`) reports each index's phase and next action.
3. **Data-stream-style write aliases.** Enforce `is_write_index`: writes
   addressed to an alias route to its single write index (error if none/
   ambiguous); rollover atomically flips the write flag via the existing
   `execute_atomic_operations`; wire the dormant `datastream.rs` metadata
   (write-index tracking, next-name generation) into the manager instead of
   leaving it library-only. Reads via alias continue to fan out over all
   generations.
4. **Ingest-pipeline-lite.** Pipeline resource (`PUT/GET/DELETE
   /_ingest/pipeline/{id}`, persisted) composed of processors: `set`,
   `rename`, `date` (parse formats → timestamp field), `dissect` (fast
   delimiter-based extraction; `grok` only if a vetted crate covers it —
   dissect is the 80% case), `drop` (conditional document drop), each with
   optional `if` conditions. Execute on the write path (document create +
   `_bulk`) via `?pipeline=` and a `default_pipeline` index setting
   (phase4 settings object), reusing the existing `script/` engine's ops
   where they fit. `POST /_ingest/pipeline/{id}/_simulate` for dry runs.
   Per-document processor failures follow `_bulk` per-item error semantics
   (phase3).
5. **Logical dumps (R-09).** `POST /dumps` enqueues a dump task (phase1
   queue) producing a single portable archive: per-index NDJSON documents +
   the phase4 settings object + mappings + templates + lifecycle/pipeline
   definitions + API keys (hashed form, from phase7's key store) + task
   history, with a version manifest (`lexumVersion`, `dumpVersion`).
   `POST /dumps/import` (or `--import-dump` at startup) rebuilds indices by
   re-indexing through the normal write path — making dumps portable across
   Tantivy segment-format changes, which binary snapshots by design are
   not. Explicitly distinct from `/_snapshot/*`; the audit finding that the
   snapshot repository currently writes hardcoded mock data
   (repository.rs:854,903) is filed with phase14 — dumps do not inherit
   that code.
6. **Task webhooks (R-11).** `webhooks` config resource (URL + auth header,
   multiple targets): on task completion (batch-debounced), POST the task
   payload (phase1 task shape) with retry/backoff and a signing header.
   Applies to all task kinds — dumps, rollover, ingest reindex, document
   batches.

Cross-phase dependencies (hard): **phase1** (task queue/taskUid — dumps run
as tasks, webhooks fire on task completion; the audit confirms only a
reindex-scoped in-memory TaskManager exists today, so phase1 must land
first), **phase4** (settings object: `default_pipeline` knob, settings
included in dumps, template stamping for policy attachment), **phase6**
(real `IndexStats.size_in_bytes` for size-based rollover). Soft: **phase3**
(`_bulk` per-item error semantics for pipeline failures), **phase7** (dump
of API keys requires the persistent key store; until then dumps export the
key section as empty-with-warning). Feeds **phase9** (rollover-based growth
is the distribution story) and **phase14** (production deployment relies on
dump/restore as the upgrade path).

## Impact

- Affected specs: `.rulebook/tasks/phase11_lifecycle-ingest-dumps/specs/`
  (ilm-lite spec: policy shape, evaluator semantics, write-alias contract;
  ingest spec: processor set, failure semantics; dumps spec: archive format
  + version manifest; webhooks spec: payload, retry, signing)
- Affected code: `crates/lexum-core/src/index/{rollover.rs, alias.rs,
  datastream.rs, timeseries.rs, manager.rs}`,
  `crates/lexum-server/src/handlers/{rollover.rs, alias.rs, document.rs,
  batch.rs, index.rs}`, `crates/lexum-server/src/services/
  rollover_service.rs` (wire into startup), new `crates/lexum-core/src/
  ingest/` (processors) + `handlers/{ilm.rs, ingest.rs, dump.rs,
  webhook.rs}`, `crates/lexum-core/src/script/` (reused by processors),
  `crates/lexum-server/src/router.rs`, `crates/lexum-server/src/openapi.rs`,
  `crates/lexum-server/src/main.rs` (background services)
- Breaking change: NO (new resources; rollover conditions that previously
  could never fire now fire — documented as a behavior fix; writes to
  aliases without a write index now error instead of being ambiguous,
  called out in CHANGELOG.md)
- User benefit: log-retention workloads run unattended (rollover + delete
  without cron scripts); shippers can point at Lexum without a transform
  layer; upgrades across Tantivy format changes become possible via
  version-portable dumps; downstream systems get push notifications
  instead of polling `/tasks`.

## Success criteria

- Rollover fires on all three real conditions in integration tests:
  `max_docs` (index N docs), `max_size` (real bytes on disk from phase6
  stats — no `num_docs * 1024` estimates remain), `max_age` (persisted
  creation timestamp + injectable clock); conditions survive restart; the
  background evaluator rolls over without any API call.
- ILM policy end-to-end: policy with `rollover.max_docs` + `delete.min_age`
  attached via template → write through the alias → auto-rollover → aged
  index deleted; `_ilm/explain` reports the correct phase at each step.
- Writes to a data-stream-style alias land only in the write index;
  rollover atomically moves the write flag (concurrent writes during
  rollover land in exactly one generation, no loss).
- Pipeline with `set` + `rename` + `date` + `dissect` + conditional `drop`
  transforms documents on create and `_bulk`; `_simulate` previews without
  indexing; a failing processor yields a per-item error, not a batch
  failure.
- Dump/restore roundtrip: dump an instance (documents + settings + mappings
  + templates + policies + pipelines + tasks), import into a fresh data
  directory, and get identical search results and settings objects;
  manifest version mismatch produces a clear error.
- Webhook receives a signed POST for a completed dump task within the
  debounce window; unreachable target retries with backoff and never blocks
  task processing.
- `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt --check`, and
  the workspace test suite pass.
