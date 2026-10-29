import Link from "next/link";
import { headers } from "next/headers";
import { redirect } from "next/navigation";
import { aktuelleSitzung } from "@/lib/auth";
import { Abmelden } from "./abmelden";

// Zweite Prüfung neben proxy.ts: Jede geschützte Seite rendert nur mit Sitzung.
export default async function GeschuetztesLayout({ children }: { children: React.ReactNode }) {
  const sitzung = await aktuelleSitzung(await headers());
  if (!sitzung) redirect("/login");

  return (
    <>
      <header className="kopfleiste">
        <div className="breite kopfleiste-inhalt">
          <Link href="/" className="marke">
            <svg viewBox="0 0 32 32" aria-hidden="true">
              <path d="M7 24V13l9-6 9 6v11h-6v-7h-6v7z" fill="currentColor" />
            </svg>
            BauPlanIO
          </Link>
          <nav className="hauptnavigation">
            <Link href="/#funktionen" className="nur-breit">Funktionen</Link>
            <Link href="/#ablauf" className="nur-breit">Ablauf</Link>
            <Link href="/handbuch">Handbuch</Link>
          </nav>
          <div className="konto">
            <span className="konto-name" title={sitzung.user.email}>
              {sitzung.user.name}
            </span>
            <Abmelden />
          </div>
        </div>
      </header>
      {children}
      <footer className="fussleiste">
        <div className="breite">
          <span>© {new Date().getFullYear()} BauPlanIO</span>
          <span>Interne Vorschau – nicht öffentlich</span>
        </div>
      </footer>
    </>
  );
}
