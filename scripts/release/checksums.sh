#!/usr/bin/env bash
# Write SHA256SUMS for every file in <dir> (except SHA256SUMS itself).
#   scripts/release/checksums.sh <dir>
set -euo pipefail
DIR="${1:?usage: checksums.sh <dir>}"
cd "$DIR"
if command -v sha256sum >/dev/null 2>&1; then SUM=(sha256sum); else SUM=(shasum -a 256); fi
find . -maxdepth 1 -type f ! -name SHA256SUMS -print | sed 's|^\./||' | LC_ALL=C sort | xargs "${SUM[@]}" > SHA256SUMS
cat SHA256SUMS
