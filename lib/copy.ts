import type { Locale } from "@/lib/locale"

const SITE = "https://deltasync.simplysoftware.app"

export const links = {
  c2pa: "https://c2pa.org/",
  simply: "https://simplysoftware.app",
  c2paRs: "https://github.com/contentauth/c2pa-rs",
  c2paSwift: "https://github.com/contentauth/c2pa-swift",
  c2paAndroid: "https://github.com/contentauth/c2pa-android",
} as const

type NavItem = {
  href: string
  label: string
}

type Step = {
  title: string
  body: string
}

type PackageCopy = {
  title: string
  body: string
  badges: { label: string; href?: string }[]
}

type Limit = {
  title: string
  body: string
}

export type ChromeCopy = {
  skip: string
  product: string
  navLabel: string
  openMenu: string
  closeMenu: string
  menuTitle: string
  theme: string
  system: string
  light: string
  dark: string
  language: string
  english: string
  spanish: string
  nav: NavItem[]
  tagline: string
  credit: string
  opensInNewTab: string
}

export type HomeCopy = {
  metaTitle: string
  metaDescription: string
  badge: string
  title: string
  leadBefore: string
  leadLink: string
  leadAfter: string
  packagesLine: string
  howTitle: string
  howIntro: string
  steps: Step[]
  packagesTitle: string
  packagesIntro: string
  packages: PackageCopy[]
  limitsTitle: string
  limitsIntro: string
  limits: Limit[]
}

export type AboutCopy = {
  metaTitle: string
  metaDescription: string
  title: string
  lead: string
  body: string
  more: string
}

type Dictionary = {
  chrome: ChromeCopy
  home: HomeCopy
  about: AboutCopy
}

const en: Dictionary = {
  chrome: {
    skip: "Skip to content",
    product: "DeltaSync",
    navLabel: "On this page",
    openMenu: "Open menu",
    closeMenu: "Close",
    menuTitle: "Menu",
    theme: "Theme",
    system: "System",
    light: "Light",
    dark: "Dark",
    language: "Language",
    english: "English",
    spanish: "Español",
    nav: [
      { href: "/#how", label: "How it works" },
      { href: "/#packages", label: "Packages" },
      { href: "/#limits", label: "Limits" },
      { href: "/about", label: "About" },
    ],
    tagline: "Open-source media provenance.",
    credit: "by Simply Software",
    opensInNewTab: "opens in a new tab",
  },
  home: {
    metaTitle: "DeltaSync — Open-source media provenance",
    metaDescription:
      "Open-source SDKs and a C2PA protocol. A phone signs a photo with a key that stays on the device, and anyone can check that signature later.",
    badge: "Open source",
    title: "Open-source media provenance.",
    leadBefore: "DeltaSync is a set of SDKs and a small protocol, built on ",
    leadLink: "C2PA",
    leadAfter:
      ". A phone signs a photo with a key that stays on the device. Someone else can check that signature later.",
    packagesLine:
      "Three packages: a Rust server (c2pa-rs), an iOS SDK (Swift, c2pa-swift, App Attest) and an Android SDK (Kotlin, c2pa-android, Play Integrity).",
    howTitle: "How it works",
    howIntro:
      "DeltaSync v1 shows that a photo was signed by a key in the phone’s secure hardware, and that the DeltaSync app asked for the certificate.",
    steps: [
      {
        title: "A key on the phone",
        body: "The phone creates a P-256 key. iOS uses the Secure Enclave. Android prefers StrongBox, then a hardware-backed keystore, then a software keystore if the device has neither. The private key is not sent to the server. When the key is in secure hardware, it stays in the chip.",
      },
      {
        title: "Proof it is this app",
        body: "The phone proves it holds the key by signing a certificate request. In live mode it also proves that the DeltaSync app asked for the certificate: App Attest on iOS, Play Integrity on Android. The proof is bound to a one-time server challenge.",
      },
      {
        title: "A short-lived certificate",
        body: "The server checks that attestation in live mode, then signs a certificate for that exact public key. The certificate lasts 15 minutes by default and uses the profile C2PA expects of a signing certificate.",
      },
      {
        title: "The photo is signed locally",
        body: "The phone signs a C2PA manifest for the JPEG on the device. c2pa-swift and c2pa-android wrap c2pa-rs: they hash the file and ask the key for an ES256 signature.",
      },
      {
        title: "A later check",
        body: "A viewer sends the file to the server. c2pa-rs reads the manifest and trusts the DeltaSync certificate authority that issued the device certificate. A changed byte no longer validates.",
      },
    ],
    packagesTitle: "Packages",
    packagesIntro:
      "The same protocol, in three trees. The phone apps capture a JPEG and embed the manifest. The server issues the certificate and checks the file.",
    packages: [
      {
        title: "Rust server",
        body: "The signing authority and the check. It uses c2pa-rs to read the manifest. In live mode it checks App Attest or Play Integrity before it issues the certificate. The certificate authority key is created on the server. It is not in this repository.",
        badges: [
          { label: "Rust" },
          { label: "c2pa-rs", href: links.c2paRs },
        ],
      },
      {
        title: "iOS SDK",
        body: "Swift, using c2pa-swift and App Attest. The signing key is created in the Secure Enclave and is not exported. The Simulator cannot use the Secure Enclave or App Attest. A debug build can use an ordinary key and labels that path. That is not a hardware proof, and it only works against a stub server.",
        badges: [
          { label: "Swift" },
          { label: "c2pa-swift", href: links.c2paSwift },
          { label: "App Attest" },
        ],
      },
      {
        title: "Android SDK",
        body: "Kotlin, using c2pa-android and Play Integrity. The app prefers StrongBox and records the backing it actually got: StrongBox, hardware keystore, or software keystore. Skipping Play Integrity is only for a stub server. The recognized Play verdict is for an install from Play, not a sideload.",
        badges: [
          { label: "Kotlin" },
          { label: "c2pa-android", href: links.c2paAndroid },
          { label: "Play Integrity" },
        ],
      },
    ],
    limitsTitle: "What this does not do yet",
    limitsIntro:
      "The check is real for a manifest this certificate authority signed. A few pieces are still missing, and the software says so.",
    limits: [
      {
        title: "Photos, not video",
        body: "The apps capture a JPEG. They do not capture video. The check also accepts PNG, WebP, and MP4.",
      },
      {
        title: "Stub mode skips attestation",
        body: "A local server defaults to stub mode. It still requires proof the caller holds the key, and it does not check App Attest or Play Integrity. The response says the attestation was not verified.",
      },
      {
        title: "Apple’s receipt is not sent",
        body: "Live App Attest checks the certificate chain and the nonce. The receipt is not sent to Apple yet.",
      },
      {
        title: "This authority, not the public list",
        body: "Verification trusts the DeltaSync certificate authority for manifests it issued. It does not add the public C2PA trust list. A trusted timestamp and an OCSP response are not embedded yet.",
      },
    ],
  },
  about: {
    metaTitle: "About us — DeltaSync",
    metaDescription:
      "Simply Software makes simple, cheap, secure, reliable apps, with no ads, nag emails or addictive patterns.",
    title: "About us",
    lead: "Simply Software makes simple, cheap, secure, reliable apps.",
    body: "The apps have no ads, no nag emails and no addictive patterns. DeltaSync is one of them: open-source media provenance, a set of SDKs and a protocol built on C2PA.",
    more: "See more from Simply Software",
  },
}

const es: Dictionary = {
  chrome: {
    skip: "Saltar al contenido",
    product: "DeltaSync",
    navLabel: "En esta página",
    openMenu: "Abrir menú",
    closeMenu: "Cerrar",
    menuTitle: "Menú",
    theme: "Tema",
    system: "Sistema",
    light: "Claro",
    dark: "Oscuro",
    language: "Idioma",
    english: "English",
    spanish: "Español",
    nav: [
      { href: "/#how", label: "Cómo funciona" },
      { href: "/#packages", label: "Paquetes" },
      { href: "/#limits", label: "Límites" },
      { href: "/about", label: "Sobre nosotros" },
    ],
    tagline: "Procedencia de medios en código abierto.",
    credit: "by Simply Software",
    opensInNewTab: "se abre en una pestaña nueva",
  },
  home: {
    metaTitle: "DeltaSync — Procedencia de medios en código abierto",
    metaDescription:
      "SDKs de código abierto y un protocolo sobre C2PA. El teléfono firma una foto con una clave que se queda en el dispositivo, y cualquiera puede comprobar esa firma después.",
    badge: "Código abierto",
    title: "Procedencia de medios, en código abierto.",
    leadBefore:
      "DeltaSync es un conjunto de SDKs y un protocolo pequeño, construido sobre ",
    leadLink: "C2PA",
    leadAfter:
      ". El teléfono firma una foto con una clave que se queda en el dispositivo. Otra persona puede comprobar esa firma después.",
    packagesLine:
      "Tres paquetes: un servidor en Rust (c2pa-rs), un SDK de iOS (Swift, c2pa-swift, App Attest) y un SDK de Android (Kotlin, c2pa-android, Play Integrity).",
    howTitle: "Cómo funciona",
    howIntro:
      "DeltaSync v1 demuestra que una foto fue firmada por una clave del hardware seguro del teléfono, y que la aplicación DeltaSync pidió el certificado.",
    steps: [
      {
        title: "Una clave en el teléfono",
        body: "El teléfono crea una clave P-256. iOS usa el Secure Enclave. Android prefiere StrongBox, después un almacén de claves con respaldo de hardware y, si el dispositivo no tiene ninguno, un almacén de software. La clave privada no se envía al servidor. Cuando la clave está en hardware seguro, se queda en el chip.",
      },
      {
        title: "Prueba de que es esta aplicación",
        body: "El teléfono demuestra que posee la clave firmando una solicitud de certificado. En modo real también demuestra que la aplicación DeltaSync pidió el certificado: App Attest en iOS y Play Integrity en Android. La prueba queda ligada a un reto de un solo uso del servidor.",
      },
      {
        title: "Un certificado breve",
        body: "El servidor comprueba esa atestación en modo real y firma un certificado para esa clave pública exacta. El certificado dura 15 minutos por defecto y usa el perfil que C2PA exige a un certificado de firma.",
      },
      {
        title: "La foto se firma en el teléfono",
        body: "El teléfono firma un manifiesto C2PA del JPEG en el propio dispositivo. c2pa-swift y c2pa-android envuelven c2pa-rs: calculan el hash del archivo y piden a la clave una firma ES256.",
      },
      {
        title: "Una comprobación posterior",
        body: "Quien quiera comprobar el archivo lo envía al servidor. c2pa-rs lee el manifiesto y confía en la autoridad de certificación de DeltaSync que emitió el certificado del dispositivo. Si cambia un byte, deja de ser válido.",
      },
    ],
    packagesTitle: "Paquetes",
    packagesIntro:
      "El mismo protocolo, en tres árboles. Las aplicaciones del teléfono capturan un JPEG e incrustan el manifiesto. El servidor emite el certificado y comprueba el archivo.",
    packages: [
      {
        title: "Servidor en Rust",
        body: "La autoridad que firma y la comprobación. Usa c2pa-rs para leer el manifiesto. En modo real comprueba App Attest o Play Integrity antes de emitir el certificado. La clave de la autoridad de certificación se crea en el servidor. No está en este repositorio.",
        badges: [
          { label: "Rust" },
          { label: "c2pa-rs", href: links.c2paRs },
        ],
      },
      {
        title: "SDK de iOS",
        body: "Swift, con c2pa-swift y App Attest. La clave de firma se crea en el Secure Enclave y no se exporta. El simulador no puede usar el Secure Enclave ni App Attest. Una compilación de depuración puede usar una clave ordinaria y lo indica. Eso no es una prueba de hardware, y solo funciona con un servidor en modo stub.",
        badges: [
          { label: "Swift" },
          { label: "c2pa-swift", href: links.c2paSwift },
          { label: "App Attest" },
        ],
      },
      {
        title: "SDK de Android",
        body: "Kotlin, con c2pa-android y Play Integrity. La aplicación prefiere StrongBox y registra el respaldo que obtuvo de verdad: StrongBox, almacén de hardware o almacén de software. Omitir Play Integrity solo sirve con un servidor en modo stub. El veredicto reconocido de Play corresponde a una instalación desde Play, no a una instalación hecha al margen.",
        badges: [
          { label: "Kotlin" },
          { label: "c2pa-android", href: links.c2paAndroid },
          { label: "Play Integrity" },
        ],
      },
    ],
    limitsTitle: "Lo que todavía no hace",
    limitsIntro:
      "La comprobación es real para un manifiesto firmado por esta autoridad de certificación. Faltan algunas piezas, y el software lo dice.",
    limits: [
      {
        title: "Fotos, no vídeo",
        body: "Las aplicaciones capturan un JPEG. No capturan vídeo. La comprobación también acepta PNG, WebP y MP4.",
      },
      {
        title: "El modo stub no atestigua",
        body: "Un servidor local usa el modo stub por defecto. Sigue exigiendo la prueba de que quien llama posee la clave, y no comprueba App Attest ni Play Integrity. La respuesta dice que la atestación no se verificó.",
      },
      {
        title: "El recibo de Apple no se envía",
        body: "App Attest en modo real comprueba la cadena de certificados y el nonce. El recibo todavía no se envía a Apple.",
      },
      {
        title: "Esta autoridad, no la lista pública",
        body: "La comprobación confía en la autoridad de certificación de DeltaSync para los manifiestos que emitió. No añade la lista pública de confianza de C2PA. Todavía no se incrustan una marca de tiempo de confianza ni una respuesta OCSP.",
      },
    ],
  },
  about: {
    metaTitle: "Sobre nosotros — DeltaSync",
    metaDescription:
      "Simply Software hace aplicaciones simples, baratas, seguras y fiables, sin anuncios, correos insistentes ni patrones adictivos.",
    title: "Sobre nosotros",
    lead: "Simply Software hace aplicaciones simples, baratas, seguras y fiables.",
    body: "Las aplicaciones no tienen anuncios, ni correos insistentes, ni patrones adictivos. DeltaSync es una de ellas: procedencia de medios en código abierto, un conjunto de SDKs y un protocolo construido sobre C2PA.",
    more: "Ver más de Simply Software",
  },
}

const dictionaries: Record<Locale, Dictionary> = { en, es }

export function getCopy(locale: Locale): Dictionary {
  return dictionaries[locale]
}

export function pageUrl(path = ""): string {
  return `${SITE}${path}`
}
