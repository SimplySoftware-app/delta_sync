# DeltaSync - Authentic Content Protocol

DeltaSync is open-source media provenance built on C2PA. A Rust signing authority (c2pa-rs) issues short-lived device certificates after App Attest or Play Integrity checks, and runs in stub mode locally without those checks. The iOS and Android capture apps sign JPEGs on the device, and anyone can verify a file with the server's `/verify` endpoint.

## Features

- **Device Capture Signing**: The iOS and Android apps sign captured JPEGs with a key held in secure hardware
- **Content Hash in the Manifest**: The C2PA manifest records a hash of the signed asset, so a changed file fails verification
- **Public Verification**: `POST /verify` reads a C2PA manifest and reports whether it was issued by the DeltaSync CA

## Core Principles

1. **Small Load** - Minimal processing overhead
2. **Encapsulation** - Permanent cryptographic linking
3. **Generalized** - Cross-platform implementation
4. **Master's Approval** - Governed partnership model
5. **Categorization** - Clear content type definitions

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

Production is the Next.js landing on the Hetzner host, at [deltasync.simplysoftware.app](https://deltasync.simplysoftware.app). A push to `main` runs `.github/workflows/deploy.yml`: it syncs this repo to `/opt/apps/delta_sync`, builds the landing container on the host (it publishes `127.0.0.1:3600` only, which the host-networked `edge-caddy` proxies to), installs `deploy/caddy/deltasync.simplysoftware.app.caddy` into `/opt/apps/edge/sites`, reloads `edge-caddy`, and checks that the landing answers on `127.0.0.1:3600`.

`vercel.json` and `.vercelignore` remain from the previous Vercel host. They are not how this landing is served. The old Vercel and GoDaddy notes are in `DEPLOYMENT.md` and `DOMAIN_SETUP.md`.

## Tech Stack

- **Framework**: Next.js 16
- **Language**: TypeScript
- **Styling**: Tailwind CSS 4
- **Icons**: Lucide React
- **Deployment**: Docker on Hetzner, behind edge-caddy
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
