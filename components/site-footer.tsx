import { Separator } from "@/components/ui/separator"
import { links, type ChromeCopy } from "@/lib/copy"

export function SiteFooter({ copy }: { copy: ChromeCopy }) {
  return (
    <footer>
      <Separator />
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-2 px-4 py-8">
        <p className="text-sm font-medium text-foreground">{copy.product}</p>
        <a
          href={links.simply}
          className="w-fit rounded-[10px] text-sm text-foreground underline-offset-4 outline-none hover:underline focus-visible:ring-3 focus-visible:ring-ring/50"
        >
          {copy.credit}
        </a>
        <a
          href={`mailto:${links.contact}`}
          className="w-fit rounded-[10px] text-sm text-muted-foreground underline-offset-4 outline-none hover:underline focus-visible:ring-3 focus-visible:ring-ring/50"
        >
          {links.contact}
        </a>
      </div>
    </footer>
  )
}
