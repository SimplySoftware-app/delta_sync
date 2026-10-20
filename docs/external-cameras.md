# Cámaras externas (listas oficiales C2PA)

Una captura firmada por un dispositivo que no es nuestro (una cámara o app de
terceros) solo se etiqueta **HM** si se cumplen todas estas condiciones. Si
falla cualquiera, el veredicto es **Unknown**; nunca se adivina.

1. El certificado de firma encadena a la C2PA Trust List fijada en el repositorio.
   La hoja no es CA, tiene `keyUsage` `digitalSignature` y el EKU
   `c2pa-kp-claimSigning` (1.3.6.1.4.1.62558.2.1) o `id-kp-documentSigning`
   (1.3.6.1.5.5.7.3.36). Las intermedias son CA y respetan su `pathLen`.
   Además, el ancla a la que llega la cadena tiene que ser de la entidad a la
   que está **vinculado** el producto (ver "Vínculo con el emisor"): un miembro
   de la Trust List no puede avalar el producto de otro fabricante.
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
   comparan sin los sufijos `__N` y `.vN`). Solo se reconocen los nombres de
   inception conocidos y las etiquetas `(c2pa|stds|cawg).[a-z0-9_]+(...)`; una
   URL IPTC completa, un comodín (`c2pa.*`) u otro espacio de nombres falla
   cerrado. La hoja no puede llevar `anyExtendedKeyUsage` y las intermedias
   necesitan `keyCertSign`.
6. Las ediciones posteriores son acciones permitidas firmadas por nuestro
   sello de servicio (Service ICA). Lo aplica `assign_label` en
   `crates/core/src/label.rs` (ver "Regla 6 en la etiqueta").

`Hm` en el veredicto solo significa que el **origen de la captura** es
elegible. Las ediciones posteriores (regla 6) las juzga la etiqueta, que aún
puede rebajar el resultado. Quien llama debe pasar datos de un manifest ya
validado por c2pa-rs (firma y hard bindings) y el MIME tomado de los bytes
reales del activo.

## Vínculo con el emisor

Sin vínculo, cualquiera de las 30 anclas de la Trust List podría emitir una
hoja `O=Google LLC, CN=Pixel Camera` y se aceptaría. Ahora cada ancla lleva el
`TEName` de su entidad, sacado de `trust/c2pa/C2PA-TRUST-LIST.json` (el mismo
upstream que el PEM, fijado en `SOURCE.lock`; si un certificado del PEM no
aparece en el JSON, el almacén no carga y todo es Unknown). Para cada producto:

1. **Derivado.** Se normalizan el `O` del DN, el `applicant` del registro y
   cada `TEName`: letras ASCII en minúsculas, cualquier otro carácter ASCII
   (puntuación, espacios) separa palabras, el resto de Unicode se deja igual y
   se quitan los sufijos legales finales (`inc`, `incorporated`, `llc`, `ltd`,
   `limited`, `co`, `corp`, `corporation`, `company`, `gmbh`, `bv`, `nv`,
   `plc`, `sa`, `ag`, `sl`, `srl`, `pte`, `pty`, `kk`) dejando al menos una
   palabra. Si exactamente una entidad coincide con el `O` o con el
   `applicant`, el producto queda vinculado a sus anclas.
2. **Curado.** Si no coincide ninguna o coincide más de una, el vínculo sale
   de `trust/external-issuer-bindings.json`, que es nuestro (no del upstream),
   se revisa en PR y da un motivo por entrada. Los emisores son `TEName`
   exactos de la lista; si alguno no está, la entrada no vale.
3. **Sin vínculo.** Si no hay ninguno, el producto es Unknown. Una entrada
   curada para un producto cuyo vínculo se puede derivar también lo deja sin
   vínculo: la tabla no puede ampliar en silencio lo derivado.

Con el commit `b842170c71` (fijado por un test), dos productos se vinculan por
derivación (Google LLC / Pixel Camera y Whole Earth Labs LLC / RealReel Android)
y seis por la tabla curada. `scripts/update-c2pa-trust.py --list-bindings`
imprime la lista completa.

Los seis curados no tienen ancla propia y no hay fuente pública que diga qué
CA emite sus certificados. Se limitan a las tres CAs comerciales auditadas
(WebTrust / CA/B Forum) de la lista que venden certificados C2PA a terceros y
validan la organización del sujeto. Así una raíz de un solo fabricante (Xiaomi,
vivo, Huawei, Adobe...) ya no puede emitir para ellos. Si uno de ellos firma en
realidad con otra CA, sus capturas son Unknown hasta añadir el emisor con
evidencia (falla cerrado, nunca se adivina).

## Regla 6 en la etiqueta

`label::external_manifest_record(id, info)` llama a
`external_capture_verdict`. Si el veredicto es `Hm`, el manifest recibe el rol
`SignerRole::External` con el producto y el emisor (`ExternalOrigin`); si no,
el rol es `Unknown`. Un mapa de roles (`report_from_store_with_roles`) no puede
dar el rol `External`: sin veredicto se trata como firmante ajeno.

Con un origen externo, `assign_label` da HM solo si:

- el propio manifest externo solo añade a su `c2pa.created` acciones
  permitidas (la misma lista que el sello de servicio);
- todos los demás manifests de la cadena son de nuestro Service ICA con
  acciones permitidas (`c2pa.edited` solo con las descripciones de mute y
  portada);
- no hay ningún manifest de nuestros dispositivos (`device_over_external`) ni
  otro origen de captura (`multiple_capture_origins`);
- siguen valiendo las reglas generales: IA declarada o detectada gana,
  y cualquier código de validación de hash, revocación o caducidad lo impide.

Cualquier otra cosa (un editor de terceros, una acción fuera de la lista, un
`c2pa.edited` sin descripción) da Unknown. El resumen dice el producto: `HM ·
captured with Pixel Camera (Google LLC), then edited: crop`.

Un mismo producto puede tener varios registros (versiones). Basta con que uno
los cumpla todos.

## Por qué nivel 2

Es el criterio aprobado: el nivel 2 del programa de conformidad C2PA incluye
atestación del dispositivo (por ejemplo `Android_KeyAttestation`) en
`attestationMethods`, es decir, evidencia de que la clave de firma está
protegida por el hardware. El nivel 1 no la exige. El
umbral es una constante en el código: solo se cambia con un PR, no en tiempo de
ejecución.

## Productos elegibles

La lista la genera `TrustStore::eligible_products` y la fija un test; con los
ficheros vendorizados, `scripts/update-c2pa-trust.py --list-eligible` la imprime
sin red. Hoy son ocho.

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
  conjunto elegible es exactamente el de la tabla, otro que los vínculos son
  los de la tabla de vínculos, y otros que `update-c2pa-trust.py`
  (`--list-eligible`, `--list-bindings`, `--normalise`) da lo mismo que Rust;
  un cambio de lista obliga a revisar todos. El resumen del PR semanal lista
  también el vínculo de cada producto elegible y marca `unbound` los que
  necesitan una entrada curada.

## Límites conocidos

- `verify_bytes` / `verify_file` todavía no llaman a
  `external_manifest_record`: c2pa-rs valida solo contra nuestra raíz, así que
  un manifest externo llega con `signingCredential.untrusted` y la cadena es
  Unknown igualmente. Conectarlo exige validar en c2pa-rs contra la Trust List
  sin que una CA de la lista pueda emitir algo con el nombre de nuestros ICAs
  (hoy `role_of_chain` identifica el ICA por su CN); queda como siguiente paso.
- No se comprueba revocación (CRL/OCSP).
- La validez de los certificados se evalúa siempre a la hora actual. No se
  verifican tokens RFC 3161, así que una captura firmada con un certificado ya
  caducado es Unknown. Verificarlos contra `C2PA-TSA-TRUST-LIST.pem` queda como
  trabajo futuro.
- Las firmas de certificado se verifican con `ring`: RSA PKCS#1, ECDSA P-256/P-384
  y Ed25519. Una cadena que necesite otro algoritmo (por ejemplo RSA-PSS) falla
  y queda Unknown.
