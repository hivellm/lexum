# Lexum Roadmap

**Last Updated**: 2026-07-17
**Planning basis**: [Elasticsearch analysis](analysis/elastic/README.md) + [Meilisearch analysis](analysis/meilisearch/README.md)
**Task tracking**: [.rulebook/tasks/](../.rulebook/tasks/README.md) — one Rulebook task per phase, with proposals, checklists, and measurable gates
**Implementation contract**: [docs/specs/](specs/README.md) — SPEC-001..SPEC-016, normative requirements per phase

## Strategy

Lexum's parity target (from the analyses):

> **Drop-in for the 20% of the ES 7.10 API that 95% of clients use, plus 2026-grade vector/hybrid search, minus ES's legacy** — not endpoint-count parity.

Combined with Meilisearch's developer-experience lessons: async task queue for all writes, uniform errors, settings-as-resource, relevance that works by default, and a simple search path that never requires a query language.

Three tracks run in this order of precedence:

1. **Compatibility kernel** (phases 1–6) — achievable on the current single-node engine; makes Lexum usable by the existing ES ecosystem before any distribution work ships.
2. **Production platform** (phases 7–11) — security, federation, distribution, vectors, lifecycle.
3. **Ecosystem** (phases 12–14) — SDKs, GUI, production operations.

Anti-goals (deliberately not planned — see analyses): mapping types, legacy scroll API, `query_string` exposure, engine-side scripting, parent-child joins, ES-style API sprawl, LMDB-style single-writer storage, paid-edition-gated distribution, default-on telemetry.

## Version strategy

- **v0.2.x** — Compatibility kernel (phases 1–6)
- **v0.3.x** — Security + federation (phases 7–8)
- **v0.4.x** — Distribution (phase 9)
- **v0.5.x** — Vectors + lifecycle (phases 10–11)
- **v1.0.0** — Production-ready single+multi-node with SDKs (phases 12–14 mature)

## Phases

Each phase is a Rulebook task (`.rulebook/tasks/phaseN_slug/`) with the full proposal, checklist, and gates. Summary and dependency graph:

### Track 1 — Compatibility kernel (P0)

| Phase | Task | Scope | Depends on |
|---|---|---|---|
| 1 | `phase1_write-path-task-queue` | Async task queue for all writes (taskUid, statuses, auto-batching of Tantivy commits, bounded queue), uniform error object `{message, code, type, link}` | — |
| 2 | `phase2_search-kernel-parity` | Core Query DSL (bool, match family, term-level), search_after + PIT, response shaping (highlight/crop/retrieve), typo-tolerance defaults, matchingStrategy; simple `q`+filter+sort path stays first-class | — |
| 3 | `phase3_bulk-and-document-crud` | `_bulk` exact NDJSON semantics with per-item errors, `_mget`, refresh semantics, optimistic concurrency, update/delete-by-query | 1 |
| 4 | `phase4_settings-mappings-analyze` | Settings as REST resource (GET/PATCH/reset, documented defaults), ES mappings compat (dynamic mapping capped, text/keyword multi-field), `_analyze`; templates stamp settings | 1 |
| 5 | `phase5_aggregations-facets` | Facet distribution + stats first (Meilisearch model), ES-style buckets/metrics via Tantivy's aggregation module, facet search | 2 |
| 6 | `phase6_ops-observability-surface` | `_cluster/health`, `_cat/*`, `_stats`, `_nodes`, experimental-features gate, opt-in telemetry | — |

**Kernel exit gate**: an unmodified ES client/shipper (e.g. a log forwarder speaking `_bulk` + a Kibana-style dashboard reading `_cat`/`_cluster/health`) works against Lexum.

### Track 2 — Production platform (P1)

| Phase | Task | Scope | Depends on |
|---|---|---|---|
| 7 | `phase7_security-rbac-tenant-tokens` | RBAC with index-pattern privileges over existing API keys; tenant tokens (JWT search rules, stateless) | — |
| 8 | `phase8_multisearch-federation` | Normalized 0–1 ranking scores + score details, `/multi-search`, federated merging with per-query errors and partial results, remote targets | 2 |
| 9 | `phase9_distributed-clustering` | WAL/durability (Tantivy has no translog — ours to build), seq-no replication, in-sync sets, allocation/rebalance; Jepsen-style fault injection as first-class deliverable; federation is the scatter-gather path; task log is the replication log. Always open source | 1, 8 |
| 10 | `phase10_vector-hybrid-search` | dense_vector + kNN + hybrid fusion over normalized scores; ANN layer evaluation (VecLite/hannoy/usearch) bound to segment lifecycle; embedders as settings; int8 quantization from day one; behind experimental gate | 4, 6, 8 |
| 11 | `phase11_lifecycle-ingest-dumps` | ILM-lite (rollover + delete + data-stream aliases), ingest-pipeline-lite, logical dumps (version-portable), task webhooks | 1, 4 |

### Track 3 — Ecosystem (P2)

| Phase | Task | Scope | Depends on |
|---|---|---|---|
| 12 | `phase12_client-sdks` | TS/JS + Python first, then Rust; family `sdks/` convention; task-queue-aware clients | 1–6 |
| 13 | `phase13_gui` | Family `gui/` convention; index management, search playground, task monitor, cluster health | 1, 6 |
| 14 | `phase14_production-deployment` | Harden deploy/helm + deploy/k8s, Grafana/Prometheus templates, runbooks, backup automation | 6, 9 |

Later-phase tasks (12–14) carry a "re-validate scope at pickup" note — they were planned ahead of implementation.

### Deferred (P2+, no task yet — create when reached)

- `semantic_text`-shaped fields with external inference endpoints
- ES|QL-inspired piped extensions to LQL (`STATS ... BY`)
- SLM (scheduled snapshots), cold tier, downsampling — only after distribution works
- Percolate-style reverse search (alerting)
- Geosearch sugar (`_geoRadius`, `_geoBoundingBox` over fast fields + Haversine)
- Chat/conversational search via MCP with built-in search tool definition

## History

The pre-2026 roadmap (calendar-quarter phases, week estimates, "3–5 developers") and the monolithic `add-elasticsearch-parity` task (610 items, "95%+ parity" goal) were retired on 2026-07-17 and archived under [.rulebook/archive/](../.rulebook/archive/). Completed foundation work (core engine, REST API, LQL, CLI, snapshots, templates, aggregations base, security base) remains documented in the archived tasks and [STATUS.md](STATUS.md).
