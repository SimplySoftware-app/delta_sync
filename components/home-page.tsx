import { Server, Smartphone, TabletSmartphone } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { links, type ChromeCopy, type HomeCopy } from "@/lib/copy"

const packageIcons = [Server, Smartphone, TabletSmartphone] as const

export function HomePage({
  copy,
  chrome,
}: {
  copy: HomeCopy
  chrome: ChromeCopy
}) {
  return (
    <main
      id="main"
      className="mx-auto flex w-full max-w-5xl flex-col gap-16 px-4 py-12 sm:py-16"
    >
      <section className="flex max-w-3xl flex-col gap-4">
        <Badge variant="outline" className="rounded-[10px]">
          {copy.badge}
        </Badge>
        <h1 className="text-4xl font-medium tracking-tight text-balance text-foreground sm:text-5xl sm:leading-tight">
          {copy.title}
        </h1>
        <p className="text-base leading-relaxed text-pretty text-muted-foreground">
          {copy.leadBefore}
          <a
            href={links.c2pa}
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-[10px] text-foreground underline underline-offset-4 outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
          >
            {copy.leadLink}
            <span className="sr-only"> ({chrome.opensInNewTab})</span>
          </a>
          {copy.leadAfter}
        </p>
        <p className="text-base leading-relaxed text-pretty text-muted-foreground">
          {copy.packagesLine}
        </p>
      </section>

      <section id="how" className="flex scroll-mt-24 flex-col gap-4">
        <h2 className="text-2xl font-medium tracking-tight text-foreground">
          {copy.howTitle}
        </h2>
        <p className="max-w-2xl text-base leading-relaxed text-pretty text-muted-foreground">
          {copy.howIntro}
        </p>
        <ol className="flex flex-col gap-4">
          {copy.steps.map((step, index) => (
            <li key={step.title}>
              <Card>
                <CardHeader>
                  <div className="flex items-center gap-3">
                    <span className="flex size-10 shrink-0 items-center justify-center rounded-[10px] border border-border bg-muted text-sm font-medium text-foreground">
                      {index + 1}
                    </span>
                    <CardTitle className="text-base font-medium">
                      <h3>{step.title}</h3>
                    </CardTitle>
                  </div>
                  <CardDescription className="text-sm leading-relaxed text-pretty">
                    {step.body}
                  </CardDescription>
                </CardHeader>
              </Card>
            </li>
          ))}
        </ol>
      </section>

      <section id="packages" className="flex scroll-mt-24 flex-col gap-4">
        <h2 className="text-2xl font-medium tracking-tight text-foreground">
          {copy.packagesTitle}
        </h2>
        <p className="max-w-2xl text-base leading-relaxed text-pretty text-muted-foreground">
          {copy.packagesIntro}
        </p>
        <ul className="grid grid-cols-1 gap-4 md:grid-cols-3">
          {copy.packages.map((pkg, index) => {
            const Icon = packageIcons[index] ?? Server
            return (
              <li key={pkg.title} className="h-full">
                <Card className="h-full">
                  <CardHeader>
                    <span className="flex size-10 items-center justify-center rounded-[10px] border border-border bg-muted text-foreground">
                      <Icon className="size-4" aria-hidden="true" />
                    </span>
                    <CardTitle className="text-base font-medium">
                      <h3>{pkg.title}</h3>
                    </CardTitle>
                    <CardDescription className="text-sm leading-relaxed text-pretty">
                      {pkg.body}
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="flex flex-wrap gap-2">
                    {pkg.badges.map((badge) =>
                      badge.href ? (
                        <Badge
                          key={badge.label}
                          variant="outline"
                          className="rounded-[10px]"
                          asChild
                        >
                          <a
                            href={badge.href}
                            target="_blank"
                            rel="noopener noreferrer"
                          >
                            {badge.label}
                            <span className="sr-only">
                              {" "}
                              ({chrome.opensInNewTab})
                            </span>
                          </a>
                        </Badge>
                      ) : (
                        <Badge
                          key={badge.label}
                          variant="outline"
                          className="rounded-[10px]"
                        >
                          {badge.label}
                        </Badge>
                      ),
                    )}
                  </CardContent>
                </Card>
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
        <ul className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          {copy.limits.map((limit) => (
            <li key={limit.title}>
              <Card className="h-full">
                <CardHeader>
                  <CardTitle className="text-base font-medium">
                    <h3>{limit.title}</h3>
                  </CardTitle>
                  <CardDescription className="text-sm leading-relaxed text-pretty">
                    {limit.body}
                  </CardDescription>
                </CardHeader>
              </Card>
            </li>
          ))}
        </ul>
      </section>
    </main>
  )
}
