# DeltaSync server

HTTP service that issues short-lived device certificates and checks C2PA manifests. The Next.js site at the repo root is separate and is not served from here.

## Run locally

```bash
cd server
cp .env.example .env
# ATTESTATION_MODE=stub is the default for a laptop.
cargo run
```

The process listens on `0.0.0.0:8080`. A phone on the same network uses your machine's LAN address.

## Endpoints

- `GET /health`
- `POST /v1/challenge` with `{ "platform": "ios" | "android" }`
- `POST /v1/device-certificate` verifies the CSR and, in live mode, App Attest or Play Integrity, then returns a device certificate (`certificate_pem`, `certificate_chain_pem` = leaf + Device ICA, `ca_pem` = the root, the client's trust anchor)
- `POST /v1/verify` raw asset as the body, headers `x-mime-type` (optional, sniffed if missing) and `x-self-declared-ai: true` (optional). Answers the HM / AI / Unknown verdict: `{ trusted, label, summary, reasons, actions }`
- `POST /verify` multipart field `file` (JPEG, PNG, WebP, or MP4). Kept for backward compatibility. It returns the raw C2PA validation, not the label
- `POST /internal/seal` for the media processor only, see below
- `POST /v1/enroll` deprecated, **only when `ATTESTATION_MODE=stub`**. A bare public key and a non-empty attestation string that nothing checks. It returns a normal device certificate. Use the challenge flow

## PKI

```text
Root (P-256)
├── Device ICA   device certificates, 24 h at most (default 15 min), only after attestation
└── Service ICA  the certificate the service seals derivatives with (1 year, renewed on boot)
```

Every leaf has the C2PA claim-signing EKU `1.3.6.1.4.1.62558.2.1` (emailProtection is gone). A verifier tells a device capture from a service seal by the ICA in the chain, so a compromised device cannot issue seals and the service cannot fabricate captures. This is our own CA, not the C2PA trust list: other verifiers will show the credential as untrusted.

On a first boot with an empty `DATA_DIR` the service generates the whole hierarchy: `root.crt.pem`, `device-ica.{crt,key}.pem`, `service-ica.{crt,key}.pem`, `service.{crt,key}.pem` (keys are mode 0600). **It does not keep the root key.** The service key is read from disk on every boot, never generated per request. Back up the ICA keys. For a public host provision the root and both ICAs from outside and point `ROOT_CERT_PATH`, `DEVICE_ICA_*`, `SERVICE_ICA_*` (and optionally `SERVICE_CERT_PATH`/`SERVICE_KEY_PATH`) at them. A partial set of files is refused rather than overwritten.

Upgrading from the single-CA layout (`ca.crt.pem`, `ca.key.pem`): those files are no longer read. The service creates the new hierarchy next to them, so old device certificates and old manifests stop validating. They lived 15 minutes, but anything already signed under the old CA will read as Unknown.

## Sealing (`POST /internal/seal`)

For the media processor, on the private network. Caddy answers 404 for `/internal/*` (see `deploy/Caddyfile.snippet`). The caller sends `Authorization: Bearer $MEDIA_SEAL_TOKEN`. Without `MEDIA_SEAL_TOKEN` configured the route answers 503; a wrong token is 401.

Multipart fields:

- `original`: the device-signed asset (`image/jpeg|png|webp`, `video/mp4|quicktime`, 32 MiB at most)
- `rendered`: the bytes the processor rendered from it
- `edits` (optional): JSON array such as `[{"op":"crop","x":0,"y":0,"width":800,"height":600},{"op":"text","body":"hi"}]` (8 at most). Ops: `crop`, `trim`, `rotate`, `text`, `filter`, `mute` (recorded as `c2pa.edited`, "audio removed") and `cover` (`{"op":"cover","at_ms":1500}`, `c2pa.edited`, "cover frame selected")
- `resized` (optional): `true` when the rendered asset has fewer pixels

Header `x-self-declared-ai: true` marks the derivative as AI.

A video poster is sealed as its own derivative: send the device-signed video as `original`, the poster image as `rendered` (`image/jpeg`) and `[{"op":"cover","at_ms":N}]`. The poster's `parentOf` ingredient is the video. MP4 and MOV are hashed with `c2pa.hash.bmff` v3 and a Merkle tree over 1024 KB chunks of the media payload.

The answer is `rendered` with a new manifest signed by the service certificate: the edit actions, `c2pa.transcoded` (and `c2pa.resized`), and `original` as its `parentOf` ingredient. The service refuses with 422 unless `original` verifies as an HM capture signed under the Device ICA. An unsigned file, a capture from another authority, or a file the service sealed itself is refused, so a seal never turns something into HM. 502 means the time-stamp authority was unreachable.

At most `SEAL_MAX_CONCURRENCY` (default 2) seals run at once. The 32 MiB cap stays until the path streams. Each seal keeps roughly four to five copies of the asset in memory.

## Time-stamps

`TSA_URL` is an RFC 3161 authority used for service seals. Unset it is `http://timestamp.digicert.com`. Set it empty or `off` to turn it off. It is on by default, so the service needs outbound HTTP; if the authority is down, sealing fails with 502 instead of producing a seal without a time-stamp. Tests never use the network.

`TSA_TRUST_ANCHORS_PATH` is a PEM file with the root certificates of the time-stamp authorities that verification trusts to date a signature (for DigiCert, its timestamping root chain). With it, a capture signed while its short-lived device certificate was valid, and time-stamped by one of those authorities, still verifies after the certificate expires. Without it a time stamp from an unknown authority proves nothing, is reported as `timeStamp.untrusted` (informational), and an expired device certificate makes the label Unknown. The device asks its own time-stamp authority (`sign_capture_file_with_timestamp` in the core: the platform makes the HTTP request).

## Verify a photo

`POST /verify` checks a photo's C2PA manifest and returns JSON with `manifest_found`, `validation_state`, the manifest and the validation results. If a single byte changed after signing, validation fails.

```bash
curl -F "file=@photo.jpg" https://<your-server>/verify
# locally: curl -F "file=@photo.jpg" http://localhost:8080/verify
```

Use this endpoint rather than a generic Content Credentials viewer such as contentcredentials.org/verify: those viewers trust the public C2PA list, and DeltaSync signs with its own certificate authority (`ca.crt.pem`), so they will not show it as trusted.

## Hetzner and Caddy

```bash
docker network create caddy   # once, shared with the Caddy container
cd server
cp .env.example .env          # set live values before a public host
docker compose up -d --build
```

Compose joins the external network named `caddy` (override with `CADDY_DOCKER_NETWORK`) and does not publish a host port. Caddy proxies to `http://delta-sync:8080`. See `deploy/Caddyfile.snippet`.

## Tests

```bash
cargo test --workspace   # from the repo root; `cargo test` inside server/ also works
```
