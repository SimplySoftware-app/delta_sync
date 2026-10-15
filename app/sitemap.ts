import type { MetadataRoute } from "next";

import { pageUrl } from "@/lib/copy";

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    {
      url: pageUrl(),
      changeFrequency: "weekly",
      priority: 1,
    },
    {
      url: pageUrl("/about"),
      changeFrequency: "monthly",
      priority: 0.6,
    },
  ];
}
