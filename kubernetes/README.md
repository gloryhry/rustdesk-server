# Kubernetes Example

`example.yaml` runs `hbbs`, `hbbr`, and the optional `rustdesk-api` binary in one Recreate Deployment with a shared SQLite volume. The API is exposed on TCP port `21114`; RustDesk WebSocket ports `21118` and `21119` are also included in the Service.

Before applying the manifest:

1. Replace `rustdesk-api-secret`'s `API_JWT_SECRET` with a random value of at least 32 bytes. Do not commit a production secret.
2. Replace the relay hostname in the `hbbs` command with the public relay hostname.
3. Configure `RUSTDESK_ID_SERVER`, `RUSTDESK_RELAY_SERVER`, and `API_PUBLIC_URL` for the deployment. The example intentionally leaves those values unset rather than inventing public addresses.
4. Serve the built `web/dist` from a separate HTTPS origin and proxy `/api` to port `21114`, or mount the assets at `/root/web` and set `API_WEB_ROOT` accordingly.

The example uses a single ReadWriteOnce PVC and `Recreate`, so it is a single-instance reference deployment. It is not an HA configuration and should not be scaled without moving the API database and peer state to a storage design that supports the required locking and recovery behavior.

```bash
kubectl apply -f kubernetes/example.yaml
kubectl get pods -l app=rustdesk
kubectl get service rustdesk-service
```

The API readiness and liveness probes use `GET /health/live`. Public API exposure should be placed behind an HTTPS reverse proxy with an explicit origin policy.
