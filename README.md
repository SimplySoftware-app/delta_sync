# DeltaSync

DeltaSync is open-source media provenance built on C2PA. A Rust signing authority (c2pa-rs) issues short-lived device certificates after App Attest or Play Integrity checks, and runs in stub mode locally without those checks. The iOS and Android capture apps sign JPEGs on the device, and anyone can verify a file with the server's `/verify` endpoint.

## Features

- **Device Capture Signing**: The iOS and Android apps sign captured JPEGs with a device-held key (Secure Enclave on iOS; StrongBox or hardware keystore on Android when available, otherwise a software keystore)
- **Content Hash in the Manifest**: The C2PA manifest records a hash of the signed asset, so a changed file fails verification
- **Public Verification**: `POST /verify` reads a C2PA manifest and reports whether it was issued by the DeltaSync CA

## Getting Started

### Prerequisites

- Node.js 18+ 
- npm or yarn

### Installation

```bash
npm install
```

### Development

```bash
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) to view the site.

### Build

```bash
npm run build
```

### Production

```bash
npm start
```

## Deployment

Production is the Next.js landing on the Hetzner host, at [deltasync.simplysoftware.app](https://deltasync.simplysoftware.app). The server never builds it. A push to `main` runs `.github/workflows/deploy.yml`:

1. The `image` job builds the root `Dockerfile` and pushes `ghcr.io/simplysoftware-app/delta_sync:<commit sha>` to GHCR.
2. The `deploy` job (one at a time, `concurrency: deploy-delta-sync`) rsyncs only `compose.prod.yaml` and `deploy/` to `/opt/apps/delta_sync`, logs the server in to GHCR with the job token, and runs `deploy/remote-deploy.sh` with `IMAGE_TAG`. That script pulls the image, recreates the container (it publishes `127.0.0.1:3600` only, which the host-networked `edge-caddy` proxies to), checks that the landing answers, installs `deploy/caddy/deltasync.simplysoftware.app.caddy` into `/opt/apps/edge/sites` and reloads `edge-caddy`.

Secrets in the `production` environment: `DEPLOY_HOST`, `DEPLOY_USER`, `DEPLOY_SSH_KEY`, and optionally `DEPLOY_SSH_PORT` (22 when unset).

The landing is not a static export. The root layout reads the `locale` cookie and the `Accept-Language` header on each request (`lib/get-locale.ts`), so every route is dynamic and needs the Node server. A static export would have to move the language choice to the browser (a flash of English for Spanish visitors, English-only titles) or to `/en` and `/es` URLs. The image is Next's `standalone` output on `node:22-alpine`. `compress: false` in `next.config.ts` leaves compression to `edge-caddy` (`encode zstd gzip`), so Next does not gzip first.

To run the production image on a laptop, build it with `docker compose up -d --build` (`docker-compose.yml`, `127.0.0.1:3600`).

`vercel.json` and `.vercelignore` remain from the previous Vercel host. They are not how this landing is served. The old Vercel and GoDaddy notes are in `DEPLOYMENT.md` and `DOMAIN_SETUP.md`.

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

The site above is unchanged. Device capture lives in three new folders:

- `server/` is the Rust signing authority and C2PA `/verify` service.
- `ios/` is the SwiftUI camera app.
- `android/` is the Compose camera app.

The trust chain, what is real, and what is stubbed are in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
