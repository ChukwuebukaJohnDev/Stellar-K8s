# Production security hardening

This guide is the deployable baseline for a production Stellar-K8s installation. It covers the operator ServiceAccount, validator seed delivery, and ingress boundaries for Horizon and Soroban RPC. The manifests in [`examples/security/`](../../examples/security/) use `stellar-mainnet` for the node namespace and `stellar-system` for the operator namespace.

## 1. Least-privilege operator RBAC

Set `operator.watchNamespace` to the one namespace the release manages. Do not leave it empty on a shared cluster:

```yaml
operator:
  watchNamespace: stellar-mainnet
```

Apply [`operator-rbac.yaml`](../../examples/security/operator-rbac.yaml) after creating both namespaces. The main Role is namespaced and grants only the resources the reconciler creates, updates, watches, and deletes. The separate `stellar-operator-namespace-reader` ClusterRole is required only for the network-label safety check and has `get,list` on `namespaces`; it cannot mutate namespaces or any cluster resource.

The operator needs `secrets` read access to resolve legacy `seedSecretRef`/`localRef` and write access only because it maintains generated TLS and related resources. Production validators should use `externalRef`, `vaultRef`, or `csiRef`; remove the `secrets` create/update/delete verbs if those features are disabled and no operator-managed TLS is enabled. Do not add `cluster-admin`, `*` resources, or `*` verbs. Optional controllers may require additional, separately reviewed namespaced rules for `horizontalpodautoscalers`, `servicemonitors`, `certificates`, CNPG resources, or Istio resources; grant only the API group/resource used by the enabled feature.

Verify the effective permission set:

```bash
kubectl auth can-i --as=system:serviceaccount:stellar-system:stellar-operator \
  --list -n stellar-mainnet
kubectl auth can-i --as=system:serviceaccount:stellar-system:stellar-operator \
  get secrets -n stellar-testnet  # must be no
kubectl auth can-i --as=system:serviceaccount:stellar-system:stellar-operator \
  patch namespaces  # must be no
```

## 2. Protect `seedSecretRef`

The legacy `validatorConfig.seedSecretRef` and `seedSecretSource.localRef` read a Kubernetes Secret in the node namespace. Base64 is not encryption; use them only for development. For production, configure the CRD's `externalRef` and let External Secrets Operator (ESO) materialize the target Secret in the same namespace:

```yaml
validatorConfig:
  seedSecretSource:
    externalRef:
      name: validator-seed
      secretStoreRef:
        name: aws-secrets-manager # or vault-secrets
        kind: SecretStore
      remoteKey: prod/stellar/mainnet/validator-01
      remoteProperty: seed
      refreshInterval: 15m
```

Apply one of [`externalsecret-aws.yaml`](../../examples/security/externalsecret-aws.yaml) or [`externalsecret-vault.yaml`](../../examples/security/externalsecret-vault.yaml), then configure the remote policy to read only that path. The operator creates the `ExternalSecret`; do not commit a seed or a generated Kubernetes Secret. For AWS, replace the example role ARN and constrain IAM to `secretsmanager:GetSecretValue` on the exact secret ARN. For Vault, bind the Kubernetes auth role to the `external-secrets` ServiceAccount in `stellar-mainnet` and grant only `read` on the exact KV v2 `data/...` path. Keep the SecretStore namespaced unless a centrally managed ClusterSecretStore is required.

ESO must be allowed to reach the backend, and the resulting Secret must be readable only by the validator ServiceAccount. Rotation is not complete until the generated Secret is updated and the validator is restarted according to the configured rotation procedure. Audit ESO status and Kubernetes Secret access; never log Secret data.

## 3. Network boundaries

Label only approved gateway/client namespaces:

```bash
kubectl label namespace api-gateway stellar.org/network-access=trusted
kubectl label namespace stellar-mainnet stellar.org/network=mainnet
```

Apply [`network-policy.yaml`](../../examples/security/network-policy.yaml). The first policy selects all operator-generated Horizon (`app.kubernetes.io/component=horizon`) and Soroban RPC (`sorobanrpc`) pods and enters ingress isolation. The second is the only ingress allow-list: TCP 8000 from a pod labeled `app.kubernetes.io/part-of=stellar-api-gateway` or `stellar.org/client=horizon` in a namespace labeled `stellar.org/network-access=trusted`. Validator peer port 11625 and admin endpoints are not allowed by these policies. NetworkPolicy rules are additive, so do not deploy another policy that allows broad ingress.

This requires a CNI that enforces NetworkPolicy (for example Cilium or Calico). Validate enforcement, not just object creation:

```bash
kubectl apply -f examples/security/network-policy.yaml
kubectl run deny-test -n untrusted --image=curlimages/curl:8.10.1 \
  --restart=Never --rm -i -- sh -c \
  'curl --connect-timeout 3 -sS http://horizon-mainnet.stellar-mainnet.svc.cluster.local:8000'
# Expected: timeout or connection refused.

kubectl run allow-test -n api-gateway --image=curlimages/curl:8.10.1 \
  --labels=app.kubernetes.io/part-of=stellar-api-gateway \
  --restart=Never --rm -i -- \
  curl --connect-timeout 3 -fsS http://horizon-mainnet.stellar-mainnet.svc.cluster.local:8000
# Expected: an HTTP response from Horizon.
```

Run the tests from namespaces that actually carry the labels shown above, and test both Horizon and Soroban Services. If the untrusted request succeeds, stop the rollout: the CNI is not enforcing the policy, the destination Service selects non-target pods, or another NetworkPolicy is allowing traffic.
