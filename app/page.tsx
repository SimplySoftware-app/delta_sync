import { HomePage } from "@/components/home-page"
import { getCopy } from "@/lib/copy"
import { getLocale } from "@/lib/get-locale"

export default async function Page() {
  const locale = await getLocale()
  const copy = getCopy(locale)

  return <HomePage copy={copy.home} chrome={copy.chrome} />
}
