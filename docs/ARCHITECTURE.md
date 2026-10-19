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
4. The server checks the platform attestation when `ATTESTATION_MODE=live`, then signs a certificate for the key. The certificate lasts `DEVICE_CERT_TTL_SECONDS` (15 minutes unless you change it) and carries the C2PA claim-signing EKU (extended key usage, the certificate field that says what a key may sign), `1.3.6.1.4.1.62558.2.1`, which C2PA requires of a signing certificate.
5. The phone signs the C2PA manifest locally. c2pa-rs (through the official Swift and Android bindings) hashes the JPEG and asks the hardware key for an ES256 signature.
6. `POST /verify` reads the manifest with c2pa-rs 0.91.2 and trusts the DeltaSync root that issued the device certificate.

The PKI is a root with two intermediate CAs (ICAs), a Device ICA for captures and a Service ICA for server seals, generated on first boot in `DATA_DIR` (keys mode 0600) or loaded from paths; the root key is not kept. No key is in this repository. Back up the ICA keys. A new hierarchy makes older device certificates fail verification. Details: [server/README.md](../server/README.md#pki).

Challenges live in the memory of one process for five minutes and are single-use. A second replica will not see them.

## What is real

- C2PA manifest creation and validation, including the asset hash, the ES256 signature, and trust of the DeltaSync CA. The server test signs a JPEG with an issued device certificate and `/verify` returns `Trusted`. A changed scan byte no longer validates.
- PKCS#10 proof of possession. A certificate is not issued for a key the caller cannot sign with, including in stub mode.
- Short-lived X.509 device certificates with the C2PA end-entity profile (version 3, digitalSignature, claim-signing EKU, authority key identifier).
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
- Challenges are not shared across processes. OCSP is not embedded. C2PA reports `signingCredential.ocsp.skipped`.
- Verification trusts the DeltaSync CA for manifests this CA issued. Third-party cameras are accepted only through the pinned official C2PA lists, under the rules in [external-cameras.md](external-cameras.md).

## Key decisions and caveats (why it is this way)

- **One source of truth for C2PA signing** — this repository owns capture, edits and derivatives. Consumers take `delta_sync_core` as a git dependency pinned to a tag, or the release artifacts ([RELEASES.md](RELEASES.md)).
- **The server signs edits, the device only signs captures** — derivatives are sealed by the service certificate with the original as `parentOf` ([server/README.md](../server/README.md#sealing-post-internalseal)), so a seal never turns something into HM.
- **The issuing ICA decides what a manifest may claim** (`role_of_chain` in `crates/core/src/sign.rs`, rules in `label.rs`). The chain is validated against our root first, so only the root can issue a certificate named like one of our ICAs. A Device ICA manifest may only create (`digitalCapture`) or open; a Service ICA manifest may only edit, and `c2pa.edited` counts only with the "audio removed" or "cover frame selected" description; it can never create a capture. One manifest from any other signer leaves the whole chain Unknown. External cameras are the one exception, under [external-cameras.md](external-cameras.md).
- **Memory** — the core reads and writes assets as files and c2pa leaves its intermediate on disk above 4 MB, so the peak is set by c2pa 0.91.2's hash buffer (up to 256 MB per range, fixed in its code), not by the file size.
- **Time-stamps** — a device certificate lives minutes, so a capture only verifies after it expires if verification trusts the root of the time-stamp authority ([server/README.md](../server/README.md#time-stamps)). Tests never use the network.

## Where the details live

- Run the server: [server/README.md](../server/README.md), including endpoints, the Hetzner and Caddy setup and the PKI.
- Apple Developer and App Attest setup: [ios/README.md](../ios/README.md). Google Play Console and Play Integrity setup: [android/README.md](../android/README.md).
- Releases and how to consume them: [RELEASES.md](RELEASES.md).
