import type { Metadata, Viewport } from "next"

import { SiteFooter } from "@/components/site-footer"
import { SiteHeader } from "@/components/site-header"
import { ThemeProvider } from "@/components/theme-provider"
import { getCopy, pageUrl } from "@/lib/copy"
import { getLocale } from "@/lib/get-locale"

import "./globals.css"

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#ffffff" },
    { media: "(prefers-color-scheme: dark)", color: "#0a0a0a" },
  ],
}

export async function generateMetadata(): Promise<Metadata> {
  const locale = await getLocale()
  const { home } = getCopy(locale)

  return {
    title: home.metaTitle,
    description: home.metaDescription,
    alternates: {
      canonical: pageUrl(),
      languages: {
        en: pageUrl(),
        es: pageUrl(),
        "x-default": pageUrl(),
      },
    },
    openGraph: {
      title: home.metaTitle,
      description: home.metaDescription,
      url: pageUrl(),
      siteName: "DeltaSync",
      locale: locale === "es" ? "es_ES" : "en_US",
      type: "website",
    },
    twitter: {
      card: "summary",
      title: home.metaTitle,
      description: home.metaDescription,
    },
  }
}

export default async function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  const locale = await getLocale()
  const { chrome } = getCopy(locale)

  return (
    <html lang={locale} className="h-full scroll-smooth" suppressHydrationWarning>
      <body className="flex min-h-full flex-col bg-background font-sans text-foreground antialiased">
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          enableSystem
          disableTransitionOnChange
        >
          <SiteHeader locale={locale} copy={chrome} />
          <div className="flex-1">{children}</div>
          <SiteFooter copy={chrome} />
        </ThemeProvider>
      </body>
    </html>
  )
}
