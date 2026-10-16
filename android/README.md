# DeltaSync Android

Camera app that creates a P-256 key in StrongBox (or the TEE / Android Keystore when StrongBox is missing), requests a Play Integrity token, enrolls the key with the DeltaSync server, and embeds a C2PA manifest in the captured JPEG.

The C2PA manifest is built by the Delta Sync Rust core (`crates/core`), packaged as the `:sdk` module (`sdk/kotlin`) with UniFFI Kotlin bindings. The Keystore key signs through `KeystoreDeviceSigner`; the private key never reaches Rust. Building needs `rustup target add aarch64-linux-android x86_64-linux-android`, `cargo install cargo-ndk --locked` and the NDK (`sdkmanager "ndk;27.2.12479018"`). Open the `android/` directory in Android Studio. The application id is `cloud.deltasync.capture`.

The emulator reaches a server on the host at `http://10.0.2.2:8080`. A phone uses the computer's LAN address. What you need in Play Console is listed in `docs/ARCHITECTURE.md`.
