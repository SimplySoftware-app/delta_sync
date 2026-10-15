import { cache } from "react"
import { cookies, headers } from "next/headers"

import { resolveLocale, type Locale, LOCALE_COOKIE } from "@/lib/locale"

export const getLocale = cache(async (): Promise<Locale> => {
  const cookieStore = await cookies()
  const headerStore = await headers()
  return resolveLocale(
    cookieStore.get(LOCALE_COOKIE)?.value,
    headerStore.get("accept-language"),
  )
})
