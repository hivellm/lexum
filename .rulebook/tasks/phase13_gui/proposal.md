# Proposal: phase13_gui

> **Re-validate scope when picked up — this phase was planned ahead of implementation.**
> The screens below track API surfaces built in phases 1–9 and the TS SDK from
> phase12; re-check what actually shipped (and the current sibling GUI stack)
> before starting.

## Why

Lexum has no visual management surface — every operation goes through curl,
the CLI, or Swagger UI. Operators need to see the task queue draining, cluster
health, and index state at a glance, and developers need a search playground
to iterate on LQL queries and settings without hand-writing requests. The
HiveLLM family already has a GUI convention: Vectorizer and Synap both ship a
`gui/` folder built on **Electron + Vue 3 + Vite + TypeScript + Tailwind +
Pinia** (with Monaco for query editing and the project's own TypeScript SDK as
the data layer). The archived `2026-07-17-add-electron-gui` task specified
Electron + **React** + MUI and a Kibana-scale scope (dashboard builder, log
viewer, auto-update); this re-scope aligns the stack with what the family
actually maintains (Vue, not React) and cuts the scope to the management/
observability core.

## What Changes

1. **Create `gui/`** following the sibling layout (Vectorizer/Synap):
   Vite + Vue 3 + TypeScript + Tailwind + Pinia + vue-router, `electron/`
   main process, `electron-builder` packaging for Windows/macOS/Linux.
   Data layer is the phase12 TypeScript SDK (`@hivehub/lexum-sdk`), the same
   pattern as Vectorizer's GUI consuming `@hivehub/vectorizer-sdk`.
2. **Connection management**: multiple saved server profiles (URL + API key),
   stored via `electron-store`; all screens scoped to the active connection.
3. **Core screens (the scope of this phase):**
   - **Index management** — list indexes with doc counts/size, create/delete,
     view mappings, browse documents, trigger snapshot/dump actions.
   - **Search playground** — dual mode: *simple* (q + filter + sort + facets
     as form controls) and *LQL* (Monaco editor with syntax highlighting);
     rendered hits with highlighting/cropping, raw JSON toggle, ranking-score
     display, and a multi-search tab when phase8 has shipped.
   - **Task queue monitor** — live task list with status/type/index filters,
     task detail (durations, error object), cancel/delete, auto-refresh;
     surfacing the phase1 `enqueued/processing/succeeded/failed/canceled`
     lifecycle.
   - **Cluster health** — node list, shard/replica state (phase9 surface when
     available; single-node health/metrics from phase6 otherwise), key
     Prometheus-derived charts (chart.js, per family convention).
   - **Settings editor** — the phase4 per-index settings resource as a form
     (searchable/filterable/sortable attributes, ranking rules, typo
     tolerance, synonyms, stop words) with diff-against-defaults, reset per
     key, and raw JSON edit mode.
4. **Explicitly out of scope (cut from the legacy plan):** dashboard builder /
   visualization designer, log viewer, security/user management UI,
   WebSocket real-time push (poll on interval instead), auto-update
   mechanism. Revisit only on demand.

## Impact

- Affected specs: `.rulebook/tasks/phase13_gui/specs/` (screen inventory,
  connection model, packaging targets)
- Affected code: new `gui/` tree (Electron main in `gui/electron/`, Vue app in
  `gui/src/`); optional `.github/workflows/` job for GUI build; no server
  changes expected (gaps found while building screens are filed against the
  owning phase, not patched here)
- Breaking change: NO (additive desktop application)
- User benefit: at-a-glance operations (task queue, cluster health) and a
  fast feedback loop for query/settings tuning, matching the tooling level of
  the sibling projects

## Dependencies

- **phase1_write-path-task-queue** (hard): the task queue monitor is a core
  screen; the write lifecycle must exist.
- **phase6_ops-observability-surface** (hard): health/stats/metrics endpoints
  feed the cluster health screen.
- **phase12_client-sdks** (hard): the TypeScript SDK is the GUI's data layer —
  build at least §2 of phase12 first.
- **phase4_settings-mappings-analyze** (hard): the settings editor edits the
  settings resource.
- **phase8_multisearch-federation**, **phase9_distributed-clustering** (soft):
  multi-search tab and shard/replica views degrade gracefully to hidden/
  single-node when absent.

## Success criteria

- `gui/` builds and packages on Windows, macOS, and Linux via
  electron-builder, with dev mode (`vite` + electron) working per sibling
  convention.
- All five core screens function against a real `lexum-server`: create an
  index, edit its settings, index documents, watch the tasks drain in the
  monitor, and search them in both simple and LQL modes — end to end without
  touching curl.
- Task queue monitor reflects a task's status transition within one refresh
  interval (≤2 s default) and renders the server error object on failures.
- GUI consumes only the phase12 TS SDK for server communication (no ad-hoc
  fetch calls to Lexum endpoints).
- Type-check (`vue-tsc`) and lint pass in CI; component tests cover the
  stores and the task/status rendering logic.
