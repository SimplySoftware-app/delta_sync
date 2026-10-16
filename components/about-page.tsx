import { Button } from "@/components/ui/button"
import { links, type AboutCopy, type ChromeCopy } from "@/lib/copy"

export function AboutPage({
  copy,
  chrome,
}: {
  copy: AboutCopy
  chrome: ChromeCopy
}) {
  return (
    <main
      id="main"
      className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-4 py-12 sm:py-16"
    >
      <h1 className="max-w-3xl text-4xl font-medium tracking-tight text-balance text-foreground sm:text-5xl sm:leading-tight">
        {copy.title}
      </h1>
      <p className="max-w-2xl text-base leading-relaxed text-pretty text-foreground">
        {copy.lead}
      </p>
      <p className="max-w-2xl text-base leading-relaxed text-pretty text-muted-foreground">
        {copy.body}
      </p>
      <div className="flex flex-wrap gap-3">
        <Button asChild>
          <a href={links.simply}>{copy.more}</a>
        </Button>
        <Button asChild variant="outline">
          <a href={links.repo} target="_blank" rel="noopener noreferrer">
            {copy.code}
            <span className="sr-only"> ({chrome.opensInNewTab})</span>
          </a>
        </Button>
      </div>
    </main>
  )
}
