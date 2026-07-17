## 1. Re-validate scope and scaffold
- [ ] 1.1 Re-validate this phase against what shipped: confirm the phase6 ops endpoints, phase1 task API, phase4 settings keys, phase8/phase9 availability, and re-check the current sibling GUI stack (`Vectorizer/gui`, `Synap/gui` package.json) — update proposal/specs where reality diverged
- [ ] 1.2 Scaffold `gui/` per family convention: Vite + Vue 3 + TypeScript + Tailwind + Pinia + vue-router, `electron/` main process, electron-builder config for win/mac/linux, dev-runner scripts
- [ ] 1.3 Wire the data layer: `@hivehub/lexum-sdk` (phase12) client factory, connection profiles (URL + API key) via electron-store, active-connection store, global error/toast handling

## 2. Index management screen
- [ ] 2.1 Index list (doc counts, size, health), create/delete with confirmation, mapping viewer
- [ ] 2.2 Document browser (paginated, raw JSON view) and snapshot/dump trigger actions with resulting-task links into the task monitor

## 3. Search playground
- [ ] 3.1 Simple mode: q + filter + sort + facet form controls, rendered hits with highlight/crop, both pagination styles, raw JSON toggle, ranking-score display
- [ ] 3.2 LQL mode: Monaco editor with LQL syntax highlighting, run/format, request/response panes, error-object rendering
- [ ] 3.3 Multi-search tab (behind capability detection — hidden if phase8 not shipped)

## 4. Task queue monitor
- [ ] 4.1 Live task list with status/type/index filters and auto-refresh interval; status badges for enqueued/processing/succeeded/failed/canceled
- [ ] 4.2 Task detail view (payload summary, durations, error object) with cancel/delete actions where the server supports them

## 5. Cluster health and settings editor
- [ ] 5.1 Cluster health screen: node/shard views when phase9 surface exists, single-node health/stats fallback from phase6; key metric charts (chart.js) with poll-based refresh
- [ ] 5.2 Settings editor: form-driven editing of the per-index settings resource with diff-against-defaults, per-key reset, and raw JSON mode; saves surface the resulting taskUid

## 6. Packaging and CI
- [ ] 6.1 electron-builder targets verified for Windows/macOS/Linux; app icons/metadata; README with dev + build instructions
- [ ] 6.2 CI job: install, type-check (vue-tsc), lint, component tests, and a headless build

## 7. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 7.1 Update or create documentation covering the implementation
- [ ] 7.2 Write tests covering the new behavior
- [ ] 7.3 Run tests and confirm they pass
