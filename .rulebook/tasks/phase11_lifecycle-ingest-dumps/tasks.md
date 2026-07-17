## 0. Preconditions (verify before starting)
- [ ] 0.1 Confirm phase1 (write-path task queue) has landed: task creation returns a `taskUid`, `GET /tasks` filters work — dumps (§4) and webhooks (§5) build on it; do not start those sections against the reindex-only in-memory TaskManager
- [ ] 0.2 Confirm phase6 §1 real index stats (`size_in_bytes`) and phase4 settings object (`default_pipeline` slot, template stamping) are available

## 1. Make rollover real
- [ ] 1.1 Replace fabricated stats in the rollover condition path: real size from `IndexStats.size_in_bytes`, real age from a persisted index-creation timestamp, real doc count — removing `size_in_bytes = num_docs * 1024` and `age_in_millis = 0` (crates/lexum-server/src/handlers/rollover.rs) and the `num_segments * 1_000_000` estimate (crates/lexum-core/src/index/rollover.rs)
- [ ] 1.2 Persist rollover conditions per alias/index and make `GET/PUT .../_rollover` read/write them (both are no-ops today); support `max_docs`, `max_size`, `max_age` with ES-style duration/size string parsing
- [ ] 1.3 Wire the existing `RolloverService` (crates/lexum-server/src/services/rollover_service.rs — complete but never started) into server startup with a configurable interval (default 60s) and graceful shutdown
- [ ] 1.4 Collapse the duplicate rollover implementations (handlers/rollover.rs vs index/rollover.rs vs the legacy `POST /{alias}/rollover` → `index::rollover_index` route) into one code path; keep `dry_run`
- [ ] 1.5 Tests: each condition fires on real data (docs indexed, bytes on disk, injectable clock for age); conditions survive restart; background evaluator rolls over with no API call

## 2. ILM-lite policies + delete phase + write aliases
- [ ] 2.1 Lifecycle policy resource: `PUT/GET/DELETE /_ilm/policy/{name}` with the minimal shape `{rollover: {max_size|max_age|max_docs}, delete: {min_age}}`, persisted; attachable via index template (phase4 stamping) or directly per index
- [ ] 2.2 Extend the background evaluator to execute the delete phase: an index rolled over more than `min_age` ago is deleted (its alias entry removed atomically); dry-run/log-only mode for safety
- [ ] 2.3 `GET /{index}/_ilm/explain` reporting current phase, attached policy, age, and next action
- [ ] 2.4 Enforce write-alias semantics: writes addressed to an alias resolve to the single `is_write_index` member (crates/lexum-core/src/index/alias.rs — flag exists but nothing resolves it today); no/ambiguous write index → uniform error; rollover flips the flag atomically via `execute_atomic_operations`
- [ ] 2.5 Wire the dormant `datastream.rs` metadata (write-index tracking, generation naming `<name>-000001`, `should_rollover`) into `IndexManager` instead of library-only code; reads via alias fan out over all generations
- [ ] 2.6 End-to-end test: template + policy → write through alias → auto-rollover at max_docs → delete after min_age; concurrent writes during rollover land in exactly one generation; `_ilm/explain` correct at each step

## 3. Ingest-pipeline-lite
- [ ] 3.1 New `crates/lexum-core/src/ingest/` module: pipeline definition (persisted) + processors `set`, `rename`, `date` (format list → parsed timestamp field), `dissect` (delimiter-pattern extraction), `drop` (conditional), each with optional `if` condition — reuse the `script/` engine's ops (SetField/RemoveField/conditions) where they fit
- [ ] 3.2 REST resource: `PUT/GET/DELETE /_ingest/pipeline/{id}` and `POST /_ingest/pipeline/{id}/_simulate` (transform sample docs without indexing)
- [ ] 3.3 Execute pipelines on the write path: `?pipeline=` param on document create and `_bulk`, plus a `default_pipeline` index setting (phase4 settings object); processor failure on a document produces a per-item error consistent with phase3 `_bulk` semantics, never a whole-batch failure
- [ ] 3.4 Tests: each processor unit-tested (incl. date format fallbacks and dissect edge patterns); pipeline ordering; conditional drop; `_simulate` parity with real ingestion; default_pipeline applied when no param given

## 4. Logical dumps (R-09) — distinct from binary snapshots
- [ ] 4.1 Dump writer: `POST /dumps` enqueues a phase1 task producing one portable archive (tar.gz) containing per-index NDJSON documents, the phase4 settings object + ES mappings, index templates, ILM policies, ingest pipelines, API keys (hashed, from phase7's store — empty-with-warning until phase7 lands), task history, and a version manifest (`lexumVersion`, `dumpVersion`, timestamp)
- [ ] 4.2 Dump import: `POST /dumps/import` (and/or `--import-dump` startup flag) rebuilding indices by re-indexing documents through the normal write path — portable across Tantivy segment-format changes by construction; reject on manifest `dumpVersion` mismatch with a clear error
- [ ] 4.3 `GET /dumps/{taskUid}` status via the phase1 task API; dump files land in a configurable `path.dumps` directory
- [ ] 4.4 Roundtrip test: populate an instance (2 indices, settings, template, policy, pipeline), dump, import into a fresh data directory, assert identical search results and settings objects; corrupted archive and version-mismatch error paths
- [ ] 4.5 Document the boundary in docs: dumps = version-portable logical export for upgrades; `/_snapshot/*` = binary backup (note: the snapshot repository's mock-data bug at crates/lexum-core/src/snapshot/repository.rs:854,903 is out of scope here — file/confirm a phase14 item so it is not lost)

## 5. Task webhooks (R-11)
- [ ] 5.1 Webhook configuration resource (persisted): target URL, optional auth header, optional event filter by task type — multiple targets supported
- [ ] 5.2 On task completion (phase1 queue), POST the task payload to each matching target, debounced per batch, with retry + exponential backoff and an HMAC signing header; delivery must never block or fail task processing
- [ ] 5.3 Tests with a local receiver: delivery on dump/rollover task completion, signature verification, retry on 5xx, unreachable target does not stall the queue

## 6. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 6.1 Update or create documentation covering the implementation
- [ ] 6.2 Write tests covering the new behavior
- [ ] 6.3 Run tests and confirm they pass
