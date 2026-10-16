# DeltaSync server

HTTP service that issues short-lived device certificates and checks C2PA manifests. The Next.js site at the repo root is separate and is not served from here.

## Run locally

```bash
cd server
cp .env.example .env
# ATTESTATION_MODE=stub is the default for a laptop.
cargo run
```

The process listens on `0.0.0.0:8080`. A phone on the same network uses your machine's LAN address.

## Endpoints

- `GET /health`
- `POST /v1/challenge` with `{ "platform": "ios" | "android" }`
- `POST /v1/device-certificate` verifies the CSR and, in live mode, App Attest or Play Integrity, then returns a device certificate
- `POST /verify` multipart field `file` (JPEG, PNG, WebP, or MP4)

The device certificate is chained to the CA created in `DATA_DIR` (`ca.crt.pem`, `ca.key.pem`). Those files are not in git.

## Hetzner and Caddy

```bash
docker network create caddy   # once, shared with the Caddy container
cd server
cp .env.example .env          # set live values before a public host
docker compose up -d --build
```

Compose joins the external network named `caddy` (override with `CADDY_DOCKER_NETWORK`) and does not publish a host port. Caddy proxies to `http://delta-sync:8080`. See `deploy/Caddyfile.snippet`.

## Tests

```bash
cargo test --workspace   # from the repo root; `cargo test` inside server/ also works
```
