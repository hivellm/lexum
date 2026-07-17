# Proposal: phase14_production-deployment

> **Re-validate scope when picked up — this phase was planned ahead of implementation.**
> This phase hardens whatever ops surface (phase6), lifecycle/dumps (phase11),
> and clustering (phase9) actually shipped; re-check the deploy tree, metric
> names, and sibling deploy conventions before starting.

## Why

Lexum already ships the family-standard deployment artifacts — `Dockerfile`,
`docker-compose.yml` / `docker-compose.cluster.yml`, `deploy/helm/lexum`, and
`deploy/k8s` manifests (StatefulSet, HPA, ServiceMonitor, ingress, etc.) — but
they predate the re-planned architecture (task queue, ops surface, clustering)
and there is no operational layer on top: no Grafana dashboards, no Prometheus
alert rules, no runbooks, no automated backups, no documented upgrade path,
and no way to verify a deployment beyond "the pod is Running".

The archived `2026-07-17-add-production-deployment` task wanted Terraform
modules (AWS/GCP/Azure), Ansible playbooks, and a Rust Kubernetes operator.
No sibling project maintains any of those — Vectorizer and Nexus deploy trees
contain only docker/helm/k8s — so that scope is **dropped** to avoid owning
infrastructure code the family has no precedent for maintaining. This re-scope
targets production *operations maturity* on the assets Lexum already has.

## What Changes

1. **Harden `deploy/helm/lexum` and `deploy/k8s`** against the re-planned
   server: correct readiness/liveness/startup probes against the phase6
   health endpoints, resource requests/limits defaults, PodDisruptionBudget,
   securityContext (non-root, read-only rootfs where possible), configurable
   persistence, cluster-mode values (phase9 discovery/peers) — and validate
   with `helm lint` + `helm template` + kubeconform in CI.
2. **Monitoring assets in `deploy/monitoring/`:**
   - Grafana dashboards (JSON, provisioning-ready): search latency/throughput,
     task-queue depth and task failure rate, indexing throughput, index
     size/segment counts, resource usage, and cluster/shard state (phase9);
   - Prometheus alert rules (YAML): instance down, task queue backing up,
     task failure spike, high search p99, disk-space low, snapshot/backup job
     failed — each alert annotated with a link to its runbook section.
3. **Operational runbooks in `docs/runbooks/`:** covering at minimum: node
   down / restart, task queue stuck or backing up, disk full, degraded search
   latency, restore-from-snapshot, restore-from-dump, scaling up/down, and
   (phase9) shard/replica recovery. One page per scenario: symptoms →
   diagnosis commands → remediation.
4. **Backup automation:** scheduled snapshot creation (K8s CronJob template in
   helm/k8s + a plain-cron/compose variant), retention pruning, optional
   object-storage upload hook, and periodic logical **dumps** (phase11) for
   version-portable backups; restore procedures exercised, not just written.
5. **Upgrade/migration procedure docs:** documented, tested paths for
   (a) same-format rolling upgrade via Helm and (b) dump-and-restore upgrade
   for releases that bump Tantivy's segment format — including pre-upgrade
   checklist (snapshot + dump), verification, and rollback steps.
6. **Deployment verification scripts in `scripts/`:** a post-deploy smoke
   check runnable against any environment (health, create index, index a doc,
   wait for the task, search it, check metrics endpoint, clean up) — usable
   both in CI against a kind/k3d cluster and by operators after an upgrade.
7. **Dropped from the legacy scope:** Terraform modules, Ansible playbooks,
   Kubernetes operator, capacity-planning tooling. Re-open as separate
   proposals only if the family adopts them elsewhere first.

## Impact

- Affected specs: `.rulebook/tasks/phase14_production-deployment/specs/`
  (alerting catalog, backup/retention policy, upgrade procedure)
- Affected code: `deploy/helm/lexum/**`, `deploy/k8s/**`, new
  `deploy/monitoring/**` (Grafana dashboards + Prometheus rules), new
  `docs/runbooks/**`, `docs/DEPLOYMENT.md`, `scripts/` (verification + backup
  scripts), `.github/workflows/` (chart lint/template + kind-based smoke job);
  server code only if the smoke tests expose gaps (filed to owning phases)
- Breaking change: NO (deployment artifacts and docs; chart value renames, if
  any, documented in the chart changelog)
- User benefit: Lexum becomes operable in production by a team that did not
  build it — dashboards to watch, alerts that fire, runbooks that resolve
  them, backups that restore, and upgrades that don't lose data

## Dependencies

- **phase6_ops-observability-surface** (hard): probes, dashboards, and alerts
  are built on its health/stats/Prometheus endpoints and metric names.
- **phase1_write-path-task-queue** (hard): task-queue depth/failure metrics
  and stuck-queue runbook target its lifecycle.
- **phase11_lifecycle-ingest-dumps** (hard for backup/upgrade items): logical
  dumps are the version-portable backup and the Tantivy-bump upgrade path.
- **phase9_distributed-clustering** (soft): cluster-mode chart values, shard
  dashboards, and shard-recovery runbook apply only if phase9 shipped;
  otherwise deliver the single-node subset and mark cluster items deferred.

## Success criteria

- `helm lint` + `helm template` + kubeconform pass in CI; a kind/k3d-based CI
  job installs the chart and runs the deployment verification script green.
- Grafana dashboards import cleanly (schema-valid JSON) and every panel query
  resolves against metric names actually exported by `lexum-server`;
  `promtool check rules` passes on the alert rules.
- Every alert rule links to a runbook section; every runbook scenario lists
  concrete diagnosis commands that work against a real deployment.
- Backup automation demonstrated end-to-end in CI or a documented test run:
  scheduled snapshot taken, retention pruned, and a restore (snapshot AND
  dump) producing a searchable index verified by the smoke script.
- Both upgrade paths (rolling; dump-and-restore) executed at least once
  against a test cluster and documented with the observed steps.
