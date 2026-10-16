# Releases y cómo se consumen

Cada etiqueta `v*` ejecuta `.github/workflows/release.yml`, que construye y adjunta
a una GitHub Release:

| Artefacto | Contenido |
|---|---|
| `DeltaSyncCoreFFI.xcframework.zip` | XCFramework (iOS dispositivo + simulador arm64/x86_64) para un `binaryTarget` de SwiftPM |
| `DeltaSyncCoreFFI.xcframework.zip.checksum` | Resultado de `swift package compute-checksum` |
| `delta_sync_core.swift` | Bindings UniFFI generados para Swift |
| `delta-sync-core-release.aar` | AAR con `libdelta_sync_core.so` (arm64-v8a, x86_64) y los bindings Kotlin |
| `SHA256SUMS` | SHA-256 de todos los anteriores |

Para publicar: `git tag v0.2.0 && git push origin v0.2.0`. `workflow_dispatch` solo
construye y deja los artefactos en la ejecución, sin crear Release. El workflow no crea
etiquetas ni releases por su cuenta.

Pruebas locales de los pasos de empaquetado:

```sh
sdk/swift/scripts/build-xcframework.sh
scripts/release/package-apple.sh dist
cp sdk/kotlin/build/outputs/aar/sdk-release.aar dist/delta-sync-core-release.aar   # tras ./gradlew :sdk:assembleRelease
scripts/release/checksums.sh dist
```

## Rust (Cargo, dependencia git fijada a etiqueta)

```toml
[dependencies]
delta_sync_core = { git = "https://github.com/carlosedm10/delta_sync", tag = "v0.2.0" }
```

Desarrollo local contra un checkout, sin tocar la dependencia:

```toml
# .cargo/config.toml o [patch] en el Cargo.toml raíz del consumidor (no se commitea)
[patch."https://github.com/carlosedm10/delta_sync"]
delta_sync_core = { path = "../delta_sync/crates/core" }
```

## iOS (SwiftPM)

Los bindings Swift generados no se versionan; se descargan de la release.

```swift
// Package.swift del consumidor
.binaryTarget(
    name: "DeltaSyncCoreFFI",
    url: "https://github.com/carlosedm10/delta_sync/releases/download/v0.2.0/DeltaSyncCoreFFI.xcframework.zip",
    checksum: "<contenido de DeltaSyncCoreFFI.xcframework.zip.checksum>"
),
```

Añade a un target Swift propio `delta_sync_core.swift` (de la release) y los fuentes de
`sdk/swift/Sources/DeltaSyncCore` (`SecureEnclaveDeviceSigner.swift`, `Csr.swift`) de la
misma etiqueta, con dependencia del `binaryTarget`.

## Android (Gradle)

Descarga `delta-sync-core-release.aar` de la release (verifica `SHA256SUMS`) y:

```kotlin
// build.gradle.kts de la app
dependencies {
    implementation(files("libs/delta-sync-core-release.aar"))
    implementation("net.java.dev.jna:jna:5.17.0@aar") // las dependencias de un AAR local no se resuelven solas
}
```

Para desarrollo local, incluye el módulo directamente: `include(":sdk")` con
`project(":sdk").projectDir = file("../delta_sync/sdk/kotlin")`.
