#!/usr/bin/env bash
# Package the XCFramework for release (run after sdk/swift/scripts/build-xcframework.sh).
#
#   scripts/release/package-apple.sh <out-dir>
#
# Writes into <out-dir>:
#   DeltaSyncCoreFFI.xcframework.zip   binaryTarget payload (xcframework at the zip root)
#   DeltaSyncCoreFFI.xcframework.zip.checksum   output of `swift package compute-checksum`
#   delta_sync_core.swift              generated UniFFI bindings
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SWIFT="$ROOT/sdk/swift"
OUT="${1:?usage: package-apple.sh <out-dir>}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

[ -d "$SWIFT/DeltaSyncCoreFFI.xcframework" ] || { echo "missing xcframework; run sdk/swift/scripts/build-xcframework.sh" >&2; exit 1; }
GEN="$SWIFT/Sources/DeltaSyncCore/Generated/delta_sync_core.swift"
[ -f "$GEN" ] || { echo "missing generated bindings" >&2; exit 1; }

rm -f "$OUT/DeltaSyncCoreFFI.xcframework.zip"
(cd "$SWIFT" && ditto -c -k --sequesterRsrc --keepParent DeltaSyncCoreFFI.xcframework "$OUT/DeltaSyncCoreFFI.xcframework.zip")
cp "$GEN" "$OUT/delta_sync_core.swift"
(cd "$SWIFT" && swift package compute-checksum "$OUT/DeltaSyncCoreFFI.xcframework.zip") > "$OUT/DeltaSyncCoreFFI.xcframework.zip.checksum"
echo ":: xcframework checksum: $(cat "$OUT/DeltaSyncCoreFFI.xcframework.zip.checksum")"
