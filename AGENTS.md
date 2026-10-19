# DeltaSync — Agent Instructions

DeltaSync is a C2PA media-provenance system: a Rust signing authority (`server/`), a shared Rust core (`crates/core`, with `sdk/swift` and `sdk/kotlin`), the iOS and Android capture apps, and a Next.js landing at the repo root that is deployed on its own.

Read before changing anything:

1. [README.md](README.md) — the landing, and how it reaches production (GHCR image, `deploy.yml`, edge-caddy).
2. [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — the trust chain, what is real versus stubbed, and the key decisions.

Known code flaws: [INCONSISTENCIES.md](INCONSISTENCIES.md) — delete entries you resolve.

## Reference (read when needed)

- [server/README.md](server/README.md) — PKI, endpoints, sealing, time-stamps, running behind Caddy. Read before touching the signing server.
- [docs/external-cameras.md](docs/external-cameras.md) — how third-party cameras are accepted through the pinned C2PA trust lists (Spanish). Read before touching `trust/` or `external_trust.rs`.
- [docs/RELEASES.md](docs/RELEASES.md) — release artifacts and how consumers pull them (Spanish).
- [ios/README.md](ios/README.md) and [android/README.md](android/README.md) — building the apps and the Apple and Google account setup. [sdk/swift](sdk/swift/README.md) and [sdk/kotlin](sdk/kotlin/README.md) — the hardware-key bindings and their tests.

## Non-negotiable rules

- **The landing server never builds** — CI builds the root `Dockerfile` and pushes to GHCR; the server only pulls. The image name in `.github/workflows/deploy.yml` must match `compose.prod.yaml` (the workflow greps for it).
- **Port 3600 on loopback is a three-way contract** — `compose.prod.yaml`, `deploy/remote-deploy.sh` and `deploy/caddy/deltasync.simplysoftware.app.caddy` must agree. Never publish it on a public interface.
- **Never leave a GHCR login on the server** — the deploy job always runs `docker logout ghcr.io`; keep that step and `packages: read` on the deploy job.
- **Compression belongs to Caddy** — `compress: false` in `next.config.ts` is deliberate.
- **The landing is dynamic, not a static export** — the root layout reads the locale per request.
- **Never run the signing server in stub mode on a public address** — `ATTESTATION_MODE=stub` does not check attestation.
- **No keys or credentials in the repo** — the only PEM under `server/certs/` is Apple's public App Attest root.
- **Trust lists under `trust/` are generated** — refresh them with `scripts/update-c2pa-trust.py` (the weekly workflow does it), never by hand.

## Local verification

- Rust (server, core): `cargo test --workspace --locked`, toolchain pinned in `rust-toolchain.toml`. This is what `ci.yml` runs.
- Landing: `npm run build`. CI does not build or lint it; the deploy image build is the first gate.
