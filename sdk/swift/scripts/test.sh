#!/usr/bin/env bash
# Build the xcframework, then run the Swift tests on an iOS simulator.
# The Secure Enclave is not available there, so the tests sign with a
# software P-256 key generated for this run.
#
#   sdk/swift/scripts/test.sh                 # picks an available iPhone simulator
#   DESTINATION='platform=iOS Simulator,name=iPhone 16' sdk/swift/scripts/test.sh
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"

PROFILE="${PROFILE:-debug}" "$HERE/scripts/build-xcframework.sh"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
(cd "$ROOT" && cargo run -q -p delta_sync_core --example test_identity --locked) > "$WORK/identity.json"

DEST="${DESTINATION:-}"
if [ -z "$DEST" ]; then
  NAME="$(xcrun simctl list devices available | grep -E '^\s+iPhone' | head -1 | sed -E 's/^\s+(.*) \([0-9A-F-]{36}\).*/\1/')"
  DEST="platform=iOS Simulator,name=$NAME"
fi

cd "$HERE"
TEST_RUNNER_DELTA_SYNC_TEST_IDENTITY="$WORK/identity.json" \
TEST_RUNNER_DELTA_SYNC_TEST_FIXTURES="$ROOT/crates/core/tests/fixtures" \
xcodebuild test -scheme DeltaSyncCore -destination "$DEST" -derivedDataPath "$WORK/dd"
