# DeltaSync Android

Camera app that creates a P-256 key in StrongBox (or the TEE / Android Keystore when StrongBox is missing), requests a Play Integrity token, enrolls the key with the DeltaSync server, and embeds a C2PA manifest in the captured JPEG.

C2PA comes from the official [c2pa-android](https://github.com/contentauth/c2pa-android) 0.0.14 library, which wraps c2pa-rs. Open the `android/` directory in Android Studio. The application id is `cloud.deltasync.capture`.

The emulator reaches a server on the host at `http://10.0.2.2:8080`. A phone uses the computer's LAN address. What you need in Play Console is listed in `docs/ARCHITECTURE.md`.
