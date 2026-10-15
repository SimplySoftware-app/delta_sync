import type { Metadata } from "next"

import { AboutPage } from "@/components/about-page"
import { getCopy, pageUrl } from "@/lib/copy"
import { getLocale } from "@/lib/get-locale"

export async function generateMetadata(): Promise<Metadata> {
  const locale = await getLocale()
  const { about } = getCopy(locale)

  return {
    title: about.metaTitle,
    description: about.metaDescription,
    alternates: {
      canonical: pageUrl("/about"),
    },
    openGraph: {
      title: about.metaTitle,
      description: about.metaDescription,
      url: pageUrl("/about"),
      siteName: "DeltaSync",
      locale: locale === "es" ? "es_ES" : "en_US",
      type: "website",
    },
  }
}

export default async function Page() {
  const locale = await getLocale()
  const { about } = getCopy(locale)

  return <AboutPage copy={about} />
}
