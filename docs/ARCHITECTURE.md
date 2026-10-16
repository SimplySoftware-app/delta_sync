# Trust chain

DeltaSync v1 proves a photo was signed by a key that lives in the phone's secure hardware, and that the DeltaSync app asked for that certificate. The landing page at the repository root is unchanged. This document covers the new `server/`, `ios/`, and `android/` trees.

```
phone secure hardware                DeltaSync signing authority              later viewer
---------------------                ----------------------------              ------------
Secure Enclave / StrongBox    --->   checks attestation (live mode)    --->   POST /verify
signs the C2PA claim                 issues a 15-minute P-256 cert            c2pa-rs checks the
with a key that cannot               for that exact public key                manifest and the CA
leave the chip
```

1. The phone creates a P-256 key in secure hardware. iOS uses the Secure Enclave. Android prefers StrongBox and falls back to the hardware-backed Android Keystore, then to a software Keystore key if the device has neither.
2. The phone proves it holds that key by signing a PKCS#10 request. The private key is not sent anywhere.
3. The phone binds the key to a one-time server challenge. The binding is `SHA-256(challenge UTF-8 || 0x00 || uncompressed P-256 point)`. App Attest uses those 32 bytes as `clientDataHash`. Play Integrity uses the base64url form of that hash as the nonce.
4. The server checks the platform attestation when `ATTESTATION_MODE=live`, then signs a certificate for the key. The certificate lasts `DEVICE_CERT_TTL_SECONDS` (15 minutes unless you change it) and carries `id-kp-emailProtection`, which C2PA requires of a signing certificate.
5. The phone signs the C2PA manifest locally. c2pa-rs (through the official Swift and Android bindings) hashes the JPEG and asks the hardware key for an ES256 signature.
6. `POST /verify` reads the manifest with c2pa-rs 0.91.2 and trusts the DeltaSync CA that issued the device certificate.

The CA private key is created on first boot in `DATA_DIR` (`ca.key.pem`, mode 0600) or loaded from `CA_CERT_PATH` and `CA_KEY_PATH`. It is not in this repository. Back it up. A new CA makes older device certificates fail verification.

Challenges live in the memory of one process for five minutes and are single-use. A second replica will not see them.

## What is real

- C2PA manifest creation and validation, including the asset hash, the ES256 signature, and trust of the DeltaSync CA. The server test signs a JPEG with an issued device certificate and `/verify` returns `Trusted`. A changed scan byte no longer validates.
- PKCS#10 proof of possession. A certificate is not issued for a key the caller cannot sign with, including in stub mode.
- Short-lived X.509 device certificates with the C2PA end-entity profile (version 3, digitalSignature, emailProtection, authority key identifier).
- iOS Secure Enclave key creation and signing through `SecureEnclaveDeviceSigner` (`sdk/swift`), which the Rust core calls over UniFFI to sign the C2PA claim. The private key is not exported.
- App Attest on a physical iPhone: `DCAppAttestService.generateKey` and `attestKey`.
- Live App Attest checking on the server: CBOR attestation, x5c chain to the public Apple App Attestation Root CA, the nonce over `authData || clientDataHash`, `rpIdHash` against the app id, counter 0, the development or production AAGUID, and the credential id. A fixture chain that does not end at Apple's root is rejected. That root certificate is public and is the only PEM committed under `server/certs/`.
- Android StrongBox / Keystore key creation, hardware CSR signing, and C2PA signing through `KeystoreDeviceSigner` (`sdk/kotlin`), which the Rust core calls over UniFFI to sign the C2PA claim.
- Live Play Integrity checking: a Google service-account JWT, `decodeIntegrityToken`, then the nonce, package name, `PLAY_RECOGNIZED`, and `MEETS_DEVICE_INTEGRITY`.

## What is stubbed

- `ATTESTATION_MODE=stub` (the default in `.env.example`) issues a device certificate after the CSR check and does not look at App Attest or Play Integrity. The response says `attestation.verified: false`. Do not put a stub server on a public address.
- The App Attest receipt is not sent to Apple. Live mode checks the certificate chain and the nonce. Apple's receipt server is the piece still missing.
- The iOS Simulator cannot use the Secure Enclave or App Attest. A debug build has a switch that creates an ordinary keychain key and skips App Attest. That path is labeled in the UI and is not a hardware proof. It only works against a stub server.
- Android emulators usually have no StrongBox. The app records the backing it actually got (StrongBox, hardware Keystore, or software Keystore). "Continue without Play Integrity" skips the token and is only for a stub server. A sideloaded build will not receive `PLAY_RECOGNIZED`.
- The mobile apps capture a JPEG. The server also accepts PNG, WebP, and MP4 on `/verify`. Video capture is not in the apps.
- Challenges are not shared across processes. OCSP and a trusted timestamp are not embedded. C2PA reports `signingCredential.ocsp.skipped`.
- Verification trusts the DeltaSync CA for manifests this CA issued. It does not add the public C2PA trust list.

## HTTP

| Method | Path | Role |
| --- | --- | --- |
| GET | `/health` | Mode and CA fingerprint |
| POST | `/v1/challenge` | `{ "platform": "ios" \| "android" }` |
| POST | `/v1/device-certificate` | CSR plus attestation, returns the device certificate |
| POST | `/verify` | Multipart field `file` |

On Hetzner, `server/docker-compose.yml` joins an external Docker network named `caddy` and publishes no host port. Caddy proxies to `http://delta-sync:8080`. Do not add a `127.0.0.1` port mapping. The process inside the container binds `0.0.0.0:8080` so the proxy can reach it. See `server/deploy/Caddyfile.snippet`.

## What Carlos does on his Mac

You need a Mac with Xcode 26 for the iOS app, and Android Studio for the Android app. The server is Rust and can run on the Mac with `cd server && cargo run` after copying `server/.env.example` to `server/.env`. Leave `ATTESTATION_MODE=stub` until the accounts below exist. Point each phone at `http://<this Mac's LAN address>:8080`. The iOS app allows local HTTP. The Android app allows cleartext HTTP so a development server works; do not ship that setting.

### Apple

1. Enroll in the Apple Developer Program.
2. Register an explicit App ID. The project uses `cloud.deltasync.capture`. Enable the App Attest capability.
3. Open `ios/DeltaSync.xcodeproj` in Xcode 26. Set the Team. Let Xcode manage signing. If you change the bundle id, change the App ID field in the app to `<TeamID>.<bundle id>` and, for live mode, set `ALLOWED_APP_IDS` to that same value.
4. Use a physical iPhone on iOS 16 or later. The Secure Enclave and App Attest are not available in the Simulator. Plug it in, trust the developer certificate, and run the DeltaSync scheme.
5. In the app, set the server URL, tap Prepare device, then Capture and sign. Share the JPEG and upload it to `POST /verify`.
6. The entitlement is `development`. Switch `DeltaSync.entitlements` to `production` only for a distribution build, and use a production App Attest key. Development and production AAGUIDs are both accepted.

### Google

1. Create a Play Console app with application id `cloud.deltasync.capture`.
2. Link a Google Cloud project and enable the Play Integrity API. Put that project's number in the Android app. It is not a secret, but it is account-specific, so it is not committed.
3. Create a service account that Play Console allows to decode integrity tokens. Download its JSON and set `GOOGLE_PLAY_CREDENTIALS_FILE` to that path. Do not commit the JSON.
4. Set `ALLOWED_ANDROID_PACKAGES=cloud.deltasync.capture` and `ATTESTATION_MODE=live`.
5. Install the app from an internal testing track on a physical phone. Play Integrity returns `PLAY_RECOGNIZED` for that install, not for a USB sideload. StrongBox needs a device that has it; otherwise the app uses the strongest Keystore backing the phone has and says so on screen.
6. `cd android && ./gradlew :app:assembleDebug` needs Android SDK 36. Android Studio will install it.

### First run without those accounts

Run the server in stub mode. On the iPhone simulator, turn on "Simulator software key". On Android, turn on "Continue without Play Integrity". You can exercise capture and C2PA signing. The certificate is real and `/verify` will trust it, but the hardware attestation was not checked. The response says so.
