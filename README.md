# DeltaSync - Authentic Content Protocol

A revolutionary protocol ensuring video authenticity in the age of AI-generated content.

## Overview

DeltaSync is a cryptographic protocol that creates an unbreakable chain of authenticity from device to platform, ensuring every video can be verified as genuine. Built to combat the growing problem of AI-generated fake content flooding social media.

## Features

- **Device Capture Verification**: Videos are cryptographically signed at capture by trusted devices
- **Unique Fingerprinting**: Each video receives a permanent cryptographic hash
- **Platform Integration**: Seamless verification across social media platforms
- **Privacy Protected**: Zero-knowledge verification preserves user privacy
- **Blockchain-Based**: Unbreakable security through distributed ledger technology

## Core Principles

1. **Small Load** - Minimal processing overhead
2. **Confidentiality** - Zero-knowledge verification
3. **Encapsulation** - Permanent cryptographic linking
4. **Generalized** - Cross-platform implementation
5. **Master's Approval** - Governed partnership model
6. **Categorization** - Clear content type definitions
7. **Unbreakable** - Blockchain-secured protocol

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

Copyright © 2025 DeltaSync. All rights reserved.

## Contact

For inquiries, visit [deltasync.simplysoftware.app](https://deltasync.simplysoftware.app)

## Capture v1

The site above is unchanged. Device capture lives in three new folders:

- `server/` is the Rust signing authority and C2PA `/verify` service.
- `ios/` is the SwiftUI camera app.
- `android/` is the Compose camera app.

The trust chain, what is real, and what is stubbed are in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
