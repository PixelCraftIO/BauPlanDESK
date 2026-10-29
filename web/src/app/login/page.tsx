import type { Metadata } from "next";
import { headers } from "next/headers";
import { redirect } from "next/navigation";
import { aktuelleSitzung } from "@/lib/auth";
import { LoginFormular } from "./login-formular";

export const metadata: Metadata = { title: "Anmelden" };

// Nur relative Ziele innerhalb der Seite zulassen (kein offener Redirect).
function sicheresZiel(weiter: string | string[] | undefined) {
  return typeof weiter === "string" && weiter.startsWith("/") && !weiter.startsWith("//") ? weiter : "/";
}

export default async function LoginSeite({ searchParams }: { searchParams: Promise<{ weiter?: string | string[] }> }) {
  const ziel = sicheresZiel((await searchParams).weiter);
  if (await aktuelleSitzung(await headers())) redirect(ziel);

  return (
    <main className="login">
      <div className="login-kasten">
        <div className="login-kopf">
          <Marke />
        </div>
        <div className="login-inhalt">
          <h1>Anmelden</h1>
          <p className="hinweis">Dieser Bereich ist derzeit nur für freigeschaltete Konten zugänglich.</p>
          <LoginFormular ziel={ziel} />
        </div>
      </div>
    </main>
  );
}

function Marke() {
  return (
    <span className="marke">
      <svg viewBox="0 0 32 32" aria-hidden="true">
        <path d="M7 24V13l9-6 9 6v11h-6v-7h-6v7z" fill="currentColor" />
      </svg>
      BauPlanIO
    </span>
  );
}
