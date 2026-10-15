export const LOCALES = ["en", "es"] as const

export type Locale = (typeof LOCALES)[number]

export const LOCALE_COOKIE = "locale"

export function isLocale(value: string | undefined): value is Locale {
  return value === "en" || value === "es"
}

export function resolveLocale(
  cookieValue: string | undefined,
  acceptLanguage: string | null,
): Locale {
  if (isLocale(cookieValue)) return cookieValue

  const tags = (acceptLanguage ?? "")
    .split(",")
    .map((part) => part.split(";")[0]?.trim().toLowerCase() ?? "")
    .filter(Boolean)

  for (const tag of tags) {
    if (tag.startsWith("es")) return "es"
    if (tag.startsWith("en")) return "en"
  }

  return "en"
}
