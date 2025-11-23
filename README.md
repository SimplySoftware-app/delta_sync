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

### Vercel (Recommended)

1. Push your code to GitHub
2. Import project in Vercel
3. Configure custom domain (deltasync.cloud) in Vercel settings
4. Deploy

### GoDaddy Domain Configuration

1. Log in to your GoDaddy account
2. Go to DNS Management for deltasync.cloud
3. Add the following DNS records:

```
Type: CNAME
Name: www
Value: cname.vercel-dns.com
TTL: 600

Type: A
Name: @
Value: 76.76.21.21
TTL: 600
```

4. In Vercel, add deltasync.cloud as a custom domain
5. Vercel will automatically provision SSL certificate

## Tech Stack

- **Framework**: Next.js 16
- **Language**: TypeScript
- **Styling**: Tailwind CSS 4
- **Icons**: Lucide React
- **Deployment**: Vercel
- **Domain**: GoDaddy

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

For inquiries, visit [deltasync.cloud](https://deltasync.cloud)
