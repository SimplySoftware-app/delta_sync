# DeltaSync iOS

Minimal camera app for Xcode 26. It creates a P-256 key in the Secure Enclave, attests the app with App Attest, enrolls that signing key with the DeltaSync server, and embeds a C2PA manifest in the captured JPEG. The C2PA manifest is built by the Delta Sync Rust core (`crates/core`) through the local Swift package `sdk/swift` (UniFFI). The Secure Enclave key signs through `SecureEnclaveDeviceSigner`; the private key never reaches Rust.

Build the package's XCFramework once, then open the project:

```sh
sdk/swift/scripts/build-xcframework.sh   # from the repository root
```

The XCFramework has an arm64 simulator slice. Build for a concrete simulator (for example `-destination 'platform=iOS Simulator,name=iPhone 17'`), or run the script with `INCLUDE_X86_64_SIM=1` for the generic destination.

Open `DeltaSync.xcodeproj`. The bundle id is `cloud.deltasync.capture`. Change it to an App ID you control, and put the same Team ID and bundle id in the App ID field (`TEAMID.cloud.deltasync.capture`).

## Setup on a Mac

You need a Mac with Xcode 26. The server runs on the Mac with `cd server && cargo run` after copying `server/.env.example` to `server/.env` (see [server/README.md](../server/README.md)). Leave `ATTESTATION_MODE=stub` until the accounts below exist. Point the phone at `http://<this Mac's LAN address>:8080`. The iOS app allows local HTTP.

1. Enroll in the Apple Developer Program.
2. Register an explicit App ID. The project uses `cloud.deltasync.capture`. Enable the App Attest capability.
3. Open `ios/DeltaSync.xcodeproj` in Xcode 26. Set the Team. Let Xcode manage signing. If you change the bundle id, change the App ID field in the app to `<TeamID>.<bundle id>` and, for live mode, set `ALLOWED_APP_IDS` to that same value.
4. Use a physical iPhone on iOS 16 or later. The Secure Enclave and App Attest are not available in the Simulator. Plug it in, trust the developer certificate, and run the DeltaSync scheme.
5. In the app, set the server URL, tap Prepare device, then Capture and sign. Share the JPEG and upload it to `POST /verify`.
6. The entitlement is `development`. Switch `DeltaSync.entitlements` to `production` only for a distribution build, and use a production App Attest key. Development and production AAGUIDs are both accepted.

Without the accounts, run the server in stub mode and turn on "Simulator software key" in the app. You can exercise capture and C2PA signing. The certificate is real and `/verify` will trust it, but the hardware attestation was not checked. The response says so.
