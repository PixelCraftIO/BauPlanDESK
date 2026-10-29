"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import type { Kapitel } from "@/lib/handbuch";

export function HandbuchNavigation({ kapitel }: { kapitel: Kapitel[] }) {
  const pfad = usePathname();
  return (
    <nav aria-label="Kapitel des Handbuchs">
      <p className="seitenleiste-titel">Benutzerhandbuch</p>
      <ol className="kapitel-liste">
        <li>
          <Link href="/handbuch" aria-current={pfad === "/handbuch" ? "page" : undefined}>
            Übersicht
          </Link>
        </li>
        {kapitel.map((k) => {
          const ziel = `/handbuch/${k.slug}`;
          return (
            <li key={k.slug}>
              <Link href={ziel} aria-current={pfad === ziel ? "page" : undefined}>
                <span className="kapitel-nr">{k.nummer}</span> {k.titel}
              </Link>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
