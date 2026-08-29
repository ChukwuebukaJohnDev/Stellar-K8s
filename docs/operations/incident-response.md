# Incident Response Operational Framework

Structured protocols for Stellar-K8s platform teams responding to node outages, network degradation, and security events.

## Severity classification

| Severity | Definition | Examples | Response target | Escalation |
| --- | --- | --- | --- | --- |
| **SEV-1** | Complete loss of consensus participation or total API outage affecting all users | Validator quorum lost; all Horizon pods down; operator unable to reconcile | 15 min acknowledge, 1 h mitigate | Page on-call + platform lead + Stellar foundation liaison |
| **SEV-2** | Major degradation; partial outage or imminent consensus risk | Single validator down in fragile quorum; ingestion lag >1000; DR drill failure | 30 min acknowledge, 4 h mitigate | Page on-call + team lead |
| **SEV-3** | Limited impact; workaround available | One tenant Horizon slow; archive lag elevated; non-critical reconcile errors | 4 h acknowledge, 24 h resolve | Ticket + Slack channel |
| **SEV-4** | Minor issue; no user impact | Documentation drift; low-volume alert noise; cosmetic UI | Next business day | Backlog |

### Severity decision matrix

Assign the **highest** matching severity:

1. **Data integrity at risk?** → minimum SEV-2; active corruption → SEV-1
2. **Consensus or signing impaired?** → SEV-1 if quorum broken; SEV-2 if at risk
3. **Customer-facing API error rate >5%?** → SEV-2; >50% → SEV-1
4. **Security breach suspected?** → SEV-1 until proven otherwise

## Incident command workflow

1. **Detect** — Alertmanager, `stellar_node_up`, user report, or security SIEM
2. **Triage** — Assign incident commander (IC), severity, and comms lead
3. **Mitigate** — Execute runbook; prefer reversible changes
4. **Resolve** — Confirm metrics green for 30 minutes
5. **Post-mortem** — Required for SEV-1/SEV-2 within 5 business days ([template](../templates/post-mortem-template.md))

Communication channels:

- Internal: `#stellar-incidents` Slack, status page draft
- External: customer email/status page per comms lead approval only

---

## Runbook: Consensus loss

**Triggers:** `stellar_quorum_fragility_score` > 0.9; SCP timeouts; validator not producing ledgers.

### Steps

1. Confirm scope — check `stellar_node_sync_status` and `stellar_quorum_critical_nodes` across validators.
2. Identify failing nodes — `kubectl get stellarnodes -A`; inspect pod events.
3. **Do not** restart more than one validator simultaneously unless quorum math allows it.
4. Check recent config changes — GitOps diff, `StellarNode` spec edits, quorum set updates.
5. If network partition suspected — verify peer connectivity (`stellar_node_active_connections`).
6. Restore minimum quorum:
   - Bring healthy standby validators online
   - Temporarily remove confirmed-bad nodes from quorum set (change management required)
7. Monitor `stellar_quorum_consensus_latency_ms` p99 until stable < 2 s.
8. Open SEV-1 bridge; document timeline for post-mortem.

### Rollback criteria

If ledger gap widens after intervention, halt changes and engage Stellar core experts before further quorum edits.

---

## Runbook: API degradation (Horizon / Soroban RPC)

**Triggers:** Elevated 5xx rate; `stellar_horizon_tps` drop; `soroban_rpc_transaction_result_total{result="failed"}` spike.

### Steps

1. Check upstream core — Horizon/RPC lag often follows `stellar_node_ingestion_lag`.
2. Scale Horizon pods — increase replicas if CPU/memory saturated (HPA or manual).
3. Inspect operator reconcile errors — `increase(stellar_operator_reconcile_errors_total[15m])`.
4. Review rate limits and ingress timeouts at load balancer.
5. For Soroban RPC — check `soroban_rpc_wasm_execution_duration_microseconds` p99 and host memory gauges.
6. Fail over to standby region if primary ingestion lag does not recover in 15 minutes.
7. Communicate degraded performance (SEV-2) until error rate < 1% for 15 minutes.

---

## Runbook: Storage saturation

**Triggers:** `stellar_pvc_disk_usage_percent` > 85; pod evictions; archive write failures.

### Steps

1. Identify PVC — label query: `stellar_pvc_disk_usage_percent` by namespace/name.
2. Verify expansion events — `stellar_pvc_expansion_total` and StorageClass `allowVolumeExpansion`.
3. Trigger operator-driven expansion if configured; otherwise manual PVC patch.
4. For history archives — check `stellar_archive_ledger_lag` and `stellar_archive_integrity_status`.
5. Purge non-essential logs and temp data on node (not ledger DB without runbook approval).
6. If integrity compromised (gauge = 0) — stop serving archive traffic; SEV-2 minimum.
7. Plan capacity — adjust ResourceQuota and order storage before returning to normal.

---

## Runbook: Security breach

**Triggers:** Unauthorized RBAC changes; leaked keys; anomalous contract invocations; CVE scanner alerts.

### Steps

1. **SEV-1 immediately** — assume active compromise until scoped.
2. Preserve evidence — export audit logs, `kubectl logs`, Prometheus snapshots; do not destroy pods yet.
3. Rotate credentials — Stellar seeds, K8s service account tokens, Soroban admin keys.
4. Isolate affected namespaces — apply deny-all NetworkPolicy; scale suspicious workloads to zero.
5. Review RBAC — `kubectl auth can-i --list` for suspicious subjects; compare to [`tenant-policy.yaml`](../../examples/rbac/tenant-policy.yaml).
6. Scan images — CVE sidecar metrics; rebuild from known-good digests.
7. Engage security team for forensic timeline; regulatory notification if PII/funds at risk.
8. Restore from last verified backup after clean rebuild; mandatory post-mortem.

---

## Metrics quick reference during incidents

| Symptom | Primary metrics |
| --- | --- |
| Node down | `stellar_node_up`, `stellar_node_sync_status` |
| Sync issues | `stellar_node_ingestion_lag`, `stellar_archive_ledger_lag` |
| Quorum risk | `stellar_quorum_fragility_score`, `stellar_quorum_critical_nodes` |
| Operator health | `stellar_operator_ready`, `stellar_operator_reconcile_errors_total` |
| RPC failures | `soroban_rpc_transaction_result_total`, `soroban_rpc_host_function_calls_total` |
| Disk pressure | `stellar_pvc_disk_usage_percent` |

## Tooling

```bash
# Operator incident report bundle
stellar-operator incident-report --namespace stellar-system --output /tmp/incident.zip

# Dry-run tenant isolation restore
kubectl apply --dry-run=client -f examples/rbac/tenant-policy.yaml
```

## Related documentation

- [Post-mortem template](../templates/post-mortem-template.md)
- [Metric reference](../observability/metric-reference.md)
- [Multi-tenancy RBAC](../security/multi-tenancy.md)
