# Custom Resource Definition (CRD) Architecture Reference Manual

This manual is the architectural reference for `stellar-operator` custom resources.
Every field, default, enum, and validation constraint below is taken from the
published OpenAPI schema in [`config/crd/stellarnode-crd.yaml`](../../config/crd/stellarnode-crd.yaml).
Companion machine-generated field listings also live in
[`docs/api-reference.md`](../api-reference.md).

**Do not treat this document as a source of additional API surface.** If a field
is not listed here, it is not present on the published `v1alpha1` schema.

---

## 1. Architecture

Stellar-K8s publishes one namespaced custom resource for node workloads:

| | |
|---|---|
| **CRD name** | `stellarnodes.stellar.org` |
| **API group** | `stellar.org` |
| **Kind** | `StellarNode` |
| **Plural / singular / shortName** | `stellarnodes` / `stellarnode` / `sn` |
| **Scope** | `Namespaced` |
| **Served / storage version** | `v1alpha1` |
| **Subresources** | `status` |

There are **no separate `Horizon` or `SorobanRpc` CRDs** in this repository.
Horizon and Soroban RPC are `spec.nodeType` values on `StellarNode`. Their
type-specific settings are `spec.horizonConfig` and `spec.sorobanConfig`.
Validator-specific settings are `spec.validatorConfig`.

```
apiVersion: stellar.org/v1alpha1
kind: StellarNode
metadata:
  name: <name>
  namespace: <namespace>
spec: { ... }      # required
status: { ... }    # operator-written subresource; status.phase is required when present
```

### 1.1 Node types (`spec.nodeType`)

| Value | Role | Type-specific object |
|---|---|---|
| `Validator` | Stellar Core consensus validator | `spec.validatorConfig` |
| `Horizon` | Horizon REST API / ingestion | `spec.horizonConfig` |
| `SorobanRpc` | Soroban JSON-RPC / captive core | `spec.sorobanConfig` |

The OpenAPI enum is exactly `Validator`, `Horizon`, `SorobanRpc` (PascalCase).

### 1.2 Networks (`spec.network`)

| Value | Meaning |
|---|---|
| `mainnet` | Public Global Stellar Network |
| `testnet` | SDF Test Network |
| `futurenet` | SDF Future Network |
| `custom` | Operator-defined network; pair with `spec.customNetworkPassphrase` |

The OpenAPI enum is lowercase. Values such as `Mainnet` are **not** valid.

### 1.3 kubectl printer columns

| Column | Type | JSONPath |
|---|---|---|
| Type | string | `.spec.nodeType` |
| Network | string | `.spec.network` |
| Ready | string | `.status.conditions[?(@.type=='Ready')].status` |
| Replicas | integer | `.spec.replicas` |
| Age | date | `.metadata.creationTimestamp` |

### 1.4 Related CRDs (out of scope for node workloads)

The repository also ships `StellarBenchmark` and `BenchmarkReport` CRDs under
`config/crd/`. They are not node-workload APIs and are not documented here.

---

## 2. Required versus optional

A `StellarNode` document **must** include `spec`. The OpenAPI `spec.required`
list is:

| Field | Type | Why it is required |
|---|---|---|
| `spec.nodeType` | string enum | Selects Validator / Horizon / SorobanRpc |
| `spec.network` | string enum | Selects the target Stellar network |
| `spec.version` | string | Container image tag for the node software |
| `spec.minAvailable` | IntOrString | PodDisruptionBudget availability floor |
| `spec.maxUnavailable` | IntOrString | PodDisruptionBudget disruption ceiling |
| `spec.topologySpreadConstraints` | array | Topology spread list; may be empty `[]` |

When `status` is present, OpenAPI requires `status.phase`.

Nested objects add their own required keys **only when that object is set**.
Examples:

- `horizonConfig` requires `databaseSecretRef` and `stellarCoreUrl`
- `sorobanConfig` requires `stellarCoreUrl`
- `autoscaling` requires `minReplicas` and `maxReplicas`
- `storage` (when explicitly set) requires `size` and `storageClass`
- `ingress` requires `hosts`
- `drConfig` requires `peerClusterId` and `role`

All other spec fields are optional. Many optional fields carry schema defaults
that the API server applies when the field is omitted.

---

## 3. Production examples

These manifests use only fields that exist on the published schema and include
the six required spec keys:

| Deployment | File |
|---|---|
| Validator (mainnet) | [examples/validator-mainnet.yaml](examples/validator-mainnet.yaml) |
| Horizon API (mainnet) | [examples/horizon-api.yaml](examples/horizon-api.yaml) |
| Soroban RPC (testnet) | [examples/soroban-rpc.yaml](examples/soroban-rpc.yaml) |

Validate locally after applying the CRD:

```bash
kubectl apply -f config/crd/stellarnode-crd.yaml
kubectl apply --dry-run=client -f docs/reference/examples/validator-mainnet.yaml
kubectl apply --dry-run=client -f docs/reference/examples/horizon-api.yaml
kubectl apply --dry-run=client -f docs/reference/examples/soroban-rpc.yaml
```

---

## 4. StellarNode spec catalog

The tables in this section enumerate **every** `spec` path published in the
OpenAPI schema: type, required-when-parent-is-set, default, validation
constraints, and schema description. Nested rows keep the full dotted path.


### 4.1 Identity and core knobs

| Path | Type | Required | Default | Constraints | Purpose |
|---|---|---|---|---|---|
| `spec.alerting` | `boolean` | no | `false` | — | — |
| `spec.customNetworkPassphrase` | `string` | no | — | nullable | — |
| `spec.historyMode` | `string` | no | `Recent` | enum: `Full`, `Recent` | History mode for the node |
| `spec.maintenanceMode` | `boolean` | no | `false` | — | — |
| `spec.maxUnavailable` | `IntOrString` | yes | — | — | IntOrString |
| `spec.minAvailable` | `IntOrString` | yes | — | — | IntOrString |
| `spec.network` | `string` | yes | — | enum: `mainnet`, `testnet`, `futurenet`, `custom` | Target Stellar network |
| `spec.nodeType` | `string` | yes | — | enum: `Validator`, `Horizon`, `SorobanRpc` | Supported Stellar node types |
| `spec.podAntiAffinity` | `string` | no | `Hard` | enum: `Hard`, `Soft`, `Disabled` | When not `Disabled`, the operator adds default pod anti-affinity so pods with the same `stellar-network` label (and same component) are not co-located on one node. |
| `spec.replicas` | `integer (int32)` | no | `1` | — | — |
| `spec.suspended` | `boolean` | no | `false` | — | — |
| `spec.topologySpreadConstraints` | `array of object (preserve-unknown-fields)` | yes | — | — | — |
| `spec.version` | `string` | yes | — | — | — |

### 4.2 Compute, storage, and placement

| Path | Type | Required | Default | Constraints | Purpose |
|---|---|---|---|---|---|
| `spec.resources` | `object` | no | `{limits: {cpu: '2', memory: 4Gi}, requests: {cpu: 500m, memory: 1Gi}}` | — | Kubernetes-style resource requirements |
| `spec.resources.limits` | `object` | yes | — | — | Resource specification for CPU and memory |
| `spec.resources.limits.cpu` | `string` | yes | — | — | — |
| `spec.resources.limits.memory` | `string` | yes | — | — | — |
| `spec.resources.requests` | `object` | yes | — | — | Resource specification for CPU and memory |
| `spec.resources.requests.cpu` | `string` | yes | — | — | — |
| `spec.resources.requests.memory` | `string` | yes | — | — | — |
| `spec.storage` | `object` | no | `{mode: PersistentVolume, retentionPolicy: Delete, size: 100Gi, storageClass: standard}` | — | Storage configuration for persistent data |
| `spec.storage.annotations` | `object` | no | — | nullable | — |
| `spec.storage.mode` | `string` | no | `PersistentVolume` | enum: `PersistentVolume`, `Local` | Storage mode for persistent data |
| `spec.storage.nodeAffinity` | `object (preserve-unknown-fields)` | no | — | — | Node affinity for local storage mode (optional) |
| `spec.storage.retentionPolicy` | `string` | no | `Delete` | enum: `Delete`, `Retain` | PVC retention policy on node deletion |
| `spec.storage.size` | `string` | yes | — | — | — |
| `spec.storage.storageClass` | `string` | yes | — | — | — |
| `spec.storage.snapshotRef` | `object` | no | — | nullable | Bootstrap this node from a pre-computed snapshot or compressed DB backup. Supports CSI VolumeSnapshot (zero-copy PVC clone) or a compressed archive (.tar.gz / .tar.zst) downloaded by an init container before Stellar Core starts. Reduces catch-up time from days to minutes. |
| `spec.storage.snapshotRef.volumeSnapshotName` | `string` | no | — | nullable | Name of an existing VolumeSnapshot (snapshot.storage.k8s.io/v1) in the same namespace. The PVC is provisioned from this snapshot — no init container is needed. |
| `spec.storage.snapshotRef.volumeSnapshotNamespace` | `string` | no | — | nullable | Optional namespace of the VolumeSnapshot when it lives in a different namespace. Requires CrossNamespaceVolumeDataSource feature gate. |
| `spec.storage.snapshotRef.backupUrl` | `string` | no | — | nullable | URL of a compressed DB backup archive (.tar.gz or .tar.zst). Supported schemes: s3://bucket/path/backup.tar.gz or https://host/path/backup.tar.gz. An init container (snapshot-restore) downloads and extracts the archive into /data before Stellar Core starts. |
| `spec.storage.snapshotRef.credentialsSecretRef` | `string` | no | — | nullable | Name of a Kubernetes Secret containing credentials for the backup URL. For S3: keys AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_DEFAULT_REGION. For HTTPS: key BEARER_TOKEN. |
| `spec.storage.snapshotRef.restoreImage` | `string` | no | — | nullable | Container image for the restore init container. Defaults to amazon/aws-cli:latest for S3 URLs, alpine:3 for HTTPS. |
| `spec.restoreFromSnapshot` | `object` | no | — | nullable | Bootstrap this node from an existing VolumeSnapshot instead of an empty volume (Validator only). The PVC will be created from the specified snapshot for near-instant startup. |
| `spec.restoreFromSnapshot.namespace` | `string` | no | — | nullable | Optional: namespace of the VolumeSnapshot if different from the StellarNode. Requires CrossNamespaceVolumeDataSource where supported. |
| `spec.restoreFromSnapshot.volumeSnapshotName` | `string` | yes | — | — | Name of the VolumeSnapshot to restore from (must exist in the same namespace as the StellarNode). |

### 4.3 Node-type configuration

| Path | Type | Required | Default | Constraints | Purpose |
|---|---|---|---|---|---|
| `spec.validatorConfig` | `object` | no | — | nullable | Validator-specific configuration |
| `spec.validatorConfig.catchupComplete` | `boolean` | no | `false` | — | Node is in catchup mode (syncing historical data) |
| `spec.validatorConfig.enableHistoryArchive` | `boolean` | no | `false` | — | Enable history archive for this validator |
| `spec.validatorConfig.historyArchiveUrls` | `array of string` | no | — | — | History archive URLs to fetch from |
| `spec.validatorConfig.hsmConfig` | `object` | no | — | nullable | Cloud HSM configuration for secure key loading (optional) |
| `spec.validatorConfig.hsmConfig.hsmCredentialsSecretRef` | `string` | no | — | nullable | — |
| `spec.validatorConfig.hsmConfig.hsmIp` | `string` | no | — | nullable | — |
| `spec.validatorConfig.hsmConfig.pkcs11LibPath` | `string` | yes | — | — | — |
| `spec.validatorConfig.hsmConfig.provider` | `string` | yes | — | enum: `AWS`, `Azure` | Supported HSM Providers |
| `spec.validatorConfig.keySource` | `string` | no | `secret` | enum: `secret`, `kMS` | Source of the validator seed (Secret or KMS) |
| `spec.validatorConfig.kmsConfig` | `object` | no | — | nullable | KMS configuration for fetching the validator seed |
| `spec.validatorConfig.kmsConfig.fetcherImage` | `string` | no | — | nullable | — |
| `spec.validatorConfig.kmsConfig.keyId` | `string` | yes | — | — | — |
| `spec.validatorConfig.kmsConfig.provider` | `string` | yes | — | — | — |
| `spec.validatorConfig.kmsConfig.region` | `string` | no | — | nullable | — |
| `spec.validatorConfig.quorumSet` | `string` | no | — | nullable | Quorum set configuration as TOML string |
| `spec.validatorConfig.seedSecretRef` | `string` | no | `""` | — | Secret name containing the validator seed (key: STELLAR_CORE_SEED) DEPRECATED: Use seed_secret_source for KMS/ESO/CSI-backed secrets in production |
| `spec.validatorConfig.seedSecretSource` | `object` | no | — | nullable | Production seed source: ESO (AWS SM / GCP SM / Vault) or CSI Secret Store Driver. Takes precedence over seed_secret_ref when present. |
| `spec.validatorConfig.seedSecretSource.csiRef` | `object` | no | — | nullable | Secrets Store CSI Driver — **recommended for production**.  Mounts the seed directly from a KMS/Vault into the pod filesystem via a CSI volume.  The seed is never written to etcd.  The controller injects `STELLAR_SEED_FILE` into the container pointing at the mount path; stellar-core reads the key from that file path. |
| `spec.validatorConfig.seedSecretSource.csiRef.mountPath` | `string` | no | `/mnt/secrets/validator` | nullable | Directory inside the container where the CSI driver mounts secrets. Defaults to `/mnt/secrets/validator`. |
| `spec.validatorConfig.seedSecretSource.csiRef.secretProviderClassName` | `string` | yes | — | — | Name of the `SecretProviderClass` CR (from secrets-store.csi.x-k8s.io) that defines which secrets to mount and from which provider. |
| `spec.validatorConfig.seedSecretSource.csiRef.seedFileName` | `string` | no | `seed` | nullable | File name within `mount_path` that contains the seed value. Defaults to `seed`. |
| `spec.validatorConfig.seedSecretSource.externalRef` | `object` | no | — | nullable | External Secrets Operator — **recommended for production**.  The operator creates an `ExternalSecret` CR which causes ESO to pull the seed from AWS Secrets Manager, GCP Secret Manager, HashiCorp Vault, or any other supported backend and materialise it as a Kubernetes Secret in the same namespace.  The seed value is never stored in the CRD itself. |
| `spec.validatorConfig.seedSecretSource.externalRef.name` | `string` | yes | — | — | Name of the `ExternalSecret` CR the operator will create/manage. Must be unique within the namespace. |
| `spec.validatorConfig.seedSecretSource.externalRef.refreshInterval` | `string` | no | `1h` | nullable | How often ESO should re-sync the secret from the remote backend. Kubernetes duration string, e.g. `"1h"`, `"30m"`. Defaults to `"1h"` if not specified. |
| `spec.validatorConfig.seedSecretSource.externalRef.remoteKey` | `string` | yes | — | — | Path / identifier of the secret in the remote backend.  Examples: - AWS Secrets Manager: `"prod/stellar/validator-seed"` - GCP Secret Manager: `"projects/MY_PROJECT/secrets/stellar-validator-seed"` - HashiCorp Vault: `"secret/data/stellar/validator"` |
| `spec.validatorConfig.seedSecretSource.externalRef.remoteProperty` | `string` | no | — | nullable | Property (field) inside the remote secret to extract.  Required for secrets that store a JSON object (e.g., `{"seed": "S..."}`) and you only want the `seed` value.  Leave empty to use the whole secret value as the seed. |
| `spec.validatorConfig.seedSecretSource.externalRef.secretStoreRef` | `object` | yes | — | — | Reference to the `SecretStore` or `ClusterSecretStore` that connects ESO to the remote backend (AWS SM, GCP SM, Vault, etc.). |
| `spec.validatorConfig.seedSecretSource.externalRef.secretStoreRef.kind` | `string` | no | `ClusterSecretStore` | — | Kind of the store resource.  - `"SecretStore"` — namespaced store (only works within the same namespace) - `"ClusterSecretStore"` — cluster-wide store (recommended for production) |
| `spec.validatorConfig.seedSecretSource.externalRef.secretStoreRef.name` | `string` | yes | — | — | Name of the `SecretStore` / `ClusterSecretStore` resource. |
| `spec.validatorConfig.seedSecretSource.localRef` | `object` | no | — | nullable | Plain Kubernetes Secret — **development only**.  Points to an existing `Secret` in the same namespace.  The secret must contain the key specified in `key` (defaults to `STELLAR_CORE_SEED`). |
| `spec.validatorConfig.seedSecretSource.localRef.key` | `string` | no | `STELLAR_CORE_SEED` | nullable | Key within the secret that holds the seed value. Defaults to `STELLAR_CORE_SEED` if not specified. |
| `spec.validatorConfig.seedSecretSource.localRef.name` | `string` | yes | — | — | Name of the `Secret` in the same namespace. |
| `spec.validatorConfig.seedSecretSource.vaultRef` | `object` | no | — | nullable | HashiCorp Vault via the **Vault Agent Injector** (init + sidecar).  Requires the Vault Agent Injector mutating webhook in the cluster. The operator sets standard `vault.hashicorp.com/*` pod annotations; the injector adds the Vault Agent containers and renders the secret file under `/vault/secrets/`. |
| `spec.validatorConfig.seedSecretSource.vaultRef.extraPodAnnotations` | `array of object` | no | — | — | Additional `vault.hashicorp.com/*` or other pod annotations to merge. |
| `spec.validatorConfig.seedSecretSource.vaultRef.extraPodAnnotations.[]` | `object` | no | — | — | Key/value pair for extra Vault Agent pod annotations (CRD-friendly vs raw maps). |
| `spec.validatorConfig.seedSecretSource.vaultRef.extraPodAnnotations.[].name` | `string` | yes | — | — | — |
| `spec.validatorConfig.seedSecretSource.vaultRef.extraPodAnnotations.[].value` | `string` | yes | — | — | — |
| `spec.validatorConfig.seedSecretSource.vaultRef.restartOnSecretRotation` | `boolean` | no | `false` | — | When true, the operator compares Vault secret-version annotations on pods and rolls the StatefulSet when the version changes after sync. |
| `spec.validatorConfig.seedSecretSource.vaultRef.role` | `string` | yes | — | — | Vault Kubernetes auth role bound to this pod's ServiceAccount. |
| `spec.validatorConfig.seedSecretSource.vaultRef.secretFileName` | `string` | no | — | nullable | Base file name rendered under `/vault/secrets/` (default `stellar-seed`). |
| `spec.validatorConfig.seedSecretSource.vaultRef.secretKey` | `string` | no | — | nullable | JSON field under `.Data.data` for KV v2 (default `seed`). Ignored if `template` is set. |
| `spec.validatorConfig.seedSecretSource.vaultRef.secretPath` | `string` | yes | — | — | Path passed to `vault.hashicorp.com/agent-inject-secret-<file>` (KV v1/v2 path as in Vault). |
| `spec.validatorConfig.seedSecretSource.vaultRef.template` | `string` | no | — | nullable | Custom Agent template; when set, overrides the default KV v2 template. |
| `spec.validatorConfig.vlSource` | `string` | no | — | nullable | Trusted source for Validator Selection List (VSL) |
| `spec.horizonConfig` | `object` | no | — | nullable | Horizon API server configuration |
| `spec.horizonConfig.autoMigration` | `boolean` | no | `true` | — | — |
| `spec.horizonConfig.databaseSecretRef` | `string` | yes | — | — | — |
| `spec.horizonConfig.enableExperimentalIngestion` | `boolean` | no | `false` | — | — |
| `spec.horizonConfig.enableIngest` | `boolean` | no | `true` | — | — |
| `spec.horizonConfig.ingestWorkers` | `integer (uint32)` | no | `1` | min `0.0` | — |
| `spec.horizonConfig.stellarCoreUrl` | `string` | yes | — | — | — |
| `spec.sorobanConfig` | `object` | no | — | nullable | Soroban RPC server configuration |
| `spec.sorobanConfig.captiveCoreConfig` | `string` | no | — | nullable | — |
| `spec.sorobanConfig.captiveCoreStructuredConfig` | `object` | no | — | nullable | Captive Core configuration for Soroban RPC |
| `spec.sorobanConfig.captiveCoreStructuredConfig.additionalConfig` | `string` | no | — | nullable | — |
| `spec.sorobanConfig.captiveCoreStructuredConfig.historyArchiveUrls` | `array of string` | no | `[]` | — | — |
| `spec.sorobanConfig.captiveCoreStructuredConfig.httpPort` | `integer (uint16)` | no | — | nullable; min `0.0` | — |
| `spec.sorobanConfig.captiveCoreStructuredConfig.logLevel` | `string` | no | — | nullable | — |
| `spec.sorobanConfig.captiveCoreStructuredConfig.networkPassphrase` | `string` | no | — | nullable | — |
| `spec.sorobanConfig.captiveCoreStructuredConfig.peerPort` | `integer (uint16)` | no | — | nullable; min `0.0` | — |
| `spec.sorobanConfig.enablePreflight` | `boolean` | no | `true` | — | — |
| `spec.sorobanConfig.maxEventsPerRequest` | `integer (uint32)` | no | `10000` | min `0.0` | — |
| `spec.sorobanConfig.cache` | `object` | no | — | nullable | Bounded fail-open cache for read-only Soroban RPC state requests |
| `spec.sorobanConfig.cache.enabled` | `boolean` | no | `false` | — | — |
| `spec.sorobanConfig.cache.ttlSecs` | `integer (int64)` | no | `30` | min `1.0` | — |
| `spec.sorobanConfig.cache.maxEntries` | `integer (int64)` | no | `10000` | min `1.0`; max `10000.0` | — |
| `spec.sorobanConfig.cache.maxBytes` | `integer (int64)` | no | `67108864` | min `1.0`; max `67108864.0` | — |
| `spec.sorobanConfig.cache.image` | `string` | no | — | nullable | — |
| `spec.sorobanConfig.stellarCoreUrl` | `string` | yes | — | — | — |

### 4.4 Scaling and rollout

| Path | Type | Required | Default | Constraints | Purpose |
|---|---|---|---|---|---|
| `spec.autoscaling` | `object` | no | — | nullable | Horizontal Pod Autoscaling configuration |
| `spec.autoscaling.behavior` | `object` | no | — | nullable | Scaling behavior configuration for HPA |
| `spec.autoscaling.behavior.scaleDown` | `object` | no | — | nullable | Scaling policy |
| `spec.autoscaling.behavior.scaleDown.policies` | `array of object` | no | — | — | — |
| `spec.autoscaling.behavior.scaleDown.policies.[]` | `object` | no | — | — | Individual HPA policy |
| `spec.autoscaling.behavior.scaleDown.policies.[].periodSeconds` | `integer (int32)` | yes | — | — | — |
| `spec.autoscaling.behavior.scaleDown.policies.[].policyType` | `string` | yes | — | — | — |
| `spec.autoscaling.behavior.scaleDown.policies.[].value` | `integer (int32)` | yes | — | — | — |
| `spec.autoscaling.behavior.scaleDown.stabilizationWindowSeconds` | `integer (int32)` | no | — | nullable | — |
| `spec.autoscaling.behavior.scaleUp` | `object` | no | — | nullable | Scaling policy |
| `spec.autoscaling.behavior.scaleUp.policies` | `array of object` | no | — | — | — |
| `spec.autoscaling.behavior.scaleUp.policies.[]` | `object` | no | — | — | Individual HPA policy |
| `spec.autoscaling.behavior.scaleUp.policies.[].periodSeconds` | `integer (int32)` | yes | — | — | — |
| `spec.autoscaling.behavior.scaleUp.policies.[].policyType` | `string` | yes | — | — | — |
| `spec.autoscaling.behavior.scaleUp.policies.[].value` | `integer (int32)` | yes | — | — | — |
| `spec.autoscaling.behavior.scaleUp.stabilizationWindowSeconds` | `integer (int32)` | no | — | nullable | — |
| `spec.autoscaling.customMetrics` | `array of string` | no | — | — | — |
| `spec.autoscaling.maxReplicas` | `integer (int32)` | yes | — | — | — |
| `spec.autoscaling.minReplicas` | `integer (int32)` | yes | — | — | — |
| `spec.autoscaling.targetCpuUtilizationPercentage` | `integer (int32)` | no | — | nullable | — |
| `spec.vpaConfig` | `object` | no | — | nullable | VPA configuration |
| `spec.vpaConfig.containerPolicies` | `array of object` | no | — | — | — |
| `spec.vpaConfig.containerPolicies.[]` | `object` | no | — | — | Per-container resource policy for the VPA |
| `spec.vpaConfig.containerPolicies.[].containerName` | `string` | yes | — | — | — |
| `spec.vpaConfig.containerPolicies.[].maxAllowed` | `object` | no | — | nullable | — |
| `spec.vpaConfig.containerPolicies.[].minAllowed` | `object` | no | — | nullable | — |
| `spec.vpaConfig.updateMode` | `string` | no | `Initial` | enum: `Initial`, `Auto` | VPA update mode |
| `spec.strategy` | `object` | no | `{type: rollingUpdate}` | — | Rollout strategy for updates (RollingUpdate or Canary) |
| `spec.strategy.canary` | `object` | no | — | nullable | Configuration for Canary rollout |
| `spec.strategy.canary.checkIntervalSeconds` | `integer (int32)` | no | `300` | — | — |
| `spec.strategy.canary.weight` | `integer (int32)` | no | `10` | — | — |
| `spec.strategy.type` | `string` | yes | — | enum: `rollingUpdate`, `canary` | Rollout strategy type |
| `spec.readReplicaConfig` | `object` | no | — | nullable | Read replica pool configuration for horizontal scaling Enables creating read-only replicas with traffic routing strategies |
| `spec.readReplicaConfig.archiveSharding` | `boolean` | no | `false` | — | Enable history archive sharding When true, replicas serve different archives to balance bandwidth |
| `spec.readReplicaConfig.replicas` | `integer (int32)` | no | `1` | — | Number of read-only replicas |
| `spec.readReplicaConfig.resources` | `object` | no | `{limits: {cpu: '2', memory: 4Gi}, requests: {cpu: 500m, memory: 1Gi}}` | — | Compute resource requirements for read replicas |
| `spec.readReplicaConfig.resources.limits` | `object` | yes | — | — | Resource specification for CPU and memory |
| `spec.readReplicaConfig.resources.limits.cpu` | `string` | yes | — | — | — |
| `spec.readReplicaConfig.resources.limits.memory` | `string` | yes | — | — | — |
| `spec.readReplicaConfig.resources.requests` | `object` | yes | — | — | Resource specification for CPU and memory |
| `spec.readReplicaConfig.resources.requests.cpu` | `string` | yes | — | — | — |
| `spec.readReplicaConfig.resources.requests.memory` | `string` | yes | — | — | — |
| `spec.readReplicaConfig.strategy` | `string` | no | `RoundRobin` | enum: `RoundRobin`, `FreshnessPreferred` | Load balancing strategy |
| `spec.readPoolEndpoint` | `string` | no | — | nullable | DNS endpoint for the read-replica pool Service. |

### 4.5 Ingress, load balancing, and network policy

| Path | Type | Required | Default | Constraints | Purpose |
|---|---|---|---|---|---|
| `spec.ingress` | `object` | no | — | nullable | Ingress configuration |
| `spec.ingress.annotations` | `object` | no | — | nullable | — |
| `spec.ingress.certManagerClusterIssuer` | `string` | no | — | nullable | — |
| `spec.ingress.certManagerIssuer` | `string` | no | — | nullable | — |
| `spec.ingress.className` | `string` | no | — | nullable | — |
| `spec.ingress.hosts` | `array of object` | yes | — | — | — |
| `spec.ingress.hosts.[]` | `object` | no | — | — | Ingress host entry |
| `spec.ingress.hosts.[].host` | `string` | yes | — | — | — |
| `spec.ingress.hosts.[].paths` | `array of object` | no | `[{path: /, pathType: Prefix}]` | — | — |
| `spec.ingress.hosts.[].paths.[]` | `object` | no | — | — | Ingress path mapping |
| `spec.ingress.hosts.[].paths.[].path` | `string` | yes | — | — | — |
| `spec.ingress.hosts.[].paths.[].pathType` | `string` | no | `Prefix` | nullable | — |
| `spec.ingress.tlsSecretName` | `string` | no | — | nullable | — |
| `spec.loadBalancer` | `object` | no | — | nullable | Load balancer configuration for external access (e.g. MetalLB) |
| `spec.loadBalancer.addressPool` | `string` | no | — | nullable | — |
| `spec.loadBalancer.annotations` | `object` | no | — | nullable | — |
| `spec.loadBalancer.bgp` | `object` | no | — | nullable | BGP configuration for MetalLB anycast routing |
| `spec.loadBalancer.bgp.advertisement` | `object` | no | — | nullable | BGP advertisement configuration |
| `spec.loadBalancer.bgp.advertisement.aggregationLength` | `integer (uint8)` | no | `32` | min `0.0` | — |
| `spec.loadBalancer.bgp.advertisement.aggregationLengthV6` | `integer (uint8)` | no | `128` | min `0.0` | — |
| `spec.loadBalancer.bgp.advertisement.localPref` | `integer (uint32)` | no | — | nullable; min `0.0` | — |
| `spec.loadBalancer.bgp.advertisement.nodeSelectors` | `object` | no | — | nullable | — |
| `spec.loadBalancer.bgp.bfdEnabled` | `boolean` | no | `false` | — | — |
| `spec.loadBalancer.bgp.bfdProfile` | `string` | no | — | nullable | — |
| `spec.loadBalancer.bgp.communities` | `array of string` | no | — | — | — |
| `spec.loadBalancer.bgp.largeCommunities` | `array of string` | no | — | — | — |
| `spec.loadBalancer.bgp.localAsn` | `integer (uint32)` | yes | — | min `0.0` | — |
| `spec.loadBalancer.bgp.nodeSelectors` | `object` | no | — | nullable | — |
| `spec.loadBalancer.bgp.peers` | `array of object` | no | — | — | — |
| `spec.loadBalancer.bgp.peers.[]` | `object` | no | — | — | BGP peer router configuration |
| `spec.loadBalancer.bgp.peers.[].address` | `string` | yes | — | — | — |
| `spec.loadBalancer.bgp.peers.[].asn` | `integer (uint32)` | yes | — | min `0.0` | — |
| `spec.loadBalancer.bgp.peers.[].ebgpMultiHop` | `boolean` | no | `false` | — | — |
| `spec.loadBalancer.bgp.peers.[].gracefulRestart` | `boolean` | no | `true` | — | — |
| `spec.loadBalancer.bgp.peers.[].holdTime` | `integer (uint32)` | no | `90` | min `0.0` | — |
| `spec.loadBalancer.bgp.peers.[].keepaliveTime` | `integer (uint32)` | no | `30` | min `0.0` | — |
| `spec.loadBalancer.bgp.peers.[].passwordSecretRef` | `object` | no | — | nullable | Reference to a key within a Kubernetes Secret |
| `spec.loadBalancer.bgp.peers.[].passwordSecretRef.key` | `string` | yes | — | — | — |
| `spec.loadBalancer.bgp.peers.[].passwordSecretRef.name` | `string` | yes | — | — | — |
| `spec.loadBalancer.bgp.peers.[].port` | `integer (uint16)` | no | `179` | min `0.0` | — |
| `spec.loadBalancer.bgp.peers.[].routerId` | `string` | no | — | nullable | — |
| `spec.loadBalancer.bgp.peers.[].sourceAddress` | `string` | no | — | nullable | — |
| `spec.loadBalancer.enabled` | `boolean` | no | `false` | — | — |
| `spec.loadBalancer.externalTrafficPolicy` | `string` | no | `Cluster` | enum: `Cluster`, `Local` | External traffic policy for LoadBalancer services |
| `spec.loadBalancer.healthCheckEnabled` | `boolean` | no | `true` | — | — |
| `spec.loadBalancer.healthCheckPort` | `integer (int32)` | no | `9100` | — | — |
| `spec.loadBalancer.loadBalancerIp` | `string` | no | — | nullable | — |
| `spec.loadBalancer.mode` | `string` | no | `L2` | enum: `L2`, `BGP` | Load balancer mode selection |
| `spec.networkPolicy` | `object` | no | — | nullable | Network Policy configuration |
| `spec.networkPolicy.allowCidrs` | `array of string` | no | — | — | — |
| `spec.networkPolicy.allowMetricsScrape` | `boolean` | no | `true` | — | — |
| `spec.networkPolicy.allowNamespaces` | `array of string` | no | — | — | — |
| `spec.networkPolicy.allowPodSelector` | `object` | no | — | nullable | — |
| `spec.networkPolicy.enabled` | `boolean` | no | `false` | — | — |
| `spec.networkPolicy.metricsNamespace` | `string` | no | `monitoring` | — | — |

### 4.6 Database

| Path | Type | Required | Default | Constraints | Purpose |
|---|---|---|---|---|---|
| `spec.database` | `object` | no | — | nullable | External database configuration for managed Postgres databases |
| `spec.database.secretKeyRef` | `object` | yes | — | — | Reference to a key within a Kubernetes Secret |
| `spec.database.secretKeyRef.key` | `string` | yes | — | — | — |
| `spec.database.secretKeyRef.name` | `string` | yes | — | — | — |
| `spec.managedDatabase` | `object` | no | — | nullable | Configuration for managed High-Availability Postgres clusters via CloudNativePG |
| `spec.managedDatabase.backup` | `object` | no | — | nullable | Backup configuration for managed databases using Barman |
| `spec.managedDatabase.backup.credentialsSecretRef` | `string` | yes | — | — | — |
| `spec.managedDatabase.backup.destinationPath` | `string` | yes | — | — | — |
| `spec.managedDatabase.backup.enabled` | `boolean` | no | `true` | — | — |
| `spec.managedDatabase.backup.retentionPolicy` | `string` | no | `30d` | — | — |
| `spec.managedDatabase.instances` | `integer (int32)` | no | `3` | — | — |
| `spec.managedDatabase.pooling` | `object` | no | — | nullable | pgBouncer connection pooling configuration |
| `spec.managedDatabase.pooling.defaultPoolSize` | `integer (int32)` | no | `20` | — | — |
| `spec.managedDatabase.pooling.enabled` | `boolean` | no | `true` | — | — |
| `spec.managedDatabase.pooling.maxClientConn` | `integer (int32)` | no | `1000` | — | — |
| `spec.managedDatabase.pooling.poolMode` | `string` | no | `transaction` | enum: `session`, `transaction`, `statement` | pgBouncer pooling modes |
| `spec.managedDatabase.pooling.replicas` | `integer (int32)` | no | `2` | — | — |
| `spec.managedDatabase.postgresVersion` | `string` | no | `16` | — | — |
| `spec.managedDatabase.storage` | `object` | yes | — | — | Storage configuration for persistent data |
| `spec.managedDatabase.storage.annotations` | `object` | no | — | nullable | — |
| `spec.managedDatabase.storage.mode` | `string` | no | `PersistentVolume` | enum: `PersistentVolume`, `Local` | Storage mode for persistent data |
| `spec.managedDatabase.storage.nodeAffinity` | `object (preserve-unknown-fields)` | no | — | — | Node affinity for local storage mode (optional) |
| `spec.managedDatabase.storage.retentionPolicy` | `string` | no | `Delete` | enum: `Delete`, `Retain` | PVC retention policy on node deletion |
| `spec.managedDatabase.storage.size` | `string` | yes | — | — | — |
| `spec.managedDatabase.storage.storageClass` | `string` | yes | — | — | — |
| `spec.dbMaintenanceConfig` | `object` | no | — | nullable | Database maintenance configuration for automated vacuum and reindexing Enables periodic maintenance windows for performance optimization |
| `spec.dbMaintenanceConfig.autoReindex` | `boolean` | no | `true` | — | Automatically reindex bloated tables |
| `spec.dbMaintenanceConfig.bloatThresholdPercent` | `integer (uint32)` | no | `30` | min `0.0` | Bloat threshold percentage to trigger VACUUM FULL (default: 30) |
| `spec.dbMaintenanceConfig.enabled` | `boolean` | no | `true` | — | Enable automated database maintenance |
| `spec.dbMaintenanceConfig.readPoolCoordination` | `boolean` | no | `true` | — | Coordination with read-pool for zero-downtime |
| `spec.dbMaintenanceConfig.windowDuration` | `string` | yes | — | — | Maintenance window duration (e.g., "2h") |
| `spec.dbMaintenanceConfig.windowStart` | `string` | yes | — | — | Maintenance window start time (24h format, e.g., "02:00") Maintenance will only trigger during this window |

### 4.7 Cross-cluster, discovery, and DR

| Path | Type | Required | Default | Constraints | Purpose |
|---|---|---|---|---|---|
| `spec.crossCluster` | `object` | no | — | nullable | Cross-cluster configuration for multi-cluster federation |
| `spec.crossCluster.autoDiscovery` | `boolean` | no | `false` | — | — |
| `spec.crossCluster.enabled` | `boolean` | no | `false` | — | — |
| `spec.crossCluster.externalName` | `object` | no | — | nullable | ExternalName service configuration |
| `spec.crossCluster.externalName.createExternalNameServices` | `boolean` | no | `true` | — | — |
| `spec.crossCluster.externalName.dnsProvider` | `string` | no | — | nullable | — |
| `spec.crossCluster.externalName.externalDnsName` | `string` | yes | — | — | — |
| `spec.crossCluster.externalName.ttl` | `integer (uint32)` | no | `300` | min `0.0` | — |
| `spec.crossCluster.healthCheck` | `object` | no | — | nullable | Health check configuration for cross-cluster peers |
| `spec.crossCluster.healthCheck.enabled` | `boolean` | no | `true` | — | — |
| `spec.crossCluster.healthCheck.failureThreshold` | `integer (uint32)` | no | `3` | min `0.0` | — |
| `spec.crossCluster.healthCheck.intervalSeconds` | `integer (uint32)` | no | `30` | min `0.0` | — |
| `spec.crossCluster.healthCheck.latencyMeasurement` | `object` | no | — | nullable | Latency measurement configuration |
| `spec.crossCluster.healthCheck.latencyMeasurement.enabled` | `boolean` | no | `true` | — | — |
| `spec.crossCluster.healthCheck.latencyMeasurement.method` | `string` | no | `ping` | enum: `ping`, `tcp`, `http`, `grpc` | Method for measuring cross-cluster latency |
| `spec.crossCluster.healthCheck.latencyMeasurement.percentile` | `integer (uint8)` | no | `95` | min `0.0` | — |
| `spec.crossCluster.healthCheck.latencyMeasurement.sampleCount` | `integer (uint32)` | no | `10` | min `0.0` | — |
| `spec.crossCluster.healthCheck.successThreshold` | `integer (uint32)` | no | `1` | min `0.0` | — |
| `spec.crossCluster.healthCheck.timeoutSeconds` | `integer (uint32)` | no | `5` | min `0.0` | — |
| `spec.crossCluster.latencyThresholdMs` | `integer (uint32)` | no | `200` | min `0.0` | — |
| `spec.crossCluster.mode` | `string` | no | `serviceMesh` | enum: `serviceMesh`, `externalName`, `directIP` | Cross-cluster networking mode |
| `spec.crossCluster.peerClusters` | `array of object` | no | — | — | — |
| `spec.crossCluster.peerClusters.[]` | `object` | no | — | — | Peer cluster configuration |
| `spec.crossCluster.peerClusters.[].clusterId` | `string` | yes | — | — | — |
| `spec.crossCluster.peerClusters.[].enabled` | `boolean` | no | `true` | — | — |
| `spec.crossCluster.peerClusters.[].endpoint` | `string` | yes | — | — | — |
| `spec.crossCluster.peerClusters.[].latencyThresholdMs` | `integer (uint32)` | no | — | nullable; min `0.0` | — |
| `spec.crossCluster.peerClusters.[].port` | `integer (uint16)` | no | — | nullable; min `0.0` | — |
| `spec.crossCluster.peerClusters.[].priority` | `integer (uint32)` | no | `100` | min `0.0` | — |
| `spec.crossCluster.peerClusters.[].region` | `string` | no | — | nullable | — |
| `spec.crossCluster.serviceMesh` | `object` | no | — | nullable | Service mesh configuration for cross-cluster networking |
| `spec.crossCluster.serviceMesh.clusterSetId` | `string` | no | — | nullable | — |
| `spec.crossCluster.serviceMesh.meshType` | `string` | yes | — | enum: `submariner`, `istio`, `linkerd`, `cilium` | Supported service mesh types for cross-cluster networking |
| `spec.crossCluster.serviceMesh.mtlsEnabled` | `boolean` | no | `true` | — | — |
| `spec.crossCluster.serviceMesh.serviceExport` | `object` | no | — | nullable | Service export configuration |
| `spec.crossCluster.serviceMesh.serviceExport.enabled` | `boolean` | no | `true` | — | — |
| `spec.crossCluster.serviceMesh.serviceExport.namespace` | `string` | no | — | nullable | — |
| `spec.crossCluster.serviceMesh.serviceExport.serviceName` | `string` | no | — | nullable | — |
| `spec.crossCluster.serviceMesh.serviceExport.targetClusters` | `array of string` | no | — | — | — |
| `spec.crossCluster.serviceMesh.trafficPolicy` | `string` | no | `localPreferred` | enum: `localPreferred`, `global`, `localOnly`, `latencyBased` | Traffic policy for cross-cluster routing |
| `spec.globalDiscovery` | `object` | no | — | nullable | Global discovery configuration for cross-cluster discovery |
| `spec.globalDiscovery.enabled` | `boolean` | no | `false` | — | — |
| `spec.globalDiscovery.externalDns` | `object` | no | — | nullable | ExternalDNS configuration |
| `spec.globalDiscovery.externalDns.annotations` | `object` | no | — | nullable | — |
| `spec.globalDiscovery.externalDns.hostname` | `string` | yes | — | — | — |
| `spec.globalDiscovery.externalDns.provider` | `string` | no | — | nullable | — |
| `spec.globalDiscovery.externalDns.ttl` | `integer (uint32)` | no | `300` | min `0.0` | — |
| `spec.globalDiscovery.priority` | `integer (uint32)` | no | `100` | min `0.0` | — |
| `spec.globalDiscovery.region` | `string` | no | — | nullable | — |
| `spec.globalDiscovery.serviceMesh` | `object` | no | — | nullable | Service mesh integration configuration |
| `spec.globalDiscovery.serviceMesh.meshType` | `string` | yes | — | enum: `istio`, `linkerd`, `consul` | Supported service mesh implementations |
| `spec.globalDiscovery.serviceMesh.mtlsMode` | `string` | no | `PERMISSIVE` | enum: `DISABLE`, `PERMISSIVE`, `STRICT` | mTLS enforcement mode |
| `spec.globalDiscovery.serviceMesh.sidecarInjection` | `boolean` | no | `true` | — | — |
| `spec.globalDiscovery.serviceMesh.virtualServiceHost` | `string` | no | — | nullable | — |
| `spec.globalDiscovery.topologyAwareHints` | `boolean` | no | `false` | — | — |
| `spec.globalDiscovery.zone` | `string` | no | — | nullable | — |
| `spec.drConfig` | `object` | no | — | nullable | Configuration for multi-cluster disaster recovery |
| `spec.drConfig.drillSchedule` | `object` | no | — | nullable | Configuration for automated DR drill scheduling |
| `spec.drConfig.drillSchedule.autoRollback` | `boolean` | no | `true` | — | Whether to automatically rollback after drill completion |
| `spec.drConfig.drillSchedule.dryRun` | `boolean` | no | `false` | — | Whether to actually perform failover or just simulate it (dry-run) |
| `spec.drConfig.drillSchedule.rollbackDelaySeconds` | `integer (uint32)` | no | `60` | min `0.0` | Rollback delay after drill completion (seconds) |
| `spec.drConfig.drillSchedule.schedule` | `string` | yes | — | — | Cron expression for drill scheduling (e.g., "0 2 * * 0" for weekly Sunday 2 AM) |
| `spec.drConfig.drillSchedule.timeoutSeconds` | `integer (uint32)` | no | `300` | min `0.0` | Maximum time to wait for failover to complete (seconds) |
| `spec.drConfig.enabled` | `boolean` | no | `false` | — | — |
| `spec.drConfig.failoverDns` | `object` | no | — | nullable | ExternalDNS configuration |
| `spec.drConfig.failoverDns.annotations` | `object` | no | — | nullable | — |
| `spec.drConfig.failoverDns.hostname` | `string` | yes | — | — | — |
| `spec.drConfig.failoverDns.provider` | `string` | no | — | nullable | — |
| `spec.drConfig.failoverDns.ttl` | `integer (uint32)` | no | `300` | min `0.0` | — |
| `spec.drConfig.healthCheckInterval` | `integer (uint32)` | no | `30` | min `0.0` | — |
| `spec.drConfig.peerClusterId` | `string` | yes | — | — | — |
| `spec.drConfig.role` | `string` | yes | — | enum: `primary`, `standby` | Role of a node in a DR configuration |
| `spec.drConfig.syncStrategy` | `string` | no | `consensus` | enum: `consensus`, `peertracking`, `archivesync` | Synchronization strategy for hot standby nodes |

### 4.8 Snapshots, CVE handling, and mesh

| Path | Type | Required | Default | Constraints | Purpose |
|---|---|---|---|---|---|
| `spec.snapshotSchedule` | `object` | no | — | nullable | Schedule and options for taking CSI VolumeSnapshots of the node's data PVC (Validator only). Enables zero-downtime backups and creating new nodes from snapshots. |
| `spec.snapshotSchedule.flushBeforeSnapshot` | `boolean` | no | `false` | — | If true, the operator will attempt to flush/lock the Stellar database briefly before creating the snapshot (e.g. via stellar-core HTTP or exec). Requires the node to be healthy. |
| `spec.snapshotSchedule.retentionCount` | `integer (uint32)` | no | `0` | min `0.0` | Maximum number of snapshots to retain per node. Oldest snapshots are deleted when exceeded. 0 means no limit. |
| `spec.snapshotSchedule.schedule` | `string` | no | — | nullable | Cron expression for scheduled snapshots (e.g. "0 2 * * *" for daily at 2 AM). If unset, snapshots are only taken when triggered via annotation `stellar.org/request-snapshot: "true"`. |
| `spec.snapshotSchedule.volumeSnapshotClassName` | `string` | no | — | nullable | VolumeSnapshotClass name. If unset, the default class for the PVC's driver is used. |
| `spec.ociSnapshot` | `object` | no | — | nullable | OCI-based ledger snapshot sync for multi-region bootstrapping |
| `spec.ociSnapshot.credentialSecretName` | `string` | yes | — | — | Name of a K8s Secret in the same namespace containing Docker registry credentials as `config.json` (standard `~/.docker/config.json` format). |
| `spec.ociSnapshot.enabled` | `boolean` | no | `false` | — | Whether the OCI snapshot feature is enabled (default: false) |
| `spec.ociSnapshot.fixedTag` | `string` | no | — | nullable | Fixed tag to use when `tag_strategy` is `Fixed` (e.g. `latest`) |
| `spec.ociSnapshot.image` | `string` | yes | — | — | Image name within the registry, e.g. `myorg/stellar-snapshot` |
| `spec.ociSnapshot.pull` | `boolean` | no | `false` | — | Enable pulling a snapshot to bootstrap a new node's PVC (default: false) |
| `spec.ociSnapshot.pullImageRef` | `string` | no | — | nullable | Image reference to pull from (full `registry/image:tag` string). Required when `pull = true`; if omitted the operator constructs the reference from `registry`, `image`, and `tag_strategy`. |
| `spec.ociSnapshot.push` | `boolean` | no | `false` | — | Enable pushing snapshots to the registry (default: false) |
| `spec.ociSnapshot.registry` | `string` | yes | — | — | OCI registry host, e.g. `ghcr.io` or `registry-1.docker.io` |
| `spec.ociSnapshot.tagStrategy` | `string` | no | `latestLedger` | enum: `latestLedger`, `fixed` | Tag used when pushing/pulling the snapshot image. With `LatestLedger` the tag is `snapshot-<ledger_seq>`; with `Fixed` the literal `fixed_tag` value is used. |
| `spec.forensicSnapshot` | `object` | no | — | nullable | Forensic snapshot: set `metadata.annotations["stellar.org/request-forensic-snapshot"]="true"` to trigger a one-shot capture (PCAP, optional core dump) uploaded to S3. |
| `spec.forensicSnapshot.credentialsSecretRef` | `string` | no | — | nullable | Secret in the same namespace with `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` when not using IRSA/instance roles. |
| `spec.forensicSnapshot.enableShareProcessNamespace` | `boolean` | no | `false` | — | Set `shareProcessNamespace: true` on validator pods so the capture container can see `stellar-core` for core dumps (recommended for forensic workflows). |
| `spec.forensicSnapshot.kmsKeyId` | `string` | no | — | nullable | Optional KMS key id for SSE-KMS (`aws s3 cp --sse aws:kms`). |
| `spec.forensicSnapshot.s3Bucket` | `string` | yes | — | — | Target S3 bucket for the encrypted forensic tarball. |
| `spec.forensicSnapshot.s3Prefix` | `string` | no | — | nullable | — |
| `spec.cveHandling` | `object` | no | — | nullable | CVE handling configuration for automated patching Enables scanning for vulnerabilities and automatic rollout of patched versions |
| `spec.cveHandling.canaryPassRateThreshold` | `number (double)` | no | `100.0` | — | — |
| `spec.cveHandling.canaryTestTimeoutSecs` | `integer (uint64)` | no | `300` | min `0.0` | — |
| `spec.cveHandling.consensusHealthThreshold` | `number (double)` | no | `0.95` | — | — |
| `spec.cveHandling.criticalOnly` | `boolean` | no | `false` | — | — |
| `spec.cveHandling.enableAutoRollback` | `boolean` | no | `true` | — | — |
| `spec.cveHandling.enabled` | `boolean` | no | `true` | — | — |
| `spec.cveHandling.scanIntervalSecs` | `integer (uint64)` | no | `3600` | min `0.0` | — |
| `spec.serviceMesh` | `object` | no | — | nullable | Service mesh configuration (Istio/Linkerd) for mTLS and advanced traffic control |
| `spec.serviceMesh.istio` | `object` | no | — | nullable | Istio-specific configuration |
| `spec.serviceMesh.istio.circuitBreaker` | `object` | no | — | nullable | Circuit breaker configuration for outlier detection |
| `spec.serviceMesh.istio.circuitBreaker.consecutiveErrors` | `integer (uint32)` | no | `5` | min `0.0` | Number of consecutive errors before opening circuit |
| `spec.serviceMesh.istio.circuitBreaker.minRequestVolume` | `integer (uint32)` | no | `10` | min `0.0` | Minimum request volume before applying circuit breaking |
| `spec.serviceMesh.istio.circuitBreaker.timeWindowSecs` | `integer (uint32)` | no | `30` | min `0.0` | Time window in seconds for counting errors |
| `spec.serviceMesh.istio.mtlsMode` | `string` | no | `STRICT` | enum: `STRICT`, `PERMISSIVE` | mTLS mode (STRICT or PERMISSIVE) |
| `spec.serviceMesh.istio.retries` | `object` | no | — | nullable | Retry policy for failed requests |
| `spec.serviceMesh.istio.retries.backoffMs` | `integer (uint32)` | no | `25` | min `0.0` | Backoff duration in milliseconds |
| `spec.serviceMesh.istio.retries.maxRetries` | `integer (uint32)` | no | `3` | min `0.0` | Maximum number of retries |
| `spec.serviceMesh.istio.retries.retryableStatusCodes` | `array of integer (uint32)` | no | `[]` | — | Retryable status codes (e.g., 503, 504) |
| `spec.serviceMesh.istio.timeoutSecs` | `integer (uint32)` | no | `30` | min `0.0` | VirtualService timeout in seconds |
| `spec.serviceMesh.linkerd` | `object` | no | — | nullable | Linkerd-specific configuration |
| `spec.serviceMesh.linkerd.autoMtls` | `boolean` | no | `true` | — | Enable automatic mTLS |
| `spec.serviceMesh.linkerd.policyMode` | `string` | no | `allow` | — | Policy mode (deny, audit, allow) |
| `spec.serviceMesh.sidecarInjection` | `boolean` | no | `true` | — | Enable sidecar injection for this node |


---

## 5. Validator (`nodeType: Validator`)

Use `spec.nodeType: Validator` plus `spec.validatorConfig` for Stellar Core
consensus nodes.

### 5.1 Validator-specific schema

`spec.validatorConfig` is optional and nullable at the spec root. None of its
own keys are required by OpenAPI when the object is present. Production
deployments still need a seed via `seedSecretRef` or `seedSecretSource`.

See the `spec.validatorConfig.*` rows in [section 4](#4-stellarnode-spec-catalog).

Notable published constraints:

| Field | Published constraint |
|---|---|
| `keySource` | enum `secret`, `kMS` (schemars camelCase of `Secret` / `KMS`); default `secret` |
| `enableHistoryArchive` | boolean, default `false` |
| `catchupComplete` | boolean, default `false` |
| `seedSecretRef` | string, default `""`; Secret key is `STELLAR_CORE_SEED` |
| `hsmConfig` | requires `pkcs11LibPath` and `provider` (`AWS` \| `Azure`) when set |
| `kmsConfig` | requires `keyId` and `provider` when set |
| `seedSecretSource.csiRef` | requires `secretProviderClassName` |
| `seedSecretSource.externalRef` | requires `name`, `remoteKey`, `secretStoreRef` |
| `seedSecretSource.localRef` | requires `name` |
| `seedSecretSource.vaultRef` | requires `role`, `secretPath` |

`seedSecretSource` takes precedence over `seedSecretRef` when both are present
(schema description). `seedSecretRef` is marked deprecated in favor of
`seedSecretSource` for production.

Validator-oriented optional spec objects that appear on the published schema:

- `spec.snapshotSchedule` — CSI VolumeSnapshots of the data PVC
- `spec.restoreFromSnapshot` — bootstrap PVC from an existing VolumeSnapshot
- `spec.storage.snapshotRef` — VolumeSnapshot or compressed archive bootstrap
- `spec.forensicSnapshot` — S3 forensic capture (requires `s3Bucket` when set)
- `spec.ociSnapshot` — OCI registry snapshot push/pull

### 5.2 Validator lifecycle notes

`status.ledgerSequence`, `status.ledgerUpdatedAt`, `status.quorumFragility`,
and `status.quorumAnalysisTimestamp` are described by the schema as
validator-oriented status. `status.phase` may include `Syncing` while Core
catches up from history archives.

---

## 6. Horizon (`nodeType: Horizon`)

Use `spec.nodeType: Horizon` plus `spec.horizonConfig`. Horizon is not a
separate Kubernetes kind.

### 6.1 Horizon-specific schema

When `spec.horizonConfig` is set, OpenAPI requires:

| Field | Type | Purpose |
|---|---|---|
| `databaseSecretRef` | string | Kubernetes Secret holding the Horizon database URL |
| `stellarCoreUrl` | string | HTTP URL of a stellar-core instance (typically port `11626`) |

Optional Horizon keys and published defaults:

| Field | Type | Default | Constraints |
|---|---|---|---|
| `enableIngest` | boolean | `true` | — |
| `ingestWorkers` | integer (uint32) | `1` | min `0` |
| `autoMigration` | boolean | `true` | — |
| `enableExperimentalIngestion` | boolean | `false` | — |

Horizon deployments commonly also set:

- `spec.replicas` (default `1`) for HA
- `spec.ingress` (requires `hosts`) for external HTTP/HTTPS
- `spec.autoscaling` (requires `minReplicas`, `maxReplicas`)
- `spec.managedDatabase` or `spec.database` for Postgres
- `spec.strategy.type: rollingUpdate` (schema default) or `canary`

See all `spec.horizonConfig.*` rows in [section 4](#4-stellarnode-spec-catalog).

### 6.2 Horizon lifecycle notes

`status.lastMigratedVersion` reports the database schema version after a
successful migration (`horizonConfig.autoMigration` default is `true`).
`status.readyReplicas` / `status.replicas` track Deployment-style replica
counts. Canary status fields apply when `spec.strategy.type` is `canary`.

---

## 7. Soroban RPC (`nodeType: SorobanRpc`)

Use `spec.nodeType: SorobanRpc` plus `spec.sorobanConfig`. Soroban RPC is not
a separate Kubernetes kind.

### 7.1 Soroban-specific schema

When `spec.sorobanConfig` is set, OpenAPI requires:

| Field | Type | Purpose |
|---|---|---|
| `stellarCoreUrl` | string | Upstream stellar-core HTTP URL for submission / captive core |

Optional Soroban keys and published defaults:

| Field | Type | Default | Constraints |
|---|---|---|---|
| `enablePreflight` | boolean | `true` | — |
| `maxEventsPerRequest` | integer (uint32) | `10000` | min `0` |
| `captiveCoreConfig` | string | — | nullable; unstructured / legacy |
| `captiveCoreStructuredConfig` | object | — | nullable; preferred typed captive-core config |
| `cache` | object | — | nullable fail-open read cache |

`captiveCoreStructuredConfig` fields (all optional):

| Field | Type | Default | Constraints |
|---|---|---|---|
| `historyArchiveUrls` | array of string | `[]` | — |
| `logLevel` | string | — | nullable |
| `networkPassphrase` | string | — | nullable |
| `httpPort` | integer (uint16) | — | nullable, min `0` |
| `peerPort` | integer (uint16) | — | nullable, min `0` |
| `additionalConfig` | string | — | nullable |

`cache` fields:

| Field | Type | Default | Constraints |
|---|---|---|---|
| `enabled` | boolean | `false` | — |
| `ttlSecs` | integer (int64) | `30` | min `1` |
| `maxEntries` | integer (int64) | `10000` | min `1`, max `10000` |
| `maxBytes` | integer (int64) | `67108864` | min `1`, max `67108864` |
| `image` | string | — | nullable |

See all `spec.sorobanConfig.*` rows in [section 4](#4-stellarnode-spec-catalog).

### 7.2 Soroban lifecycle notes

Soroban RPC typically reaches `Ready` after captive core has ingested enough
history to serve preflight and event queries. Use `spec.autoscaling` for
replica scaling; `minReplicas` and `maxReplicas` are required when that object
is present.

---

## 8. Status fields

`status` is an operator-written subresource. Clients must not rely on setting
it in apply manifests. When the object is present, OpenAPI requires
`status.phase`.


| Path | Type | Required | Default | Constraints | Purpose |
|---|---|---|---|---|---|
| `status` | `object` | yes | — | nullable | Status subresource for StellarNode  Reports the current state of the managed Stellar node using Kubernetes conventions. The operator continuously updates this status as the node progresses through its lifecycle.  # Node Phases  - `Pending` - Resource creation is queued but not started - `Creating` - Infrastructure (Pod, Service, etc.) is being created - `Running` - Pod is running but not yet synced - `Syncing` - Node is syncing blockchain data (validators) - `Ready` - Node is fully synced and operational - `Failed` - Node encountered an unrecoverable error - `Degraded` - Node is running but not fully healthy - `Remediating` - Operator is attempting to recover the node - `Terminating` - Node resources are being cleaned up |
| `status.bgpStatus` | `object` | no | — | nullable | BGP advertisement status (when using BGP mode) |
| `status.bgpStatus.activePeers` | `integer (int32)` | yes | — | — | Number of active BGP peers |
| `status.bgpStatus.advertisedPrefixes` | `array of string` | no | — | — | Advertised IP prefixes |
| `status.bgpStatus.lastUpdate` | `string` | no | — | nullable | Last BGP update time |
| `status.bgpStatus.sessionsEstablished` | `boolean` | yes | — | — | Whether BGP sessions are established |
| `status.canaryReadyReplicas` | `integer (int32)` | no | `0` | — | Current number of ready canary replicas (for canary deployments) |
| `status.canaryStartTime` | `string` | no | — | nullable | Timestamp when the canary was created (RFC3339) |
| `status.canaryVersion` | `string` | no | — | nullable | Version deployed in the canary deployment (if active) |
| `status.conditions` | `array of object` | no | — | — | Readiness conditions following Kubernetes conventions  Standard conditions include: - Ready: True when all sub-resources are healthy and the node is operational - Progressing: True when the node is being created, updated, or syncing - Degraded: True when the node is operational but experiencing issues |
| `status.conditions.[]` | `object` | no | — | — | Condition for status reporting |
| `status.conditions.[].lastTransitionTime` | `string` | yes | — | — | — |
| `status.conditions.[].message` | `string` | yes | — | — | — |
| `status.conditions.[].observedGeneration` | `integer (int64)` | no | — | nullable | — |
| `status.conditions.[].reason` | `string` | yes | — | — | — |
| `status.conditions.[].status` | `string` | yes | — | — | — |
| `status.conditions.[].type` | `string` | yes | — | — | — |
| `status.drStatus` | `object` | no | — | nullable | Status of the cross-region disaster recovery setup (if enabled) |
| `status.drStatus.currentRole` | `string` | no | — | enum: `primary`, `standby`; nullable | Role of a node in a DR configuration |
| `status.drStatus.failoverActive` | `boolean` | yes | — | — | — |
| `status.drStatus.lastDrillResult` | `object` | no | — | nullable | Result of a DR drill execution |
| `status.drStatus.lastDrillResult.applicationAvailability` | `boolean` | yes | — | — | Whether application remained available during drill |
| `status.drStatus.lastDrillResult.completedAt` | `string` | no | — | nullable | Timestamp when drill completed |
| `status.drStatus.lastDrillResult.message` | `string` | yes | — | — | Human-readable message about drill result |
| `status.drStatus.lastDrillResult.standbyTakeoverSuccess` | `boolean` | yes | — | — | Whether standby successfully took over |
| `status.drStatus.lastDrillResult.startedAt` | `string` | yes | — | — | Timestamp when drill started |
| `status.drStatus.lastDrillResult.status` | `string` | yes | — | enum: `pending`, `running`, `success`, `failed`, `rolledback` | Drill execution status |
| `status.drStatus.lastDrillResult.timeToRecoveryMs` | `integer (uint64)` | no | — | nullable; min `0.0` | Time to recovery in milliseconds |
| `status.drStatus.lastDrillTime` | `string` | no | — | nullable | — |
| `status.drStatus.lastPeerContact` | `string` | no | — | nullable | — |
| `status.drStatus.peerHealth` | `string` | no | — | nullable | — |
| `status.drStatus.syncLag` | `integer (uint64)` | no | — | nullable; min `0.0` | — |
| `status.endpoint` | `string` | no | — | nullable | Endpoint where the node is accessible (Service ClusterIP or external) |
| `status.externalIp` | `string` | no | — | nullable | External load balancer IP assigned by MetalLB |
| `status.forensicSnapshotPhase` | `string` | no | — | nullable | Phase of the last forensic snapshot request (`Pending`, `Capturing`, `Complete`, `Failed`). |
| `status.lastMigratedVersion` | `string` | no | — | nullable | Version of the database schema after last successful migration |
| `status.ledgerSequence` | `integer (uint64)` | no | — | nullable; min `0.0` | For validators: current ledger sequence number |
| `status.ledgerUpdatedAt` | `string` | no | — | nullable | Timestamp of the last ledger update (RFC3339) |
| `status.message` | `string` | no | — | nullable | Human-readable message about current state |
| `status.observedGeneration` | `integer (int64)` | no | — | nullable | Observed generation for status sync detection |
| `status.phase` | `string` | yes | — | — | Current phase of the node lifecycle (Pending, Creating, Running, Syncing, Ready, Failed, Degraded, Remediating, Terminating)  DEPRECATED: Use the conditions array instead. This field is maintained for backward compatibility and will be removed in a future version. The phase is now derived from the conditions. |
| `status.quorumAnalysisTimestamp` | `string` | no | — | nullable | Timestamp of last quorum analysis (RFC3339) |
| `status.quorumFragility` | `number (double)` | no | — | nullable | Quorum fragility score (0.0 = resilient, 1.0 = fragile) Only populated for validator nodes |
| `status.readyReplicas` | `integer (int32)` | no | `0` | — | Current number of ready replicas |
| `status.replicas` | `integer (int32)` | no | `0` | — | Total number of desired replicas |
| `status.vaultObservedSecretVersion` | `string` | no | — | nullable | Last observed Vault secret version annotation (for rotation-driven rollouts). |
| `status.labelPropagationStatus` | `string` | no | — | nullable | Result of the last label propagation pass. One of "Synced", "Partial", "Failed" |
| `status.snapshotBootstrap` | `object` | no | — | nullable | Bootstrap status when the node was started from a snapshot or compressed backup. Tracks the restore phase and time-to-sync for observability. A secondsToSync value ≤ 600 satisfies the "synced within 10 minutes" acceptance criterion. |
| `status.snapshotBootstrap.phase` | `string` | yes | — | — | Current phase of the bootstrap operation. One of: Pending, Restoring, Restored, Syncing, Synced, Failed |
| `status.snapshotBootstrap.source` | `string` | no | — | nullable | Source used for bootstrap (VolumeSnapshot name or backup URL). |
| `status.snapshotBootstrap.restoreStartedAt` | `string` | no | — | nullable | RFC3339 timestamp when the restore init container started. |
| `status.snapshotBootstrap.restoreCompletedAt` | `string` | no | — | nullable | RFC3339 timestamp when the restore init container completed successfully. |
| `status.snapshotBootstrap.syncedAt` | `string` | no | — | nullable | RFC3339 timestamp when the node first reached Synced state after bootstrap. |
| `status.snapshotBootstrap.secondsToSync` | `integer (uint64)` | no | — | nullable; min `0.0` | Elapsed seconds from restore completion to first Synced state. A value ≤ 600 satisfies the "synced within 10 minutes" acceptance criterion. |
| `status.snapshotBootstrap.message` | `string` | no | — | nullable | Human-readable message about the current bootstrap state. |


### 8.1 Condition object

Each `status.conditions[]` entry requires `type`, `status`, `reason`,
`message`, and `lastTransitionTime`. `observedGeneration` is optional and
nullable.

The published schema describes these **standard condition types**:

| Type | Meaning in the published schema |
|---|---|
| `Ready` | `True` when all sub-resources are healthy and the node is operational |
| `Progressing` | `True` when the node is being created, updated, or syncing |
| `Degraded` | `True` when the node is operational but experiencing issues |

Condition `status` values follow Kubernetes conventions used by the operator
implementation: `True`, `False`, `Unknown`.

`lastTransitionTime` is updated when the condition **status** changes.

### 8.2 Phase and lifecycle

`status.phase` is required. The schema lists these lifecycle values:

`Pending`, `Creating`, `Running`, `Syncing`, `Ready`, `Failed`, `Degraded`,
`Remediating`, `Terminating`.

The same schema marks `phase` **deprecated**: prefer `status.conditions`. The
operator still writes `phase` for compatibility and can derive it from
conditions:

| Observed conditions / reasons | Derived phase |
|---|---|
| `Ready=True` and `readyReplicas >= replicas` | `Ready` |
| `Degraded=True` | `Degraded` |
| `Progressing=True` | `Progressing` |
| `Ready=False` reason `PodsPending` | `Pending` |
| `Ready=False` reason `Creating` | `Creating` |
| `Ready=False` (other reason) | `NotReady` |
| no `Ready` condition | `Pending` |

Typical apply-to-ready path represented by those phases:

```
Pending → Creating → Running / Syncing → Ready
                                      ↘ Failed | Degraded | Remediating
Ready → Terminating (on delete)
```

Horizon and SorobanRpc follow the same condition types. Horizon additionally
surfaces `lastMigratedVersion` after schema migration. Validators additionally
surface ledger and quorum fields while syncing.

### 8.3 Other published status enums

| Field | Published values |
|---|---|
| `status.forensicSnapshotPhase` | `Pending`, `Capturing`, `Complete`, `Failed` |
| `status.labelPropagationStatus` | `Synced`, `Partial`, `Failed` |
| `status.snapshotBootstrap.phase` | `Pending`, `Restoring`, `Restored`, `Syncing`, `Synced`, `Failed` |
| `status.drStatus.currentRole` | `primary`, `standby` |
| `status.drStatus.lastDrillResult.status` | `pending`, `running`, `success`, `failed`, `rolledback` |

`status.snapshotBootstrap.secondsToSync` is a uint64 (`minimum: 0`). The schema
states that a value `≤ 600` meets the "synced within 10 minutes" criterion.

`status.drStatus` requires `failoverActive` when the object is present.
`status.bgpStatus` requires `activePeers` and `sessionsEstablished`.
`status.snapshotBootstrap` requires `phase`.
`status.drStatus.lastDrillResult` requires `applicationAvailability`,
`message`, `standbyTakeoverSuccess`, `startedAt`, and `status`.

---

## 9. Shared operational objects

These optional spec objects apply to more than one node type. Only fields from
the published schema are listed; see [section 4](#4-stellarnode-spec-catalog)
for nested defaults.

| Object | Typical node types | Required keys when set |
|---|---|---|
| `resources` | all | `limits`, `requests` (each requires `cpu`, `memory`) |
| `storage` | all | `size`, `storageClass` |
| `autoscaling` | Horizon, SorobanRpc | `minReplicas`, `maxReplicas` |
| `vpaConfig` | all | — (`updateMode` default `Initial`) |
| `ingress` | Horizon, SorobanRpc | `hosts` |
| `loadBalancer` | all | — (`mode` default `L2`) |
| `networkPolicy` | all | — (`enabled` default `false`) |
| `serviceMesh` | all | — |
| `crossCluster` | all | — |
| `globalDiscovery` | all | — |
| `drConfig` | all | `peerClusterId`, `role` |
| `managedDatabase` | Horizon | `storage` |
| `database` | Horizon | `secretKeyRef` |
| `dbMaintenanceConfig` | Horizon | `windowDuration`, `windowStart` |
| `readReplicaConfig` | Horizon | — |
| `cveHandling` | all | — |
| `strategy` | Horizon, SorobanRpc | `type` (default object `{type: rollingUpdate}`) |

`spec.strategy.type` enum is `rollingUpdate` \| `canary`.

`spec.podAntiAffinity` enum is `Hard` \| `Soft` \| `Disabled` (default `Hard`).

`spec.historyMode` enum is `Full` \| `Recent` (default `Recent`).

`spec.storage.mode` enum is `PersistentVolume` \| `Local` (default
`PersistentVolume`). `spec.storage.retentionPolicy` enum is `Delete` \|
`Retain` (default `Delete`). The default storage object is
`{mode: PersistentVolume, retentionPolicy: Delete, size: 100Gi, storageClass: standard}`.

Default compute resources are requests `500m` / `1Gi` and limits `2` / `4Gi`.

---

## 10. Accuracy and validation

Source of truth for this manual:

1. Published CRD: `config/crd/stellarnode-crd.yaml`
2. OpenAPI under `spec.versions[name=v1alpha1].schema.openAPIV3Schema`
3. Status condition helpers in `src/controller/conditions.rs` (condition types
   `Ready`, `Progressing`, `Degraded` and statuses `True` / `False` / `Unknown`)
4. Phase derivation in `StellarNodeStatus::derive_phase_from_conditions`

Fields that exist only on Rust structs and are **not** present on the published
OpenAPI schema are intentionally omitted.

Re-check this document whenever `config/crd/stellarnode-crd.yaml` changes.
