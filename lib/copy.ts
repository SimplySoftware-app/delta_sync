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
      "“Delta” is every change a photo goes through after it is taken. “Sync” is the proof travelling with it: we issue the certificate, it signs each delta, and you sync the proof when you check the file.",
    diagramLabel: "Us, certificate, you: how the proof travels",
    edgeLabels: ["issues", "signs"],
    pillars: [
      {
        name: "Us",
        title: "Us",
        body: "DeltaSync checks the device and issues a certificate that lasts 15 minutes. The code is open, so anyone can audit it.",
      },
      {
        name: "Certificate",
        title: "Certificate",
        body: "The phone signs the photo with that certificate and a key that stays on the device, in the secure chip when there is one.",
      },
      {
        name: "You",
        title: "You",
        body: "You check the file yourself. If a single byte changes, you see it.",
      },
    ],
    packagesTitle: "Packages",
    packages: [
      {
        title: "Rust server",
        body: "Built on c2pa-rs. Issues the certificate and checks the file.",
        label: "c2pa-rs",
        href: links.c2paRs,
      },
      {
        title: "iOS SDK",
        body: "Built on c2pa-swift. Swift, Secure Enclave and App Attest.",
        label: "c2pa-swift",
        href: links.c2paSwift,
      },
      {
        title: "Android SDK",
        body: "Built on c2pa-android. Kotlin, StrongBox and Play Integrity.",
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
    tagline: "Origen verificable de tus fotos en código abierto.",
    credit: "por Simply Software",
    opensInNewTab: "se abre en una pestaña nueva",
  },
  home: {
    metaTitle: "DeltaSync — Origen verificable de tus fotos en código abierto",
    metaDescription:
      "SDKs de código abierto y un protocolo sobre C2PA. El teléfono firma una foto con una clave que se queda en el dispositivo, y cualquiera puede comprobar esa firma después.",
    title: "Origen verificable de tus fotos, en código abierto.",
    leadBefore:
      "Una foto firmada en el teléfono que tú puedes comprobar después. Construido sobre ",
    leadLink: "C2PA",
    leadAfter: ".",
    pillarsTitle: "Tres pilares",
    nameLine:
      "«Delta» es cada cambio que sufre una foto desde que se toma. «Sync» es que la prueba viaja con ella: nosotros emitimos el certificado, él firma cada delta y tú sincronizas la prueba al comprobar el archivo.",
    diagramLabel: "Nosotros, certificado, tú: cómo viaja la prueba",
    edgeLabels: ["emite", "firma"],
    pillars: [
      {
        name: "Nosotros",
        title: "Nosotros",
        body: "DeltaSync comprueba el dispositivo y emite un certificado de 15 minutos. El código es abierto: cualquiera puede auditarlo.",
      },
      {
        name: "Certificado",
        title: "Certificado",
        body: "El teléfono firma la foto con ese certificado y una clave que se queda en el dispositivo, en el chip seguro cuando lo hay.",
      },
      {
        name: "Tú",
        title: "Tú",
        body: "Compruebas el archivo tú mismo. Si cambia un solo byte, lo ves.",
      },
    ],
    packagesTitle: "Paquetes",
    packages: [
      {
        title: "Servidor en Rust",
        body: "Basado en c2pa-rs. Emite el certificado y comprueba el archivo.",
        label: "c2pa-rs",
        href: links.c2paRs,
      },
      {
        title: "SDK de iOS",
        body: "Basado en c2pa-swift. Swift, Secure Enclave y App Attest.",
        label: "c2pa-swift",
        href: links.c2paSwift,
      },
      {
        title: "SDK de Android",
        body: "Basado en c2pa-android. Kotlin, StrongBox y Play Integrity.",
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
    body: "Las aplicaciones no tienen anuncios, ni correos insistentes, ni patrones adictivos. DeltaSync es una de ellas: origen verificable de fotos en código abierto, un conjunto de SDKs y un protocolo construido sobre C2PA.",
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
