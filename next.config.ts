import type { NextConfig } from "next";

// Not a static export: the root layout reads the locale cookie and the
// Accept-Language header per request (lib/get-locale.ts), so every route is
// dynamic and the landing needs the Node server.
const nextConfig: NextConfig = {
  output: "standalone",
  // edge-caddy compresses (zstd, gzip). Leaving this on would make Next gzip
  // first, and Caddy passes an already-encoded response through untouched.
  compress: false,
  // The landing does not use next/image, and the standalone server does not load
  // next.config.ts at runtime (the config is inlined in server.js). Keep their
  // 50 MB of native and compiler files out of the traced output.
  outputFileTracingExcludes: {
    "*": ["node_modules/@img/**", "node_modules/sharp/**", "node_modules/typescript/**"],
  },
};

export default nextConfig;
