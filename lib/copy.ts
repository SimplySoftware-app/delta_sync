import type { Locale } from "@/lib/locale"

const SITE = "https://deltasync.simplysoftware.app"

export const links = {
  c2pa: "https://c2pa.org/",
  simply: "https://simplysoftware.app",
  contact: "info@simplysoftware.app",
  repo: "https://github.com/carlosedm10/delta_sync",
  verify:
    "https://github.com/carlosedm10/delta_sync/blob/main/server/README.md#verify-a-photo",
  serverDir: "https://github.com/carlosedm10/delta_sync/tree/main/server",
  iosDir: "https://github.com/carlosedm10/delta_sync/tree/main/ios",
  androidDir: "https://github.com/carlosedm10/delta_sync/tree/main/android",
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
  action?: { label: string; href: string }
}

type PackageCopy = {
  title: string
  body: string
  label: string
  href: string
  repoLabel: string
  repoHref: string
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
  cta: string
  secondaryCta: string
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
  code: string
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
    credit: "by Simply Software",
    opensInNewTab: "opens in a new tab",
  },
  home: {
    metaTitle: "DeltaSync — Open-source verifiable photo provenance",
    metaDescription:
      "Open-source SDKs and a protocol based on C2PA. A phone signs a photo with a key that stays on the device, and anyone can check that signature later.",
    title: "Open-source verifiable provenance for your photos.",
    leadBefore:
      "A photo signed on the phone, which anyone can check later. Based on the ",
    leadLink: "C2PA",
    leadAfter: " standard.",
    cta: "How to check a photo",
    secondaryCta: "View the code",
    pillarsTitle: "Three pillars",
    nameLine:
      "“Delta” is any change to the photo. “Sync” is the proof travelling with it. We issue the certificate, your phone signs the photo with it, and you spot any delta when you check the file.",
    diagramLabel: "Us, certificate, you: how the proof travels",
    edgeLabels: ["issues", "travels with the photo"],
    pillars: [
      {
        name: "Us",
        title: "Us",
        body: "DeltaSync checks the device and issues a certificate that lasts 15 minutes. The code is open: anyone can audit it.",
        action: { label: "View the code", href: links.repo },
      },
      {
        name: "Certificate",
        title: "Certificate",
        body: "The phone signs the photo with that certificate and a key that stays on the device, in the secure chip when there is one.",
      },
      {
        name: "You",
        title: "You",
        body: "You check the file yourself with the open-source verifier. If a single byte changes, you see it.",
        action: { label: "How to check a photo", href: links.verify },
      },
    ],
    packagesTitle: "Packages",
    packages: [
      {
        title: "Rust server",
        body: "Issues the certificate and checks the file.",
        label: "Based on: c2pa-rs",
        href: links.c2paRs,
        repoLabel: "Source: server/",
        repoHref: links.serverDir,
      },
      {
        title: "iOS SDK",
        body: "Swift, Secure Enclave and App Attest.",
        label: "Based on: c2pa-swift",
        href: links.c2paSwift,
        repoLabel: "Source: ios/",
        repoHref: links.iosDir,
      },
      {
        title: "Android SDK",
        body: "Kotlin, StrongBox and Play Integrity.",
        label: "Based on: c2pa-android",
        href: links.c2paAndroid,
        repoLabel: "Source: android/",
        repoHref: links.androidDir,
      },
    ],
    limitsTitle: "Honest limits",
    limitsIntro: "The check is real for manifests this authority signed. Current limits:",
    limits: [
      "Photos only: the apps capture a JPEG, not video.",
      "Locally (test mode) App Attest and Play Integrity are not verified; the response says so.",
      "Apple’s receipt is not sent yet.",
      "The authority is DeltaSync’s own, not the public C2PA trust list.",
      "No trusted timestamp or OCSP response is embedded yet.",
    ],
  },
  about: {
    metaTitle: "About us — DeltaSync",
    metaDescription:
      "Simply Software makes simple, cheap, secure, reliable apps, with no ads, nag emails or addictive patterns. DeltaSync is open source.",
    title: "About us",
    lead: "Simply Software makes simple, cheap, secure, reliable apps.",
    body: "The apps have no ads, no nag emails and no addictive patterns. DeltaSync is one of them: open-source verifiable photo provenance, a set of SDKs and a protocol based on C2PA.",
    more: "See more from Simply Software",
    code: "DeltaSync on GitHub",
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
    credit: "por Simply Software",
    opensInNewTab: "se abre en una pestaña nueva",
  },
  home: {
    metaTitle: "DeltaSync — Origen verificable de tus fotos en código abierto",
    metaDescription:
      "SDKs de código abierto y un protocolo basado en C2PA. El teléfono firma una foto con una clave que se queda en el dispositivo, y cualquiera puede comprobar esa firma después.",
    title: "Origen verificable de tus fotos, en código abierto.",
    leadBefore:
      "Una foto firmada en el teléfono que puedes comprobar después. Todo basado en el estándar ",
    leadLink: "C2PA",
    leadAfter: ".",
    cta: "Cómo comprobar una foto",
    secondaryCta: "Ver el código",
    pillarsTitle: "Tres pilares",
    nameLine:
      "«Delta» es cualquier cambio en la foto. «Sync» es que la prueba viaja con ella. Nosotros emitimos el certificado, tu teléfono firma la foto con él y tú detectas cualquier delta al comprobarla.",
    diagramLabel: "Nosotros, certificado, tú: cómo viaja la prueba",
    edgeLabels: ["emite", "viaja con la foto"],
    pillars: [
      {
        name: "Nosotros",
        title: "Nosotros",
        body: "DeltaSync comprueba el dispositivo y emite un certificado de 15 minutos. El código es abierto: cualquiera puede auditarlo.",
        action: { label: "Ver el código", href: links.repo },
      },
      {
        name: "Certificado",
        title: "Certificado",
        body: "El teléfono firma la foto con ese certificado y una clave que se queda en el dispositivo, en el chip seguro cuando lo hay.",
      },
      {
        name: "Tú",
        title: "Tú",
        body: "Compruebas el archivo tú mismo con el verificador de código abierto. Si cambia un solo byte, lo ves.",
        action: { label: "Cómo comprobar una foto", href: links.verify },
      },
    ],
    packagesTitle: "Paquetes",
    packages: [
      {
        title: "Servidor en Rust",
        body: "Emite el certificado y comprueba el archivo.",
        label: "Basado en: c2pa-rs",
        href: links.c2paRs,
        repoLabel: "Código: server/",
        repoHref: links.serverDir,
      },
      {
        title: "SDK de iOS",
        body: "Swift, Secure Enclave y App Attest.",
        label: "Basado en: c2pa-swift",
        href: links.c2paSwift,
        repoLabel: "Código: ios/",
        repoHref: links.iosDir,
      },
      {
        title: "SDK de Android",
        body: "Kotlin, StrongBox y Play Integrity.",
        label: "Basado en: c2pa-android",
        href: links.c2paAndroid,
        repoLabel: "Código: android/",
        repoHref: links.androidDir,
      },
    ],
    limitsTitle: "Límites honestos",
    limitsIntro:
      "La comprobación es real para los manifiestos que firmó esta autoridad. Límites actuales:",
    limits: [
      "Solo fotos: las aplicaciones capturan un JPEG, no vídeo.",
      "En local (modo de prueba) no se verifica App Attest ni Play Integrity; la respuesta lo indica.",
      "El recibo de Apple todavía no se envía.",
      "La autoridad es la propia de DeltaSync, no la lista pública de confianza de C2PA.",
      "Todavía no se incrusta una marca de tiempo de confianza ni una respuesta OCSP.",
    ],
  },
  about: {
    metaTitle: "Sobre nosotros — DeltaSync",
    metaDescription:
      "Simply Software hace aplicaciones simples, baratas, seguras y fiables, sin anuncios, correos insistentes ni patrones adictivos. DeltaSync es de código abierto.",
    title: "Sobre nosotros",
    lead: "Simply Software hace aplicaciones simples, baratas, seguras y fiables.",
    body: "Las aplicaciones no tienen anuncios, ni correos insistentes, ni patrones adictivos. DeltaSync es una de ellas: origen verificable de fotos en código abierto, un conjunto de SDKs y un protocolo basado en C2PA.",
    more: "Ver más de Simply Software",
    code: "DeltaSync en GitHub",
  },
}

const dictionaries: Record<Locale, Dictionary> = { en, es }

export function getCopy(locale: Locale): Dictionary {
  return dictionaries[locale]
}

export function pageUrl(path = ""): string {
  return `${SITE}${path}`
}
