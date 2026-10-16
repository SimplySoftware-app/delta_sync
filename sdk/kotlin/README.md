# Delta Sync core for Android

Android library (AAR) with the Delta Sync Rust core (`crates/core`) built by
`cargo-ndk` for `arm64-v8a` and `x86_64`, the UniFFI Kotlin bindings (JNA 5.17.0)
and a Keystore `DeviceSigner`.

- `KeystoreDeviceSigner`: EC P-256 key in the Android Keystore. StrongBox first,
  TEE as fallback, `setAttestationChallenge(nonce)`, and DER to raw `r || s` for
  COSE. It also builds the PKCS#10 request for `/v1/device-certificate`.
- `uniffi.delta_sync_core.signCaptureFile(inPath, outPath, signer)`: JPEG, MOV and
  MP4, streamed from disk.

Requirements on the build machine:

```sh
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk --locked
sdkmanager "ndk;27.2.12479018"
```

```sh
cd android
./gradlew :sdk:assembleRelease      # sdk/kotlin/build/outputs/aar/sdk-release.aar
./gradlew :sdk:testDebugUnitTest    # JVM tests against the host build of the core
./gradlew :sdk:connectedDebugAndroidTest   # on a device or emulator
```

The JVM tests sign a JPEG, MOV and MP4 with a software key generated for the run
and check the result verifies as HM. The instrumented tests exercise the real
Android Keystore API and the native library on Android. An emulator has no
StrongBox and no hardware TEE, so hardware backing is only testable on a
physical phone.
