import {
  ArrowDown,
  ArrowRight,
  BadgeCheck,
  ScanSearch,
  Server,
  ShieldCheck,
  Smartphone,
  TabletSmartphone,
} from "lucide-react"

import { Button } from "@/components/ui/button"
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { links, type ChromeCopy, type HomeCopy } from "@/lib/copy"

const pillarIcons = [ShieldCheck, BadgeCheck, ScanSearch] as const
const packageIcons = [Server, Smartphone, TabletSmartphone] as const

const linkClass =
  "rounded-[10px] text-foreground underline underline-offset-4 outline-none focus-visible:ring-3 focus-visible:ring-ring/50"

export function HomePage({
  copy,
  chrome,
}: {
  copy: HomeCopy
  chrome: ChromeCopy
}) {
  const newTab = <span className="sr-only"> ({chrome.opensInNewTab})</span>
  return (
    <main
      id="main"
      className="mx-auto flex w-full max-w-5xl flex-col gap-16 px-4 py-12 sm:py-16"
    >
      <section className="flex max-w-3xl flex-col gap-4">
        <h1 className="text-4xl font-medium tracking-tight text-balance text-foreground sm:text-5xl sm:leading-tight">
          {copy.title}
        </h1>
        <p className="text-base leading-relaxed text-pretty text-muted-foreground">
          {copy.leadBefore}
          <a
            href={links.c2pa}
            target="_blank"
            rel="noopener noreferrer"
            className={linkClass}
          >
            {copy.leadLink}
            {newTab}
          </a>
          {copy.leadAfter}
        </p>
        <div className="pt-2">
          <Button asChild>
            <a href={links.verify} target="_blank" rel="noopener noreferrer">
              {copy.cta}
              {newTab}
            </a>
          </Button>
        </div>
      </section>

      <section id="pillars" className="flex scroll-mt-24 flex-col gap-6">
        <div className="flex flex-col gap-2">
          <h2 className="text-2xl font-medium tracking-tight text-foreground">
            {copy.pillarsTitle}
          </h2>
          <p className="max-w-2xl text-base leading-relaxed text-pretty text-muted-foreground">
            {copy.nameLine}
          </p>
        </div>
        <ol
          aria-label={copy.diagramLabel}
          className="flex flex-col gap-2 md:grid md:grid-cols-3 md:gap-20"
        >
          {copy.pillars.map((pillar, index) => {
            const Icon = pillarIcons[index] ?? ShieldCheck
            return (
              <li key={pillar.name} className="relative flex flex-col gap-2">
                {index > 0 && (
                  <div
                    aria-hidden="true"
                    className="flex flex-row items-center justify-center gap-2 py-1 text-sm font-medium text-muted-foreground md:absolute md:top-1/2 md:right-full md:w-20 md:-translate-y-1/2 md:flex-col md:gap-1 md:py-0 md:text-center md:text-xs"
                  >
                    <span>{copy.edgeLabels[index - 1]}</span>
                    <ArrowDown className="size-4 md:hidden" />
                    <ArrowRight className="hidden size-4 md:block" />
                  </div>
                )}
                <Card className="h-full">
                  <CardHeader>
                    <CardTitle className="text-base font-medium">
                      <h3 className="flex items-center gap-3">
                        <span className="flex size-10 shrink-0 items-center justify-center rounded-[10px] border border-border bg-muted text-foreground">
                          <Icon className="size-5" aria-hidden="true" />
                        </span>
                        <span>{pillar.title}</span>
                      </h3>
                    </CardTitle>
                    <CardDescription className="text-sm leading-relaxed text-pretty">
                      {pillar.body}
                    </CardDescription>
                    {pillar.action && (
                      <a
                        href={pillar.action.href}
                        target="_blank"
                        rel="noopener noreferrer"
                        className={`${linkClass} w-fit text-sm`}
                      >
                        {pillar.action.label}
                        {newTab}
                      </a>
                    )}
                  </CardHeader>
                </Card>
              </li>
            )
          })}
        </ol>
      </section>

      <section id="packages" className="flex scroll-mt-24 flex-col gap-4">
        <h2 className="text-2xl font-medium tracking-tight text-foreground">
          {copy.packagesTitle}
        </h2>
        <ul className="flex flex-col divide-y divide-border rounded-[16px] border border-border">
          {copy.packages.map((pkg, index) => {
            const Icon = packageIcons[index] ?? Server
            return (
              <li
                key={pkg.title}
                className="flex flex-col gap-1 px-4 py-3 sm:flex-row sm:items-center sm:gap-4"
              >
                <span className="flex items-center gap-3 sm:w-56">
                  <Icon
                    className="size-4 shrink-0 text-muted-foreground"
                    aria-hidden="true"
                  />
                  <span className="text-sm font-medium text-foreground">
                    {pkg.title}
                  </span>
                </span>
                <span className="flex-1 text-sm text-muted-foreground">
                  {pkg.body}
                </span>
                <a
                  href={pkg.href}
                  target="_blank"
                  rel="noopener noreferrer"
                  className={`${linkClass} w-fit text-sm`}
                >
                  {pkg.label}
                  {newTab}
                </a>
              </li>
            )
          })}
        </ul>
      </section>

      <section id="limits" className="flex scroll-mt-24 flex-col gap-4">
        <h2 className="text-2xl font-medium tracking-tight text-foreground">
          {copy.limitsTitle}
        </h2>
        <p className="max-w-2xl text-base leading-relaxed text-pretty text-muted-foreground">
          {copy.limitsIntro}
        </p>
        <ul className="flex max-w-2xl list-disc flex-col gap-2 pl-5 text-base leading-relaxed text-pretty text-foreground/80 marker:text-muted-foreground">
          {copy.limits.map((limit) => (
            <li key={limit}>{limit}</li>
          ))}
        </ul>
      </section>
    </main>
  )
}
