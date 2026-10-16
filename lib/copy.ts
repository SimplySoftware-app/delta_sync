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

export type Pillar = {
  name: string
  title: string
  short: string
  body: string
}

type PackageCopy = {
  title: string
  body: string
  label: string
  href: string
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
  title: string
  leadBefore: string
  leadLink: string
  leadAfter: string
  pillarsTitle: string
  nameLine: string
  diagramLabel: string
  edgeLabels: [string, string]
  pillars: [Pillar, Pillar, Pillar]
  packagesTitle: string
  packages: PackageCopy[]
  limitsTitle: string
  limitsIntro: string
  limits: string[]
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
      { href: "/#pillars", label: "Pillars" },
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
    title: "Open-source media provenance.",
    leadBefore:
      "A photo signed on the phone, which anyone can check later. Built on ",
    leadLink: "C2PA",
    leadAfter: ".",
    pillarsTitle: "Three pillars",
    nameLine:
      "“Delta” is every change a photo goes through after it is taken. “Sync” is the proof travelling with it.",
    diagramLabel: "Certificate, us, you: how the proof travels",
    edgeLabels: ["signs", "verifies"],
    pillars: [
      {
        name: "Certificate",
        title: "Certificate",
        short: "Signs on the phone",
        body: "The phone signs the photo with a key that stays on the device, in the secure chip when there is one, and a certificate that lasts 15 minutes.",
      },
      {
        name: "Us",
        title: "Us",
        short: "Issues the certificate",
        body: "DeltaSync issues that certificate after checking the device. The code is open, so anyone can audit it.",
      },
      {
        name: "You",
        title: "You",
        short: "Checks the file",
        body: "Anyone can check the file. If a single byte changes, it shows.",
      },
    ],
    packagesTitle: "Packages",
    packages: [
      {
        title: "Rust server",
        body: "Issues the certificate and checks the file.",
        label: "c2pa-rs",
        href: links.c2paRs,
      },
      {
        title: "iOS SDK",
        body: "Swift, Secure Enclave and App Attest.",
        label: "c2pa-swift",
        href: links.c2paSwift,
      },
      {
        title: "Android SDK",
        body: "Kotlin, StrongBox and Play Integrity.",
        label: "c2pa-android",
        href: links.c2paAndroid,
      },
    ],
    limitsTitle: "Honest limits",
    limitsIntro: "The check is real for manifests this authority signed. Still missing:",
    limits: [
      "Photos only: the apps capture a JPEG, not video.",
      "Stub mode does not attest: a local server skips App Attest and Play Integrity, and the response says so.",
      "Apple’s receipt is not sent yet.",
      "The authority is DeltaSync’s own, not the public C2PA trust list.",
      "No trusted timestamp or OCSP response is embedded yet.",
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
      { href: "/#pillars", label: "Pilares" },
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
    title: "Procedencia de medios, en código abierto.",
    leadBefore:
      "Una foto firmada en el teléfono, que cualquiera puede comprobar después. Construido sobre ",
    leadLink: "C2PA",
    leadAfter: ".",
    pillarsTitle: "Tres pilares",
    nameLine:
      "«Delta» es cada cambio que sufre una foto desde que se toma. «Sync» es que la prueba viaja con ella.",
    diagramLabel: "Certificado, nosotros, tú: cómo viaja la prueba",
    edgeLabels: ["firma", "verifica"],
    pillars: [
      {
        name: "Certificado",
        title: "Certificado",
        short: "Firma en el teléfono",
        body: "El teléfono firma la foto con una clave que se queda en el dispositivo, en el chip seguro cuando lo hay, y un certificado de 15 minutos.",
      },
      {
        name: "Nosotros",
        title: "Nosotros",
        short: "Emitimos el certificado",
        body: "DeltaSync emite ese certificado tras comprobar el dispositivo. El código es abierto: cualquiera puede auditarlo.",
      },
      {
        name: "Tú",
        title: "Tú",
        short: "Compruebas el archivo",
        body: "Cualquiera puede comprobar el archivo. Si cambia un byte, se nota.",
      },
    ],
    packagesTitle: "Paquetes",
    packages: [
      {
        title: "Servidor en Rust",
        body: "Emite el certificado y comprueba el archivo.",
        label: "c2pa-rs",
        href: links.c2paRs,
      },
      {
        title: "SDK de iOS",
        body: "Swift, Secure Enclave y App Attest.",
        label: "c2pa-swift",
        href: links.c2paSwift,
      },
      {
        title: "SDK de Android",
        body: "Kotlin, StrongBox y Play Integrity.",
        label: "c2pa-android",
        href: links.c2paAndroid,
      },
    ],
    limitsTitle: "Límites honestos",
    limitsIntro:
      "La comprobación es real para los manifiestos que firmó esta autoridad. Todavía falta:",
    limits: [
      "Solo fotos: las aplicaciones capturan un JPEG, no vídeo.",
      "El modo stub no atestigua: un servidor local no comprueba App Attest ni Play Integrity, y la respuesta lo dice.",
      "El recibo de Apple todavía no se envía.",
      "La autoridad es la propia de DeltaSync, no la lista pública de confianza de C2PA.",
      "Todavía no se incrusta una marca de tiempo de confianza ni una respuesta OCSP.",
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
