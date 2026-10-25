# DeltaSync

DeltaSync is open-source media provenance built on C2PA. A Rust signing authority (c2pa-rs) issues short-lived device certificates after App Attest or Play Integrity checks, and runs in stub mode locally without those checks. The iOS and Android capture apps sign JPEGs on the device, and anyone can verify a file with the server's `/verify` endpoint.

## Features

- **Device Capture Signing**: The iOS and Android apps sign captured JPEGs with a device-held key (Secure Enclave on iOS; StrongBox or hardware keystore on Android when available, otherwise a software keystore)
- **Content Hash in the Manifest**: The C2PA manifest records a hash of the signed asset, so a changed file fails verification
- **Public Verification**: `POST /verify` reads a C2PA manifest and reports whether it was issued by the DeltaSync CA

## Getting Started

### Prerequisites

- Node.js 20.9+ (Next.js 16 requires it; the image uses Node 22)
- Bun 1.4+ (package manager; `packageManager` in `package.json` pins the version)

### Installation

```bash
bun install
```

### Development

```bash
bun run dev
```

Open [http://localhost:3000](http://localhost:3000) to view the site.

### Build

```bash
bun run build
```

### Production

```bash
bun run start
```

## Deployment

Production is the Next.js landing on the Hetzner host, at [deltasync.simplysoftware.app](https://deltasync.simplysoftware.app). The server never builds it. A push to `main` runs `.github/workflows/deploy.yml`:

1. The `image` job builds the root `Dockerfile` and pushes `ghcr.io/simplysoftware-app/delta_sync:<commit sha>` to GHCR.
2. The `deploy` job (one at a time, `concurrency: deploy-delta-sync`) rsyncs only `compose.prod.yaml` and `deploy/` to `/opt/apps/delta_sync`, logs the server in to GHCR with the job token (the job needs `packages: read` to pull the private image), and runs `deploy/remote-deploy.sh` with `IMAGE_TAG`. That script rolls the landing over blue/green with no downtime: the landing exists as two containers (`delta-sync-landing-blue` on `127.0.0.1:3650`, `delta-sync-landing-green` on `127.0.0.1:3651`, loopback only). The script finds the live colour (`.active-color` in the deploy directory, else whichever is running), pulls the image and starts the idle colour, waits until Docker reports it `healthy`, installs `deploy/caddy/deltasync.simplysoftware.app.caddy` into `/opt/apps/edge/sites` and reloads the host-networked `edge-caddy` (which lists both ports, health-checks `/` every 2 s and sends traffic only to a healthy one, retrying a failed dial for up to 30 s), waits a few seconds, then stops the old colour with a 30 s drain. If the new colour never becomes healthy it is removed and the live one is untouched (the deploy fails). The image tags of both colours are kept in the server `.env` as `IMAGE_TAG_BLUE` and `IMAGE_TAG_GREEN`; manual `docker compose` commands need them. The first deploy after the move from the single `delta-sync-landing` container starts blue, switches the snippet and then removes the old container. Rollback: run Deploy on an older commit. The landing has no database, but if one is ever added, migrations must be backward compatible with the previous release (expand, deploy, contract), because the old and new colour run side by side for a few seconds. The Rust `server/` is not deployed by this workflow and is outside this change. A last step always runs `docker logout ghcr.io` on the server: the job token expires with the job, and a stale one left in the deploy user's Docker config makes every later `ghcr.io` pull on that host fail with "denied", public images included.

Secrets in the `production` environment: `DEPLOY_HOST`, `DEPLOY_USER`, `DEPLOY_SSH_KEY`, and optionally `DEPLOY_SSH_PORT` (22 when unset).

The landing is not a static export. The root layout reads the `locale` cookie and the `Accept-Language` header on each request (`lib/get-locale.ts`), so every route is dynamic and needs the Node server. A static export would have to move the language choice to the browser (a flash of English for Spanish visitors, English-only titles) or to `/en` and `/es` URLs. The image is Next's `standalone` output on `node:22-alpine`. `compress: false` in `next.config.ts` leaves compression to `edge-caddy` (`encode zstd gzip`), so Next does not gzip first.

To run the production image on a laptop, build it with `docker compose up -d --build` (`docker-compose.yml`, `127.0.0.1:3600`).

`vercel.json` and `.vercelignore` remain from the previous Vercel host. They are not how this landing is served.

## Tech Stack

- **Framework**: Next.js 16
- **Language**: TypeScript
- **Styling**: Tailwind CSS 4
- **Icons**: Lucide React
- **Deployment**: Docker image built in GitHub Actions (GHCR), pulled by the Hetzner host, behind edge-caddy
- **Domain**: deltasync.simplysoftware.app

## Project Structure

```
delta_sync/
├── app/
│   ├── layout.tsx      # Root layout with metadata
│   ├── page.tsx        # Landing page
│   └── globals.css     # Global styles
├── public/             # Static assets
├── package.json        # Dependencies
└── README.md          # This file
```

## License

Apache-2.0 — see LICENSE.

## Contact

For inquiries, visit [deltasync.simplysoftware.app](https://deltasync.simplysoftware.app)

## Capture v1

The site above is unchanged. Device capture lives in these folders:

- `server/` is the Rust signing authority and C2PA `/verify` service.
- `crates/core/` is the shared Rust signing and verification core (`delta_sync_core`), with `sdk/swift` and `sdk/kotlin` as its hardware-key bindings.
- `ios/` is the SwiftUI camera app.
- `android/` is the Compose camera app.
- `trust/` holds the pinned official C2PA trust lists used for external cameras ([docs/external-cameras.md](docs/external-cameras.md)); `.github/workflows/c2pa-trust-update.yml` refreshes them weekly.

The trust chain, what is real, and what is stubbed are in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
