# DeltaSync Android

Camera app that creates a P-256 key in StrongBox (or the TEE / Android Keystore when StrongBox is missing), requests a Play Integrity token, enrolls the key with the DeltaSync server, and embeds a C2PA manifest in the captured JPEG.

The C2PA manifest is built by the Delta Sync Rust core (`crates/core`), packaged as the `:sdk` module (`sdk/kotlin`) with UniFFI Kotlin bindings. The Keystore key signs through `KeystoreDeviceSigner`; the private key never reaches Rust. Building needs `rustup target add aarch64-linux-android x86_64-linux-android`, `cargo install cargo-ndk --locked` and the NDK (`sdkmanager "ndk;27.2.12479018"`). Open the `android/` directory in Android Studio. The application id is `cloud.deltasync.capture`.

The emulator reaches a server on the host at `http://10.0.2.2:8080`. A phone uses the computer's LAN address.

## Setup

You need Android Studio. The server runs on a computer with `cd server && cargo run` after copying `server/.env.example` to `server/.env` (see [server/README.md](../server/README.md)). Leave `ATTESTATION_MODE=stub` until the accounts below exist. The app allows cleartext HTTP so a development server works; do not ship that setting.

1. Create a Play Console app with application id `cloud.deltasync.capture`.
2. Link a Google Cloud project and enable the Play Integrity API. Put that project's number in the Android app. It is not a secret, but it is account-specific, so it is not committed.
3. Create a service account that Play Console allows to decode integrity tokens. Download its JSON and set `GOOGLE_PLAY_CREDENTIALS_FILE` to that path. Do not commit the JSON.
4. Set `ALLOWED_ANDROID_PACKAGES=cloud.deltasync.capture` and `ATTESTATION_MODE=live`.
5. Install the app from an internal testing track on a physical phone. Play Integrity returns `PLAY_RECOGNIZED` for that install, not for a USB sideload. StrongBox needs a device that has it; otherwise the app uses the strongest Keystore backing the phone has and says so on screen.
6. `cd android && ./gradlew :app:assembleDebug` needs Android SDK 36. Android Studio will install it.

Without the accounts, run the server in stub mode and turn on "Continue without Play Integrity" in the app. You can exercise capture and C2PA signing. The certificate is real and `/verify` will trust it, but the hardware attestation was not checked. The response says so.
