# Lexum — Specifications

This directory is the **implementation contract** for Lexum, derived from the [Elasticsearch](../analysis/elastic/README.md) and [Meilisearch](../analysis/meilisearch/README.md) analyses. The analyses explain *why*; these documents say *what to build*, normatively.

## How to navigate

| Question | Read |
|---|---|
| What are we building, in what order, and why? | [ROADMAP.md](../ROADMAP.md) |
| Where does the project stand right now? | [STATUS.md](../STATUS.md) |
| What is the active work breakdown? | [.rulebook/tasks/](../../.rulebook/tasks/README.md) |
| How exactly does component X behave? | The SPEC for that component (below) |
| Why was it designed this way? | [docs/analysis/](../analysis/) (findings F-NNN, recommendations R-NN, anti-patterns A-NN) |

Traceability chain: **analysis** findings (`F-xxx`/`R-xx`/`A-xx`) → **roadmap** phases → **Rulebook tasks** (`phaseN_slug`) → **SPEC** requirement IDs (`TSK-xxx`, `SRCH-xxx`, …) → tests ([SPEC-016](SPEC-016-testing-and-conformance.md)).

## Specifications

| Spec | Prefix | Scope | Phase |
|---|---|---|---|
| [SPEC-001](SPEC-001-architecture.md) — Architecture | `ARC` | Crate model, layering, write door / read path, config, target architecture | all |
| [SPEC-002](SPEC-002-write-path-task-queue.md) — Write Path & Task Queue | `TSK` | Task object/state machine, 202+taskUid, per-index scheduler, auto-batching, bounded queue, recovery | 1 |
| [SPEC-003](SPEC-003-error-contract.md) — Error Contract | `ERR` | Uniform `{message, code, type, link}`, code registry, HTTP mapping, migration | 1 |
| [SPEC-004](SPEC-004-search-api.md) — Search API | `SRCH` | Simple `q`+filter+sort door, ES DSL subset, pagination/PIT, response shaping, typo tolerance, matchingStrategy | 2 |
| [SPEC-005](SPEC-005-documents-and-bulk.md) — Documents & Bulk | `DOC` | `_bulk` NDJSON semantics, CRUD, `_mget`, refresh, optimistic concurrency, by-query ops | 3 |
| [SPEC-006](SPEC-006-settings-and-mappings.md) — Settings & Mappings | `SET` | Settings resource + defaults, persistence, ES mappings compat, `_analyze`, templates | 4 |
| [SPEC-007](SPEC-007-aggregations-and-facets.md) — Aggregations & Facets | `AGG` | facetDistribution/facetStats, facet search, ES aggs grammar, limits | 5 |
| [SPEC-008](SPEC-008-ops-surface.md) — Ops Surface | `OPS` | `_cluster/health`, `_cat/*`, `_stats`, `_nodes`, Prometheus, experimental gate, opt-in telemetry | 6 |
| [SPEC-009](SPEC-009-security.md) — Security | `SEC` | Hashed API keys, RBAC taxonomy × index patterns, tenant tokens (JWT) | 7 |
| [SPEC-010](SPEC-010-federation-multisearch.md) — Federation & Multi-Search | `FED` | Normalized 0–1 scores, `/multi-search`, federated merge, partial results, remotes | 8 |
| [SPEC-011](SPEC-011-distribution.md) — Distribution | `DST` | Per-shard WAL, seq_no/primary_term, in-sync sets, allocation, recovery, fault-injection invariants | 9 |
| [SPEC-012](SPEC-012-vector-hybrid.md) — Vector & Hybrid Search | `VEC` | `dense_vector`, `VectorIndex` trait + segment sidecar, kNN, hybrid fusion, embedders, quantization | 10 |
| [SPEC-013](SPEC-013-lifecycle-ingest-dumps.md) — Lifecycle, Ingest & Dumps | `LCM` | ILM-lite, ingest-pipeline-lite, logical dumps, webhooks | 11 |
| [SPEC-014](SPEC-014-lql.md) — LQL | `LQL` | Grammar, pipe operations, lowering onto SPEC-004/007, params, error positions | 2 |
| SPEC-015 — Protocols (MCP/UMICP) | `PRO` | **Reserved** — to be written with the protocols work | TBD |
| [SPEC-016](SPEC-016-testing-and-conformance.md) — Testing & Conformance | `TST` | Test tiers, ES-fixture harness, error-contract walk, CI gates, fault injection | all |

## Conventions

- RFC 2119 keywords (**MUST**, **MUST NOT**, **SHOULD**, **MAY**) are normative.
- Requirement IDs are stable and referenced from tasks, commits, PRs, and tests. Removing or changing the meaning of an ID requires the same review bar as the behavior change itself.
- All specs are **Draft** until their phase's implementation freezes them; SPEC-011 is explicitly pre-ADR — phase9's ADRs may amend it.
- Status of each spec lives in its metadata table, never in this index.
