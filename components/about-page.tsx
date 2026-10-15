import { Button } from "@/components/ui/button"
import { links, type AboutCopy } from "@/lib/copy"

export function AboutPage({ copy }: { copy: AboutCopy }) {
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
      <div>
        <Button asChild>
          <a href={links.simply}>{copy.more}</a>
        </Button>
      </div>
    </main>
  )
}
