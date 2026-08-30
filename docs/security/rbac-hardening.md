# Stellar-K8s security hardening and least-privilege RBAC

This reference defines a deployable least-privilege profile for the `stellar-operator` controller. The companion manifest is [`examples/security/strict-rbac.yaml`](../../examples/security/strict-rbac.yaml); the companion audit is [`examples/security/audit-rbac.sh`](../../examples/security/audit-rbac.sh).

The profile assumes:

- operator namespace: `stellar-operator`
- managed workload namespace: `stellar`
- operator ServiceAccount: `stellar-operator`
- controller started with `--watch-namespace=stellar`

If you change those names, change them consistently in the manifest and audit environment variables. Do **not** use this Role with an all-namespaces controller: RBAC permissions are additive and a cluster-wide watch needs a different, broader review.

## Security invariants

The strict profile has no RBAC wildcards. It intentionally prevents the operator ServiceAccount from creating/binding RBAC roles, creating namespaces, mutating Kubernetes Secrets, using `pods/exec` or `pods/attach`, or creating ServiceAccount tokens. Cluster-scoped access is limited to named Namespace and StorageClass objects.

Validator seed material should be owned by Kubernetes/ESO/Vault/CSI rather than by the Stellar-K8s process. The operator may discover Secrets but does not receive Secret write verbs in this profile.

## Exact permission map

### Managed namespace Role

| API/resource | Verbs | Why |
| --- | --- | --- |
| `stellar.org/stellarnodes` | `get,list,watch,update,patch` | Watch desired state and patch metadata during reconciliation. User/GitOps owns creation/deletion of the primary CR. |
| `stellar.org/stellarnodes/status` | `get,update,patch` | Publish readiness, health, and reconciliation conditions. |
| `stellar.org/stellarnodes/finalizers` | `update` | Finalizer lifecycle. |
| `apps/deployments,statefulsets` | CRUD + `list,watch` | Reconcile node workloads and canary workloads. |
| core `services,configmaps,persistentvolumeclaims` | CRUD + `list,watch` | Reconcile networking, configuration, and persistent storage. |
| core `pods` | `get,list,watch,update,patch,delete` | Health/remediation and managed-pod lifecycle operations. |
| core `pods/log` | `get` | Diagnostics/collection without exec access. |
| core `pods/ephemeralcontainers` | `get,update,patch` | Explicit forensic-snapshot path. Remove this rule if the feature is prohibited. |
| core `secrets` | `get,list,watch` | Read-only discovery/reference; **no Secret writes**. |
| `networking.k8s.io/networkpolicies,ingresses` | CRUD + `list,watch` | Network isolation and optional ingress. |
| `policy/poddisruptionbudgets` | CRUD + `list,watch` | Availability policy. |
| `autoscaling/horizontalpodautoscalers` | CRUD + `list,watch` | HPA when configured. |
| `external-secrets.io/externalsecrets` | `get,create,patch` | Apply/read the ESO object for `seedSecretSource.externalRef`; ESO writes the target Secret under its own SA. |
| `postgresql.cnpg.io/clusters,poolers` | CRUD + `list,watch` | Optional CloudNativePG managed database path. Remove if unused. |
| core `events` | `create,patch` | Kubernetes event publication. |

The manifest deliberately does not grant `create`/`delete` on `StellarNode`, Secret mutation, `pods/exec`, `pods/attach`, RBAC write access, or namespace creation.

### Operator namespace Role

Leader election is isolated to the operator namespace:

```yaml
apiGroups: ["coordination.k8s.io"]
resources: ["leases"]
verbs: ["get", "create", "update", "patch"]
```

### Named cluster-scoped exceptions

`src/controller/pss.rs` patches the managed Namespace on reconcile and network isolation reads it. Because Namespace is cluster-scoped, the profile grants only:

```yaml
resources: ["namespaces"]
resourceNames: ["stellar"]
verbs: ["get", "patch"]
```

The namespace must therefore be pre-created; the ServiceAccount cannot create arbitrary namespaces.

For local-storage auto-detection, `resources.rs` probes only `local-path` and `local-storage`. The profile grants `get` only on those two StorageClass names. If every `StellarNode` sets `spec.storage.storageClass`, remove this ClusterRole and binding.

## Restricted Pod Security Standards

Both namespaces are labelled for `restricted` **enforce**, **audit**, and **warn** using the `latest` policy version. Before enabling enforcement on an existing namespace, start with audit/warn and resolve violations; Pod Security Admission acts at admission time and does not rewrite existing pods.

Stellar-K8s already builds hardened pod/container contexts using non-root execution, `RuntimeDefault` seccomp, `allowPrivilegeEscalation: false`, a read-only root filesystem, and `capabilities.drop: [ALL]`.

The forensic snapshot feature is a deliberate exception: its ephemeral container may request `NET_RAW`/`SYS_PTRACE`, which a truly `restricted` namespace can reject. Treat it as break-glass. Keep it disabled in the strict production profile or use a separately governed diagnostic namespace; do not weaken production PSS just to make diagnostics convenient.

## HashiCorp Vault validator seeds

For `seedSecretSource.vaultRef`, Stellar-K8s adds Vault Agent Injector annotations; it does not fetch the secret value itself. Use a Vault policy that exposes only the required validator path, for example KV-v2:

```hcl
path "kv/data/stellar/validators/validator-0" {
  capabilities = ["read"]
}
```

Bind the Vault Kubernetes-auth role to the **actual workload pod ServiceAccount** and managed namespace used by the deployment:

```bash
vault write auth/kubernetes/role/stellar-validator \
  bound_service_account_names=<validator-workload-service-account> \
  bound_service_account_namespaces=stellar \
  policies=stellar-validator \
  ttl=1h
```

Do not copy a guessed JWT audience from generic examples; audience configuration must match the Kubernetes/Vault auth setup deployed at your site. Enable Vault audit logging, do not grant list/write/delete on validator seed paths, and rotate seeds through Vault rather than Git/Helm values.

Example `StellarNode` fragment:

```yaml
spec:
  validatorConfig:
    seedSecretSource:
      vaultRef:
        role: stellar-validator
        secretPath: kv/data/stellar/validators/validator-0
        secretKey: seed
        secretFileName: stellar-seed
```

## AWS Secrets Manager through ESO

Use External Secrets Operator with IRSA/workload identity. Give the **ESO ServiceAccount**, not the Stellar-K8s operator, a narrowly scoped IAM policy:

```json
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Action": ["secretsmanager:GetSecretValue", "secretsmanager:DescribeSecret"],
    "Resource": "arn:aws:secretsmanager:REGION:ACCOUNT:secret:stellar/prod/validator-*"
  }]
}
```

A platform-managed `SecretStore`/`ClusterSecretStore` can then be referenced from a `StellarNode`:

```yaml
spec:
  validatorConfig:
    seedSecretSource:
      externalRef:
        name: validator-seed
        secretStoreRef:
          name: aws-secrets
          kind: ClusterSecretStore
        remoteKey: stellar/prod/validator-0
        remoteProperty: seed
        refreshInterval: 1h
```

ESO writes the resulting Kubernetes Secret. Stellar-K8s remains read-only to Secret objects in the strict baseline.

## Capabilities intentionally kept out of the baseline

Some features need broader permissions and should be separate reviewed add-on Roles rather than silently widening the baseline: operator-managed mTLS/cert-manager (Secret writes), Prometheus `ServiceMonitor`/`PrometheusRule`, VPA, OCI snapshot Jobs, VolumeSnapshot automation, and Istio resources. The `stellar-k8s benchmark` subcommand is also a separate execution surface that creates load-generator Pods and updates benchmark/report CRs; do not broaden the production `run` controller Role merely to support ad-hoc benchmarks.

## Apply and audit

Apply the reference profile **instead of** layering it on top of a broader generated ClusterRole:

```bash
kubectl apply -f examples/security/strict-rbac.yaml
```

Configure the controller with `operator.watchNamespace: stellar` (or the equivalent `--watch-namespace=stellar`) and use the pre-created ServiceAccount.

Run the audit with:

```bash
bash examples/security/audit-rbac.sh
```

For different names:

```bash
OPERATOR_NAMESPACE=my-operator \
MANAGED_NAMESPACE=my-stellar \
SERVICE_ACCOUNT=my-operator \
bash examples/security/audit-rbac.sh
```

The audit performs server-side manifest validation, positive `auth can-i` checks required by reconciliation, negative checks for high-risk permissions, positive/negative PSS admission tests, and an operator-log scan for `Forbidden`/permission errors.

## Reconciliation validation gate

The final security proof must be run on a disposable/test Kubernetes cluster:

```bash
# Apply CRDs/controller using the strict ServiceAccount and --watch-namespace=stellar.
# Apply a testnet StellarNode in namespace stellar, then:
kubectl get stellarnodes -n stellar -w
kubectl get deploy,statefulset,svc,pvc,networkpolicy,pdb -n stellar
kubectl logs -n stellar-operator deploy/stellar-operator --since=10m \
  | grep -Ei 'forbidden|permission denied|cannot (get|list|watch|create|update|patch|delete)' \
  && echo 'RBAC FAILURE' || echo 'no RBAC errors found'
bash examples/security/audit-rbac.sh
```

Retain an optional rule only if the corresponding feature is exercised. If a feature is not used, remove its rule and repeat the audit. That is how this reference becomes deployment-specific least privilege rather than a permanently broad permission set.

## Security-review rejection criteria

Reject a change if it introduces RBAC wildcards; Secret write verbs; RBAC role/binding writes; namespace creation; cross-namespace Secret reads; `pods/exec`, `pods/attach`, or `serviceaccounts/token`; removes the Namespace `resourceNames` restriction; runs this profile without `--watch-namespace`; broadens Vault/AWS access to global secrets; or weakens `restricted` PSS without an explicit break-glass decision.

The permission map is grounded in the current chart and controller paths: `charts/stellar-operator/templates/rbac.yaml`, `src/controller/reconciler.rs`, `src/controller/resources.rs`, `src/controller/pss.rs`, `src/controller/network_isolation.rs`, `src/controller/kms_secret.rs`, and `charts/stellar-operator/templates/deployment.yaml`. Re-audit this document when those paths change.
