import Link from "next/link";

// Inhalte nach dem Benutzerhandbuch (Kapitel 1 „Was BauPlanIO leistet“);
// Bilder kommen aus docs/handbuch/benutzerhandbuch/bilder.

const FUNKTIONEN = [
  {
    titel: "Grundlagen erfassen",
    text: "Bauherrschaft, Rechtsgrundlagen nach §§ 30, 34, 35 BauGB, BauNVO-Fassung, Landesbauordnung, Bebauungsplan-Festsetzungen, Flurstücke und Grundbuch in einer Projektakte.",
    kapitel: "04-projekt-grundlagen",
  },
  {
    titel: "Aufmaß und Zeicheneditor",
    text: "Flächen, Längen und Höhen auf PDF-Plänen, DXF-Zeichnungen oder eigenen Zeichnungen aufmessen und direkt mit Gebäudemodell und Raumbuch verknüpfen.",
    kapitel: "05-ermittlung",
  },
  {
    titel: "IFC-Modelle (BIM)",
    text: "IFC-Modelle des Planers prüfen, für den Bauantrag nacharbeiten, auswerten und wieder als IFC-Datei weitergeben.",
    kapitel: "05-ermittlung",
  },
  {
    titel: "Berechnungen und Nachweise",
    text: "Baugrundstück, GRZ I/II, GFZ, BMZ, Abstandsflächen, Stellplätze, Wohnflächen, DIN 277, Vollgeschosse und mehr – archiviert als unveränderliche Nachweisstände.",
    kapitel: "06-nachweise",
  },
  {
    titel: "Formulare und Druckausgabe",
    text: "Amtliche PDF-Formulare mit Projektdaten ausfüllen, Berechnungsheft und Behördenausgabe im eigenen Briefbogen drucken.",
    kapitel: "07-formulare-druck",
  },
  {
    titel: "Verfahrensakte und XBau",
    text: "Behörde, Aktenzeichen, Abgabestände, Portalübergabe, XBau-Nachrichten, Nachforderungen, Bescheide und Bauausführung lückenlos dokumentiert.",
    kapitel: "08-verfahren",
  },
];

const ABLAUF = [
  { titel: "Büro einrichten", text: "Büroangaben, Briefbogen, Benutzer und Formularbibliothek – einmalig." },
  { titel: "Projekt anlegen", text: "Stammdaten, Rechtsgrundlagen, B-Plan und Flurstücke erfassen." },
  { titel: "Ermitteln", text: "Pläne ablegen, aufmessen, Gebäude und Räume aufbauen." },
  { titel: "Nachweisen", text: "Baugrundstück und Kennwerte berechnen, archivieren, freigeben." },
  { titel: "Einreichen", text: "Formulare ausfüllen, Abgabestand bilden, Verfahren führen." },
];

const ANSICHTEN = [
  { bild: "ermittlung-aufmass", titel: "Aufmaß auf dem Lageplan" },
  { bild: "nachweis-grz-ergebnis", titel: "GRZ/GFZ-Nachweis" },
  { bild: "verfahren-akte", titel: "Verfahrensakte" },
];

export default function Startseite() {
  return (
    <main>
      <section className="held">
        <div className="breite held-inhalt">
          <div className="held-text">
            <p className="oberzeile">Software für Architektur- und Planungsbüros</p>
            <h1>Der Bauantrag vom Flurstück bis zur Baugenehmigung – in einer Projektakte.</h1>
            <p className="einleitung">
              BauPlanIO führt Grundlagen, Aufmaß, Nachweise, Formulare und das Genehmigungsverfahren zusammen. Jeder
              Wert bleibt mit seiner Herkunft verknüpft, jeder Abgabestand nachvollziehbar.
            </p>
            <div className="knopfreihe">
              <Link href="/handbuch" className="knopf primaer">
                Handbuch öffnen
              </Link>
              <Link href="/#funktionen" className="knopf">
                Funktionen ansehen
              </Link>
            </div>
          </div>
          <figure className="held-bild">
            <img
              src="/handbuch/bilder/akte-projektuebersicht.png"
              alt="Projektakte mit Aktenkopf, Registern und Gliederung"
              width={1440}
              height={900}
            />
          </figure>
        </div>
      </section>

      <section id="funktionen" className="abschnitt">
        <div className="breite">
          <h2>Funktionen</h2>
          <p className="abschnitt-einleitung">Alles, was ein Bauantragsverfahren braucht – ohne Medienbrüche.</p>
          <div className="karten">
            {FUNKTIONEN.map((f) => (
              <article key={f.titel} className="karte">
                <h3>{f.titel}</h3>
                <p>{f.text}</p>
                <Link href={`/handbuch/${f.kapitel}`} className="mehr">
                  Im Handbuch lesen →
                </Link>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section id="ablauf" className="abschnitt abschnitt-grau">
        <div className="breite">
          <h2>So läuft ein Projekt</h2>
          <ol className="ablauf">
            {ABLAUF.map((s, i) => (
              <li key={s.titel}>
                <span className="schritt-nr">{i + 1}</span>
                <h3>{s.titel}</h3>
                <p>{s.text}</p>
              </li>
            ))}
          </ol>
          <p className="abschnitt-hinweis">
            Schritt für Schritt erklärt in den{" "}
            <Link href="/handbuch/09-workflows">Standard-Workflows des Handbuchs</Link>.
          </p>
        </div>
      </section>

      <section className="abschnitt">
        <div className="breite">
          <h2>Einblicke</h2>
          <div className="ansichten">
            {ANSICHTEN.map((a) => (
              <figure key={a.bild}>
                <img src={`/handbuch/bilder/${a.bild}.png`} alt={a.titel} width={1440} height={900} loading="lazy" />
                <figcaption>{a.titel}</figcaption>
              </figure>
            ))}
          </div>
        </div>
      </section>

      <section className="abschnitt abschnitt-band">
        <div className="breite band-inhalt">
          <div>
            <h2>Ausprobieren mit dem Beispielprojekt</h2>
            <p>
              Ein vollständig bearbeitetes Verfahren – Einfamilienhaus mit Garage in Nordrhein-Westfalen – liegt der
              Anwendung bei: <strong>Hilfe → Beispielprojekt öffnen</strong>.
            </p>
          </div>
          <Link href="/handbuch/01-einstieg#beispielprojekt-öffnen" className="knopf hell">
            Zur Anleitung
          </Link>
        </div>
      </section>
    </main>
  );
}
