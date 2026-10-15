"use client"

import { useState, useSyncExternalStore } from "react"
import Link from "next/link"
import { usePathname, useRouter } from "next/navigation"
import { Languages, Menu, Monitor, Moon, Sun, SunMoon } from "lucide-react"
import { useTheme } from "next-themes"

import { Logo } from "@/components/logo"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Sheet,
  SheetClose,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet"
import type { ChromeCopy } from "@/lib/copy"
import { LOCALE_COOKIE, type Locale } from "@/lib/locale"

const THEMES = ["system", "light", "dark"] as const

function itemHref(href: string, pathname: string) {
  if (pathname === "/" && href.startsWith("/#")) return href.slice(1)
  return href
}

function ThemeToggle({ copy }: { copy: ChromeCopy }) {
  const { theme, setTheme } = useTheme()
  const selected = useSyncExternalStore(
    () => () => {},
    () => theme ?? "system",
    () => "system",
  )
  const labels = {
    system: copy.system,
    light: copy.light,
    dark: copy.dark,
  } as const
  const icons = {
    system: Monitor,
    light: Sun,
    dark: Moon,
  } as const

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="size-11"
          aria-label={copy.theme}
        >
          <SunMoon />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuRadioGroup
          value={selected}
          onValueChange={setTheme}
        >
          {THEMES.map((value) => {
            const Icon = icons[value]
            return (
              <DropdownMenuRadioItem key={value} value={value}>
                <Icon aria-hidden="true" />
                {labels[value]}
              </DropdownMenuRadioItem>
            )
          })}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function LanguageSwitcher({
  locale,
  copy,
}: {
  locale: Locale
  copy: ChromeCopy
}) {
  const router = useRouter()

  function choose(next: Locale) {
    if (next === locale) return
    const secure = window.location.protocol === "https:" ? "; Secure" : ""
    document.cookie = `${LOCALE_COOKIE}=${next}; Path=/; Max-Age=31536000; SameSite=Lax${secure}`
    router.refresh()
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="size-11"
          aria-label={copy.language}
        >
          <Languages />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuRadioGroup
          value={locale}
          onValueChange={(value) => choose(value as Locale)}
        >
          <DropdownMenuRadioItem value="en" lang="en">
            {copy.english}
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem value="es" lang="es">
            {copy.spanish}
          </DropdownMenuRadioItem>
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

export function SiteHeader({
  locale,
  copy,
}: {
  locale: Locale
  copy: ChromeCopy
}) {
  const pathname = usePathname()
  const [menuOpen, setMenuOpen] = useState(false)

  return (
    <header className="sticky top-0 z-10 border-b border-border bg-background">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-50 focus:rounded-[10px] focus:bg-primary focus:px-3 focus:py-2 focus:text-sm focus:text-primary-foreground"
      >
        {copy.skip}
      </a>
      <div className="mx-auto flex w-full max-w-5xl items-center gap-3 px-4 py-2">
        <Link
          href="/"
          className="flex min-h-11 items-center gap-2 rounded-[10px] text-sm font-medium text-foreground outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
        >
          <Logo className="shrink-0" />
          <span className="whitespace-nowrap">{copy.product}</span>
        </Link>
        <nav aria-label={copy.navLabel} className="hidden md:block">
          <ul className="flex items-center gap-1">
            {copy.nav.map((item) => {
              const href = itemHref(item.href, pathname)
              const current = item.href === "/about" && pathname === "/about"
              return (
                <li key={item.href}>
                  <Link
                    href={href}
                    aria-current={current ? "page" : undefined}
                    className="inline-flex min-h-11 items-center rounded-[10px] px-2.5 text-sm text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 aria-[current=page]:text-foreground"
                  >
                    {item.label}
                  </Link>
                </li>
              )
            })}
          </ul>
        </nav>
        <div className="ml-auto flex items-center gap-1">
          <ThemeToggle copy={copy} />
          <LanguageSwitcher locale={locale} copy={copy} />
          <Sheet open={menuOpen} onOpenChange={setMenuOpen}>
            <SheetTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="size-11 md:hidden"
                aria-label={copy.openMenu}
              >
                <Menu />
              </Button>
            </SheetTrigger>
            <SheetContent side="right" showCloseButton={false}>
              <SheetHeader>
                <SheetTitle>{copy.menuTitle}</SheetTitle>
                <SheetDescription className="sr-only">
                  {copy.navLabel}
                </SheetDescription>
              </SheetHeader>
              <nav aria-label={copy.navLabel} className="px-4">
                <ul className="flex flex-col">
                  {copy.nav.map((item) => (
                    <li key={item.href}>
                      <SheetClose asChild>
                        <Link
                          href={itemHref(item.href, pathname)}
                          onClick={() => setMenuOpen(false)}
                          className="flex min-h-11 items-center rounded-[10px] px-2 text-base text-foreground outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
                        >
                          {item.label}
                        </Link>
                      </SheetClose>
                    </li>
                  ))}
                </ul>
              </nav>
              <SheetClose asChild>
                <Button variant="outline" className="mx-4 mt-auto mb-4">
                  {copy.closeMenu}
                </Button>
              </SheetClose>
            </SheetContent>
          </Sheet>
        </div>
      </div>
    </header>
  )
}
