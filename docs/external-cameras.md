# Cámaras externas (listas oficiales C2PA)

Una captura firmada por un dispositivo que no es nuestro (una cámara o app de
terceros) solo se etiqueta **HM** si se cumplen todas estas condiciones. Si
falla cualquiera, el veredicto es **Unknown**; nunca se adivina.

1. El certificado de firma encadena a la C2PA Trust List fijada en el repositorio.
   La hoja no es CA, tiene `keyUsage` `digitalSignature` y el EKU
   `c2pa-kp-claimSigning` (1.3.6.1.4.1.62558.2.1) o `id-kp-documentSigning`
   (1.3.6.1.5.5.7.3.36). Las intermedias son CA y respetan su `pathLen`.
2. El sujeto tiene exactamente un `O` y un `CN`, y `O`, `CN`, `C` (y `OU` si el
   registro lo tiene) coinciden con un registro de la lista de productos
   conformes con `productType = generatorProduct` y `status = conformant`.
3. Ese registro tiene `maxAssuranceLevel >= 2` (`MIN_EXTERNAL_ASSURANCE`).
4. La versión del claim generator es `>= minVersion` del registro (si lo
   tiene) y el MIME del activo está en `containers.generate`. Las versiones se
   comparan como números con puntos o como número de build entero (Pixel usa
   `69307851`). Si no se puede interpretar, se rechaza.
5. El manifest tiene una sola acción de inicio (`c2pa.created` o
   `c2pa.opened`), es `c2pa.created` con `digitalSourceType = digitalCapture`,
   ninguna acción lleva otro `digitalSourceType` (IA, composite, etc.) y no hay
   acción, tipo de origen ni aserción de `disallowedSignals`
   (`capturedMediaStitched` equivale a `compositeCapture`; las etiquetas se
   comparan sin el sufijo `__N`). Una señal que el código no reconoce falla
   cerrado.
6. (Integración en la etiqueta, pendiente) Las ediciones posteriores deben ser
   acciones permitidas o nuestro sello de servicio.

`Hm` en el veredicto solo significa que el **origen de la captura** es
elegible. Las ediciones posteriores (regla 6) las juzga la integración en la
etiqueta, que aún puede rebajar el resultado. Quien llama debe pasar datos de un
manifest ya validado por c2pa-rs (firma y hard bindings) y el MIME tomado de los
bytes reales del activo.

Un mismo producto puede tener varios registros (versiones). Basta con que uno
los cumpla todos.

## Por qué nivel 2

Es el criterio aprobado: el nivel 2 del programa de conformidad C2PA incluye
atestación del dispositivo (por ejemplo `Android_KeyAttestation`) en
`attestationMethods`, es decir, evidencia de que la clave de firma está
protegida por el hardware. El nivel 1 no la exige. El
umbral es una constante en el código: solo se cambia con un PR, no en tiempo de
ejecución.

## Productos elegibles (commit `b842170c71`)

Generado por `TrustStore::eligible_products` y fijado por un test:

| Organización | Producto |
|---|---|
| Google LLC | Pixel Camera |
| Qualcomm Technologies, Inc. | Snapdragon 8 Elite Gen 5 |
| UX Vision LLC | Stroma |
| Whole Earth Labs LLC | RealReel Android |
| Rheely Inc. | Rheely for Android |
| Nuevo.Studio LLC | VWFNDR MBL Android |
| Evergreen Labs | GreenCheckmark - Android |
| EZDRM, INC | DynamicSigner |

Se excluye a quien tiene `capturedMedia` en `disallowedSignals`, porque no
puede declarar una captura (por ejemplo Eddie AI).

## Cámaras de marca fuera del programa

Leica, Sony, Nikon, Canon, Samsung y Apple no aparecen en la lista de
productos conformes. Aunque firmen con certificados de la Trust List fuera del
programa de conformidad, sus capturas son **Unknown**.

## Cómo se actualizan las listas

- Los ficheros están en `trust/c2pa/` tal cual los publica
  `c2pa-org/conformance-public` y `trust/c2pa/SOURCE.lock` guarda el commit y
  el sha256 de cada uno. Se incrustan en el binario al compilar, así que el
  veredicto de un binario queda fijado por su build.
- `scripts/update-c2pa-trust.py [commit]` descarga los ficheros a ese commit
  (por defecto, el último de `main`), comprueba que parsean, reescribe el lock e
  imprime qué productos ganan o pierden elegibilidad. El mismo commit produce
  siempre los mismos bytes.
- El workflow `c2pa-trust-update.yml` lo ejecuta cada lunes a las 06:00 UTC (y
  a mano) y abre un PR con ese resumen. Nunca se fusiona solo. Como un PR
  abierto con `GITHUB_TOKEN` no lanza CI, el workflow dispara `ci.yml` por
  `workflow_dispatch` en su rama para que los checks requeridos de `main` corran.
- La rama se llama `chore/c2pa-trust-<sha10>` (SHA del upstream). Si ya existe
  esa rama o un PR abierto, el workflow termina sin hacer nada y nunca fuerza.
  Para abrir el PR hay que activar el ajuste del repositorio "Allow GitHub
  Actions to create and approve pull requests".
- Un test comprueba que los hashes coinciden con `SOURCE.lock`, otro que el
  conjunto elegible es exactamente el de la tabla, y otro que el filtro de
  `update-c2pa-trust.py` (`--list-eligible`) da el mismo conjunto que Rust; un
  cambio de lista obliga a revisar todos.

## Límites conocidos

- No se comprueba revocación (CRL/OCSP).
- La validez de los certificados se evalúa siempre a la hora actual. No se
  verifican tokens RFC 3161, así que una captura firmada con un certificado ya
  caducado es Unknown. Verificarlos contra `C2PA-TSA-TRUST-LIST.pem` queda como
  trabajo futuro.
- Las firmas de certificado se verifican con `ring`: RSA PKCS#1, ECDSA P-256/P-384
  y Ed25519. Una cadena que necesite otro algoritmo (por ejemplo RSA-PSS) falla
  y queda Unknown.
