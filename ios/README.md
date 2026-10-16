# DeltaSync iOS

Minimal camera app for Xcode 26. It creates a P-256 key in the Secure Enclave, attests the app with App Attest, enrolls that signing key with the DeltaSync server, and embeds a C2PA manifest in the captured JPEG. The C2PA manifest is built by the Delta Sync Rust core (`crates/core`) through the local Swift package `sdk/swift` (UniFFI). The Secure Enclave key signs through `SecureEnclaveDeviceSigner`; the private key never reaches Rust.

Build the package's XCFramework once, then open the project:

```sh
sdk/swift/scripts/build-xcframework.sh   # from the repository root
```

The XCFramework has an arm64 simulator slice. Build for a concrete simulator (for example `-destination 'platform=iOS Simulator,name=iPhone 17'`), or run the script with `INCLUDE_X86_64_SIM=1` for the generic destination.

Open `DeltaSync.xcodeproj`. The bundle id is `cloud.deltasync.capture`. Change it to an App ID you control, and put the same Team ID and bundle id in the App ID field (`TEAMID.cloud.deltasync.capture`).

What you need on the Mac is listed in `docs/ARCHITECTURE.md`.
