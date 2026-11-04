import type { Metadata } from "next";
import Link from "next/link";
import { FUNKTIONEN } from "@/lib/zeicheneditor";

export const metadata: Metadata = { title: "Zeicheneditor" };

export default function Zeicheneditor() {
  return (
    <main>
      <section className="seitenkopf">
        <div className="breite">
          <p className="oberzeile">Zeicheneditor</p>
          <h1>Messen, zeichnen und übernehmen – auf jedem Plan</h1>
          <p className="einleitung">
            Der Zeicheneditor ist das Werkzeug für Genehmigungsplanung und Nachweise neben dem Büro-CAD. Er misst auf
            PDF- und DXF-Plänen, zeichnet eigene Grundrisse in Metern und schreibt jeden Messwert verknüpft in die
            Projektakte. Alle Bilder stammen aus dem Beispielprojekt „Neubau Einfamilienhaus Mustermann“.
          </p>
          <ul className="sprungmarken">
            {FUNKTIONEN.map((f) => (
              <li key={f.id}>
                <a href={`#${f.id}`}>{f.titel}</a>
              </li>
            ))}
          </ul>
        </div>
      </section>

      <div className="breite">
        {FUNKTIONEN.map((f, i) => (
          <section key={f.id} id={f.id} className="funktion">
            <figure className="funktion-bild">
              <a href={`/handbuch/bilder/${f.bild}.png`} title="Bild in voller Größe öffnen">
                <img
                  src={`/handbuch/bilder/${f.bild}.png`}
                  alt={f.alt}
                  width={1440}
                  height={900}
                  loading={i < 2 ? "eager" : "lazy"}
                />
              </a>
            </figure>
            <div className="funktion-text">
              <h2>{f.titel}</h2>
              <p>{f.text}</p>
              <ul>
                {f.punkte.map((p) => (
                  <li key={p}>{p}</li>
                ))}
              </ul>
              <Link href={f.link} className="mehr">
                {f.linkText} →
              </Link>
            </div>
          </section>
        ))}
      </div>

      <section className="abschnitt abschnitt-band">
        <div className="breite band-inhalt">
          <div>
            <h2>Wie die Werte weiterlaufen</h2>
            <p>Vom Messwert über Nachweis und Abgabestand bis zur Behörde – als Ablaufdiagramme.</p>
          </div>
          <Link href="/ablaeufe" className="knopf hell">
            Zu den Abläufen
          </Link>
        </div>
      </section>
    </main>
  );
}
