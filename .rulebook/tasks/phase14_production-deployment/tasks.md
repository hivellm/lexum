## 1. Re-validate scope and audit existing artifacts
- [ ] 1.1 Re-validate this phase against what shipped: phase6 endpoint paths + Prometheus metric names, phase1 task metrics, phase11 dump/snapshot APIs, phase9 cluster surface; re-check sibling deploy trees (`Vectorizer/deploy`, `Nexus/deploy`) for convention drift — update proposal/specs accordingly
- [ ] 1.2 Audit `deploy/helm/lexum`, `deploy/k8s`, Dockerfile, and both docker-compose files against the current server (ports, env/config keys, probe paths, cluster mode); list and fix drift

## 2. Harden Helm chart and K8s manifests
- [ ] 2.1 Probes wired to phase6 health endpoints (readiness/liveness/startup), sane resource defaults, PodDisruptionBudget, securityContext (non-root, read-only rootfs where possible), persistence and cluster-mode values
- [ ] 2.2 CI validation: `helm lint`, `helm template`, kubeconform on rendered manifests and raw `deploy/k8s`; kind/k3d install job that deploys the chart and runs the smoke script (§6)

## 3. Monitoring assets (`deploy/monitoring/`)
- [ ] 3.1 Grafana dashboards (provisioning-ready JSON): search latency/throughput, task-queue depth + failure rate, indexing throughput, index/segment stats, resource usage, cluster/shard state (phase9 panels conditional)
- [ ] 3.2 Prometheus alert rules: instance down, task queue backlog, task failure spike, search p99 high, low disk, backup job failed — each annotated with its runbook link; `promtool check rules` in CI
- [ ] 3.3 Verify every dashboard panel and alert expression against metric names actually exported by a running `lexum-server`

## 4. Operational runbooks (`docs/runbooks/`)
- [ ] 4.1 Write runbooks (symptoms → diagnosis commands → remediation) for: node down/restart, task queue stuck, disk full, degraded search latency, restore-from-snapshot, restore-from-dump, scale up/down, shard/replica recovery (phase9, or marked deferred)
- [ ] 4.2 Cross-link alerts ↔ runbooks and index them from `docs/DEPLOYMENT.md`

## 5. Backup automation and upgrade procedures
- [ ] 5.1 Scheduled snapshot automation: K8s CronJob (helm/k8s) + cron/compose variant, retention pruning, optional object-storage upload hook
- [ ] 5.2 Periodic logical dumps (phase11) as the version-portable backup tier
- [ ] 5.3 Exercise restore end-to-end (snapshot AND dump → searchable index, verified with the smoke script); record the procedure in the runbooks
- [ ] 5.4 Document and test both upgrade paths: rolling Helm upgrade (same segment format) and dump-and-restore (Tantivy format bump), incl. pre-upgrade checklist, verification, rollback

## 6. Deployment verification scripts (`scripts/`)
- [ ] 6.1 Post-deploy smoke script runnable against any environment: health → create index → index doc → wait for task → search → metrics endpoint → cleanup; clear pass/fail output
- [ ] 6.2 Wire the smoke script into the kind/k3d CI job (§2.2) and reference it from the upgrade checklist and runbooks

## 7. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 7.1 Update or create documentation covering the implementation
- [ ] 7.2 Write tests covering the new behavior
- [ ] 7.3 Run tests and confirm they pass
