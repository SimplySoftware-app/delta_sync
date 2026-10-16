# Unificación de Delta Sync y el núcleo de Social

Delta Sync pasa a ser la única fuente de verdad del sistema de firma C2PA propio
(captura, ediciones y derivados). Social deja de mantener su copia de
`delta-sync/` y consume este repositorio como dependencia Cargo por git, fijada a
una etiqueta, más los artefactos de release. Ese cambio en Social queda fuera de
estos 5 PR.

## Plan en 5 PR (apilados)

| PR | Contenido | Estado |
|---|---|---|
| 1 | Workspace Cargo (`server`, `crates/core`). Importa el núcleo Rust de Social (`delta_sync_core`) sin cambios de comportamiento, con sus tests. CI ejecuta `cargo test --workspace --locked`. Arregla el test inestable `live_mode_rejects_a_fake_attestation`. Licencia Apache-2.0 en todos los `Cargo.toml`. | Este PR |
| 2 | PKI propia: root offline, Device ICA y Service ICA. EKU `c2pa-kp-claimSigning` (1.3.6.1.4.1.62558.2.1) en lugar de emailProtection. TTL máximo del cert de dispositivo de 24 h (por defecto 15 min, configurable). TSA RFC 3161 externa y configurable (por defecto `http://timestamp.digicert.com`), opcional y simulada en tests. | Pendiente |
| 3 | Sello del servidor: el servicio firma ediciones y derivados con el cert del Service ICA dentro del manifest del derivado (`parentOf` = original). Solo si el original verifica como HM bajo la Device ICA. | Pendiente |
| 4 | Acciones: mute como `c2pa.edited` con descripción "audio removed" (añadida a las acciones permitidas), cover registrado como acción y póster firmado como derivado propio. | Pendiente |
| 5 | Firma de captura con clave en hardware (Secure Enclave / StrongBox) mediante un trait `DeviceSigner` y API basada en ficheros (`sign_capture_file`), con soporte de vídeo. | Pendiente |

## Decisiones aprobadas

- Las ediciones las firma el servidor (cert de servicio) dentro del manifest del derivado.
- EKU `c2pa-kp-claimSigning` para ambas ICAs.
- TSA RFC 3161 externa y configurable; los tests no usan red.
- Cert de dispositivo de 24 h como máximo.
- Social consume `delta_sync` como dependencia Cargo por git fijada a una etiqueta.

## Estructura tras el PR 1

```
Cargo.toml          # workspace
Cargo.lock
crates/core         # delta_sync_core (importado de Social, commit f394217)
server              # delta-sync-server
```

`server/Dockerfile` se construye ahora con la raíz del repositorio como contexto
(`server/docker-compose.yml` ya lo configura). El Dockerfile de la landing no
cambia.
