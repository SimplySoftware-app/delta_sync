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
| 3 | Trait `DeviceSigner` (Secure Enclave / StrongBox) y `sign_capture_file` en el core; las apps de demo usan el core por UniFFI. JPEG, MOV y MP4. `sdk/swift` (XCFramework) y `sdk/kotlin` (AAR). | Este PR |
| 4 | Reglas de etiqueta por ICA, mute como `c2pa.edited` con descripción "audio removed" (añadida a las acciones permitidas), cover registrado como acción y póster firmado como derivado propio, TSA en la firma del dispositivo, BMFF con merkle. Fixtures de vídeo. | Pendiente |
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
- Lo que sigue en PR 4: reglas de etiqueta por ICA, TSA en la firma del dispositivo y fixtures de vídeo del round-trip con el servidor.

`server/Dockerfile` se construye ahora con la raíz del repositorio como contexto
(`server/docker-compose.yml` ya lo configura). El Dockerfile de la landing no
cambia.
