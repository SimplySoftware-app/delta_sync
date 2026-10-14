# DeltaSync iOS

Minimal camera app for Xcode 26. It creates a P-256 key in the Secure Enclave, attests the app with App Attest, enrolls that signing key with the DeltaSync server, and embeds a C2PA manifest in the captured JPEG. Signing uses the official [c2pa-swift](https://github.com/contentauth/c2pa-swift) 0.0.14 bindings (c2pa-rs).

Open `DeltaSync.xcodeproj`. The bundle id is `cloud.deltasync.capture`. Change it to an App ID you control, and put the same Team ID and bundle id in the App ID field (`TEAMID.cloud.deltasync.capture`).

What you need on the Mac is listed in `docs/ARCHITECTURE.md`.
