# Kubernetes Example

`example.yaml` runs hbbs, hbbr and rustdesk-api in one Recreate Deployment with a shared ReadWriteOnce PVC. It uses the source-built `runtime` target; the API binary is explicitly selected. Built Web assets and CA/runtime libraries are included.

1. Build `docker/Dockerfile --target runtime` from the repository root with recursive submodules and publish to your own registry. Replace all four image references with that exact tested version/digest.
2. Change the public API/ID/relay addresses and hbbs relay argument. Add the fixed OAuth callback and exact allowed origins when needed. Terminate HTTPS in a reverse proxy.
3. Create `rustdesk-api-secret` separately using a restricted file/secret manager. Required keys: independent random `API_JWT_SECRET` (at least 32 bytes), base64 32-byte `API_OAUTH_CONFIG_KEY`, and bootstrap administrator username/password for a new database. The sample contains no usable default secret. Retain the encryption key across restarts.
4. Ensure the volume supports SQLite locking and UID/GID 10001. The initContainer serializes initialization, verifies the keypair/configuration/Web assets and runs migrations before services start.

```bash
kubectl apply --dry-run=server -f kubernetes/example.yaml
kubectl apply -f kubernetes/example.yaml
kubectl rollout status deployment/rustdesk-server
kubectl get pods -l app=rustdesk
```

API readiness uses `/health/ready` (database and static resources), liveness uses `/health/live`. TCP probes check hbbs/hbbr listeners. Data and keys reside in `/data`; static assets reside in `/usr/share/rustdesk-api-web`.

This is a single-instance deployment. Do not scale replicas without redesigning SQLite/peer persistence and locking. Backup, migration preflight, matched-version rollback and disposable kind verification are documented in `docs/deployment.md`.
