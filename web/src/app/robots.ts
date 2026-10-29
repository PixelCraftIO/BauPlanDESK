import type { MetadataRoute } from "next";

// Vorerst nicht öffentlich: Suchmaschinen sollen nichts aufnehmen.
export default function robots(): MetadataRoute.Robots {
  return { rules: { userAgent: "*", disallow: "/" } };
}
