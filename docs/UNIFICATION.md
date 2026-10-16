# Unificación de Delta Sync y el núcleo de Social

Delta Sync pasa a ser la única fuente de verdad del sistema de firma C2PA propio
(captura, ediciones y derivados). Social deja de mantener su copia de
`delta-sync/` y consume este repositorio como dependencia Cargo por git, fijada a
una etiqueta, más los artefactos de release. Ese cambio en Social queda fuera de
estos 5 PR.

## Plan en 5 PR (apilados)

| PR | Contenido | Estado |
|---|---|---|
| 1 | Workspace Cargo (`server`, `crates/core`). Importa el núcleo Rust de Social (`delta_sync_core`) sin cambios de comportamiento, con sus tests. CI ejecuta `cargo test --workspace --locked`. Arregla el test inestable `live_mode_rejects_a_fake_attestation`. Licencia Apache-2.0 en todos los `Cargo.toml`. | Hecho |
| 2 | Autoridad: root, Device ICA y Service ICA (P-256), EKU `c2pa-kp-claimSigning` (1.3.6.1.4.1.62558.2.1) en lugar de emailProtection, cert de dispositivo de 24 h como máximo (por defecto 15 min), clave de servicio persistente, TSA RFC 3161 externa y configurable (por defecto `http://timestamp.digicert.com`; los tests no usan red), `POST /internal/seal` (el servicio firma derivados y ediciones con el cert del Service ICA, `parentOf` = original, solo si el original es una captura HM bajo la Device ICA), `POST /v1/verify` con el `LabelVerdict`. `/v1/enroll` solo con `ATTESTATION_MODE=stub`. | Hecho |
| 3 | Trait `DeviceSigner` (Secure Enclave / StrongBox) y `sign_capture_file` en el core; las apps de demo usan el core por UniFFI. JPEG, MOV y MP4. `sdk/swift` (XCFramework) y `sdk/kotlin` (AAR). | En revisión |
| 4 | Reglas de etiqueta por ICA, mute como `c2pa.edited` con descripción "audio removed" (añadida a las acciones permitidas), cover registrado como acción y póster firmado como derivado propio, TSA en la firma del dispositivo, BMFF con merkle. Fixtures de vídeo. | Este PR |
| 5 | CI de release: XCFramework, AAR y etiqueta. | Pendiente |

## Decisiones aprobadas

- Las ediciones las firma el servidor (cert de servicio) dentro del manifest del derivado.
- EKU `c2pa-kp-claimSigning` para ambas ICAs.
- TSA RFC 3161 externa y configurable; los tests no usan red.
- Cert de dispositivo de 24 h como máximo.
- Social consume `delta_sync` como dependencia Cargo por git fijada a una etiqueta.

## Estructura

```
Cargo.toml          # workspace
Cargo.lock
crates/core         # delta_sync_core (importado de Social, commit f394217)
server              # delta-sync-server
sdk/swift           # paquete SwiftPM: bindings UniFFI + SecureEnclaveDeviceSigner
sdk/kotlin          # módulo Android library (AAR): bindings UniFFI + KeystoreDeviceSigner
ios/ android/       # apps de demo, ya sobre el core por UniFFI
```

## Firma en el dispositivo (PR 3)

La clave privada no llega nunca a Rust. El core construye el claim C2PA y pide la
firma a un `DeviceSigner` implementado en la plataforma:

```
trait DeviceSigner { alg() -> SignerAlg; cert_chain() -> String (PEM); sign(bytes) -> raw r||s }
sign_capture_file(in_path, out_path, signer)   // JPEG, MOV y MP4, por streaming
verify_file(path, mime?, ca_pem, declared_ai)  // verifica sin cargar el fichero en memoria
```

- iOS: `SecureEnclaveDeviceSigner` (P-256 no exportable). El DER de Security.framework se convierte a `r || s`.
- Android: `KeystoreDeviceSigner` (StrongBox con respaldo a TEE, `setAttestationChallenge`, DER a `r || s`).
- Memoria: el origen y la salida se leen y escriben como ficheros y c2pa deja el intermedio en disco por encima de 4 MB. El pico lo marca el buffer de hash de c2pa 0.91.2 (hasta 256 MB por tramo, fijo en su código), no el tamaño total.

`server/Dockerfile` se construye ahora con la raíz del repositorio como contexto
(`server/docker-compose.yml` ya lo configura). El Dockerfile de la landing no
cambia.

## Reglas de etiqueta por ICA, mute/cover y vídeo (PR 4)

El rol de cada manifest de la cadena se lee de la ICA que emitió su certificado
(`role_of_chain` en `crates/core/src/sign.rs`). La cadena ya la validó c2pa contra
la raíz, así que la ICA es la de la raíz: solo la raíz puede firmar un certificado
con el nombre `Delta Sync Device ICA` o `Delta Sync Service ICA`.

| Manifest firmado por | Puede | No puede (el resultado es Unknown) |
|---|---|---|
| Device ICA | `c2pa.created` con `digitalCapture` (y `c2pa.opened`) | cualquier edición o sello (`c2pa.transcoded`, `c2pa.cropped`...) |
| Service ICA | ediciones, `c2pa.transcoded`, `c2pa.resized`, `c2pa.edited` solo con descripción `audio removed` o `cover frame selected` | `c2pa.created`, cualquier `digitalCapture`, cualquier otra acción |
| Otra cosa (cualquier ingrediente) | nada | todo: un solo manifest ajeno deja la cadena en Unknown |

Para HM hace falta además un `c2pa.created` + `digitalCapture` firmado por la Device
ICA. `LabelVerdict.signers` lista el rol de cada manifest (más reciente primero) y
`/internal/seal` solo acepta como origen una cadena cuyo único firmante es `device`.

- **mute**: `{"op":"mute"}` se registra como `c2pa.edited` con descripción `audio removed`.
- **cover**: `{"op":"cover","at_ms":N}` se registra como `c2pa.edited` con descripción
  `cover frame selected`. El póster se sella como derivado propio: el vídeo firmado es el
  `original` (su `parentOf`) y la imagen del póster es `rendered`.
- **Vídeo**: MP4 y MOV se firman con `c2pa.hash.bmff` v3 y un árbol Merkle de 1024 KB
  (`core.merkle_tree_chunk_size_in_kb`), tanto en la captura como en el sello.
- **TSA**: `TimestampProvider` (trait UniFFI `with_foreign`) deja que la plataforma haga la
  petición RFC 3161 y `sign_capture_file_with_timestamp` la usa al firmar la captura.
  Para verificar un certificado de dispositivo ya caducado hay que confiar en la raíz de la
  TSA (`VerifyOptions::with_tsa_anchors`, `TSA_TRUST_ANCHORS_PATH` en el servidor). Sin
  ella el label cae a Unknown cuando caduca el certificado.
- `DevAuthority` (dev/tests) tiene ahora la misma forma que la PKI del servidor: raíz,
  Device ICA y Service ICA, con EKU `c2pa-kp-claimSigning`.
