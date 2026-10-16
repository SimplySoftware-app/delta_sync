#!/usr/bin/env bash
# Build DeltaSyncCoreFFI.xcframework and the generated Swift bindings.
#
#   sdk/swift/scripts/build-xcframework.sh            # release, device + simulator (arm64)
#   PROFILE=debug sdk/swift/scripts/build-xcframework.sh
#   INCLUDE_X86_64_SIM=1 sdk/swift/scripts/build-xcframework.sh
#
# Needs: rustup targets aarch64-apple-ios, aarch64-apple-ios-sim
# (and x86_64-apple-ios for INCLUDE_X86_64_SIM), Xcode command line tools.
# Output (both git-ignored):
#   sdk/swift/DeltaSyncCoreFFI.xcframework
#   sdk/swift/Sources/DeltaSyncCore/Generated/delta_sync_core.swift
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
PROFILE="${PROFILE:-release}"
PROFILE_FLAG=()
[ "$PROFILE" = "release" ] && PROFILE_FLAG=(--release)
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

# Match the package's platform so the linker does not warn about newer objects.
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-15.0}"

TARGETS=(aarch64-apple-ios aarch64-apple-ios-sim)
[ "${INCLUDE_X86_64_SIM:-0}" = "1" ] && TARGETS+=(x86_64-apple-ios)

cd "$ROOT"
for target in "${TARGETS[@]}"; do
  echo ":: cargo build $target ($PROFILE)"
  cargo build -p delta_sync_core ${PROFILE_FLAG[@]+"${PROFILE_FLAG[@]}"} --target "$target" --locked
done

echo ":: generate Swift bindings"
cargo build -p delta_sync_core ${PROFILE_FLAG[@]+"${PROFILE_FLAG[@]}"} --locked
HOST_LIB="$TARGET_DIR/$PROFILE/libdelta_sync_core.dylib"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cargo run -q -p delta_sync_core --features bindgen --bin uniffi-bindgen ${PROFILE_FLAG[@]+"${PROFILE_FLAG[@]}"} --locked -- \
  generate --library "$HOST_LIB" --language swift --out-dir "$WORK/bindings"

mkdir -p "$HERE/Sources/DeltaSyncCore/Generated"
cp "$WORK/bindings/delta_sync_core.swift" "$HERE/Sources/DeltaSyncCore/Generated/delta_sync_core.swift"

mkdir -p "$WORK/Headers"
cp "$WORK/bindings/delta_sync_coreFFI.h" "$WORK/Headers/"
cp "$WORK/bindings/delta_sync_coreFFI.modulemap" "$WORK/Headers/module.modulemap"

echo ":: lipo simulator slice"
SIM_LIBS=("$TARGET_DIR/aarch64-apple-ios-sim/$PROFILE/libdelta_sync_core.a")
[ "${INCLUDE_X86_64_SIM:-0}" = "1" ] && SIM_LIBS+=("$TARGET_DIR/x86_64-apple-ios/$PROFILE/libdelta_sync_core.a")
mkdir -p "$WORK/sim"
lipo -create "${SIM_LIBS[@]}" -output "$WORK/sim/libdelta_sync_core.a"

echo ":: xcodebuild -create-xcframework"
rm -rf "$HERE/DeltaSyncCoreFFI.xcframework"
xcodebuild -create-xcframework \
  -library "$TARGET_DIR/aarch64-apple-ios/$PROFILE/libdelta_sync_core.a" -headers "$WORK/Headers" \
  -library "$WORK/sim/libdelta_sync_core.a" -headers "$WORK/Headers" \
  -output "$HERE/DeltaSyncCoreFFI.xcframework"

echo ":: done: $HERE/DeltaSyncCoreFFI.xcframework"
