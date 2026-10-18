import type { NextConfig } from "next";

// Not a static export: the root layout reads the locale cookie and the
// Accept-Language header per request (lib/get-locale.ts), so every route is
// dynamic and the landing needs the Node server.
const nextConfig: NextConfig = {
  output: "standalone",
  // edge-caddy compresses (zstd, gzip). Leaving this on would make Next gzip
  // first, and Caddy passes an already-encoded response through untouched.
  compress: false,
};

export default nextConfig;
