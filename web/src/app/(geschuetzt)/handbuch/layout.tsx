import { HandbuchNavigation } from "./navigation";
import { kapitelListe } from "@/lib/handbuch";

export default async function HandbuchLayout({ children }: { children: React.ReactNode }) {
  const liste = await kapitelListe();
  return (
    <div className="breite handbuch">
      <aside className="handbuch-seitenleiste">
        <HandbuchNavigation kapitel={liste} />
        <a className="knopf klein pdf-knopf" href="/handbuch/pdf" target="_blank" rel="noopener">
          Handbuch als PDF
        </a>
      </aside>
      <div className="handbuch-inhalt">{children}</div>
    </div>
  );
}
