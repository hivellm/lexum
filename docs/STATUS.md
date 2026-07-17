# Lexum - Project Status

**Last Updated**: 2026-07-17
**Phase**: Re-planned — foundation complete, compatibility kernel next
**Task tracking**: [.rulebook/tasks/](../.rulebook/tasks/README.md)

## Where the project stands

The 2025 foundation work is complete and archived (18 completed tasks in [.rulebook/archive/](../.rulebook/archive/)): core Tantivy engine, REST API (39 endpoints, all route tests passing), LQL query language, CLI, snapshots/restore, index templates, aggregation framework base, API-key auth + rate limiting, protocol support (StreamableHTTP/MCP/UMICP base), Docker/K8s manifests.

On 2026-07-17 the project was re-planned around two analyses ([Elasticsearch](analysis/elastic/README.md), [Meilisearch](analysis/meilisearch/README.md)) and re-organized to the HiveLLM family standard:

- Workspace moved to `crates/` layout (`lexum-core`, `lexum-server`, `lexum-macros`)
- `docs/` reorganized into thematic subdirectories
- Legacy planning (calendar-based roadmap, monolithic 610-item parity task) archived
- 14 phased Rulebook tasks created (`phase1`…`phase14`), each with proposal, checklist, and measurable gates — see [ROADMAP.md](ROADMAP.md) for the track structure

## Quality state (verified 2026-07-17)

- `cargo check --workspace --all-targets` — clean
- `cargo clippy --workspace --all-targets -- -D warnings` — clean (enforced by pre-commit hook)
- `cargo fmt` — enforced by pre-commit hook
- Test suite: unit + integration + e2e + stress crates under `tests/` (coverage threshold configured at 95% in rulebook.json)

## Next up

Track 1 (compatibility kernel), starting with `phase1_write-path-task-queue` — the architectural keystone every later phase builds on. See [ROADMAP.md](ROADMAP.md).

## Known constraints

- **WSL/Tantivy incompatibility**: all builds and tests must run on native Windows (PowerShell) — see [development/WSL_TANTIVY_CONFLICT.md](development/WSL_TANTIVY_CONFLICT.md)
- **Tantivy 0.25**: no translog (durability layer is ours to build — phase9), no built-in HNSW (vector layer is an integration project — phase10), thinner analyzer ecosystem than Lucene (language packs planned incrementally)
