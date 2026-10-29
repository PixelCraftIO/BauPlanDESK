import path from "node:path";
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Eigenständiger Server für das Docker-Image (siehe Dockerfile).
  output: "standalone",
  // web/ ist ein eigenes npm-Projekt im Repository; nicht die Wurzel nehmen.
  outputFileTracingRoot: path.join(__dirname),
  turbopack: { root: path.join(__dirname) },
  // Keine Bildoptimierung: sharp (libvips, LGPL) ist bewusst nicht installiert.
  images: { unoptimized: true },
  serverExternalPackages: ["better-sqlite3"],
  poweredByHeader: false,
  async headers() {
    return [
      {
        source: "/:path*",
        headers: [
          // Solange die Seite nur hinter dem Login steht, nicht indexieren.
          { key: "X-Robots-Tag", value: "noindex, nofollow" },
          { key: "X-Frame-Options", value: "DENY" },
          { key: "X-Content-Type-Options", value: "nosniff" },
          { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
        ],
      },
    ];
  },
};

export default nextConfig;
