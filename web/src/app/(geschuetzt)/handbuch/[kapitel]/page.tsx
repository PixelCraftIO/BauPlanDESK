import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { kapitel, kapitelListe } from "@/lib/handbuch";

type Parameter = { params: Promise<{ kapitel: string }> };

export async function generateMetadata({ params }: Parameter): Promise<Metadata> {
  const seite = await kapitel((await params).kapitel);
  return { title: seite ? `${seite.nummer} ${seite.titel}` : "Handbuch" };
}

export default async function KapitelSeite({ params }: Parameter) {
  const slug = (await params).kapitel;
  const seite = await kapitel(slug);
  if (!seite) notFound();

  const liste = await kapitelListe();
  const index = liste.findIndex((k) => k.slug === slug);
  const vorher = liste[index - 1];
  const nachher = liste[index + 1];

  return (
    <>
      {seite.abschnitte.length > 1 && (
        <nav className="abschnitte" aria-label="Auf dieser Seite">
          <p className="seitenleiste-titel">Auf dieser Seite</p>
          <ul>
            {seite.abschnitte.map((a) => (
              <li key={a.id}>
                <a href={`#${a.id}`}>{a.titel}</a>
              </li>
            ))}
          </ul>
        </nav>
      )}
      <article className="handbuch-text" dangerouslySetInnerHTML={{ __html: seite.html }} />
      <nav className="blaettern" aria-label="Blättern">
        {vorher ? (
          <Link href={`/handbuch/${vorher.slug}`}>← {vorher.nummer} {vorher.titel}</Link>
        ) : (
          <Link href="/handbuch">← Übersicht</Link>
        )}
        {nachher && <Link href={`/handbuch/${nachher.slug}`}>{nachher.nummer} {nachher.titel} →</Link>}
      </nav>
    </>
  );
}
