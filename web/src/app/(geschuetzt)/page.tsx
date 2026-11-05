import Link from "next/link";
import { FUNKTIONEN as EDITOR } from "@/lib/zeicheneditor";

// Inhalte nach dem Benutzerhandbuch (Kapitel 1 „Was BauPlanIO leistet“);
// Bilder kommen aus docs/handbuch/benutzerhandbuch/bilder.

const FUNKTIONEN = [
  {
    titel: "Grundlagen erfassen",
    text: "Bauherrschaft, Rechtsgrundlagen nach §§ 30, 34, 35 BauGB, BauNVO-Fassung, Landesbauordnung, Bebauungsplan-Festsetzungen, Flurstücke und Grundbuch in einer Projektakte.",
    link: "/handbuch/04-projekt-grundlagen",
  },
  {
    titel: "Zeicheneditor und Aufmaß",
    text: "Auf PDF-Plänen, DXF-Zeichnungen oder eigenen Zeichnungen in Metern messen und konstruieren – mit Fang, Befehlszeile, Wänden, Öffnungen, Planzeichen und Planlayout. Jeder Messwert bleibt mit seinem Zielfeld verknüpft.",
    link: "/zeicheneditor",
  },
  {
    titel: "IFC-Modelle (BIM)",
    text: "IFC-Modelle des Planers prüfen, für den Bauantrag nacharbeiten, auswerten, in Gebäude, Zeichnungen und Raumbücher übernehmen und wieder als IFC-Datei weitergeben.",
    link: "/handbuch/05-ermittlung#ifc-modell-bim",
  },
  {
    titel: "Berechnungen und Nachweise",
    text: "Baugrundstück, GRZ I/II, GFZ, BMZ, Abstandsflächen, Stellplätze, Wohnflächen, DIN 277, Vollgeschosse und mehr – archiviert als unveränderliche Nachweisstände.",
    link: "/handbuch/06-nachweise",
  },
  {
    titel: "Formulare, Planliste und Druck",
    text: "Amtliche PDF-Formulare mit Projektdaten ausfüllen, Planliste mit Plannummern und Index führen, Berechnungsheft und Behördenausgabe im eigenen Briefbogen drucken.",
    link: "/handbuch/07-formulare-druck",
  },
  {
    titel: "Verfahrensakte und XBau",
    text: "Behörde, Aktenzeichen, Abgabestände, Portalübergabe, XBau-Nachrichten, Nachforderungen, Bescheide und Bauausführung lückenlos dokumentiert – mit Einreichungsprüfung vor dem Absenden.",
    link: "/handbuch/08-verfahren",
  },
  {
    titel: "Fristen und Projektbetreuung",
    text: "Büro-Cockpit über alle Projekte, Kalender-Abo und tägliche E-Mail im Netzwerkbetrieb, Projekttagebuch, Lieferlisten der Fachplaner und Sachstandsbericht für den Bauherrn.",
    link: "/handbuch/08-verfahren#büro-cockpit-kalender-abo-und-tägliche-e-mail",
  },
  {
    titel: "Abgleich statt Doppelerfassung",
    text: "Ein Feld hat genau eine Quelle. Handwerte und gezeichnete Flächen werden abgeglichen, veraltete Verknüpfungen nach Planänderungen gemeldet, jede Übernahme in der Chronik vermerkt.",
    link: "/ablaeufe#handwert",
  },
];

const ABLAUF = [
  { titel: "Büro einrichten", text: "Büroangaben, Briefbogen, Benutzer und Formularbibliothek – einmalig." },
  { titel: "Projekt anlegen", text: "Stammdaten, Rechtsgrundlagen, B-Plan und Flurstücke erfassen." },
  { titel: "Ermitteln", text: "Pläne ablegen, aufmessen, Gebäude und Räume aufbauen." },
  { titel: "Nachweisen", text: "Baugrundstück und Kennwerte berechnen, archivieren, freigeben." },
  { titel: "Einreichen", text: "Formulare ausfüllen, Abgabestand bilden, Verfahren führen." },
];

// Vorschau auf den Funktionsrundgang /zeicheneditor.
const EDITOR_VORSCHAU = ["zeichnung", "aufgaben", "konstruieren"].map((id) => EDITOR.find((f) => f.id === id)!);

const ANSICHTEN = [
  { bild: "arbeitsplatz-projektverzeichnis", titel: "Büro-Cockpit mit Fristen aller Projekte", link: "/ablaeufe#betreuung" },
  { bild: "nachweis-grz-ergebnis", titel: "GRZ/GFZ-Nachweis", link: "/handbuch/06-nachweise" },
  { bild: "verfahren-akte", titel: "Verfahrensakte", link: "/handbuch/08-verfahren" },
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
                <Link href={f.link} className="mehr">
                  {f.link.startsWith("/handbuch") ? "Im Handbuch lesen →" : "Mehr erfahren →"}
                </Link>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section id="zeicheneditor" className="abschnitt abschnitt-grau">
        <div className="breite">
          <h2>Zeicheneditor</h2>
          <p className="abschnitt-einleitung">
            Auf PDF- und DXF-Plänen messen, eigene Grundrisse in Metern zeichnen und jeden Wert verknüpft übernehmen.
          </p>
          <div className="ansichten">
            {EDITOR_VORSCHAU.map((f) => (
              <figure key={f.id}>
                <Link href={`/zeicheneditor#${f.id}`}>
                  <img src={`/handbuch/bilder/${f.bild}.png`} alt={f.alt} width={1440} height={900} loading="lazy" />
                </Link>
                <figcaption>{f.titel}</figcaption>
              </figure>
            ))}
          </div>
          <p className="abschnitt-hinweis">
            <Link href="/zeicheneditor" className="mehr">
              Alle Funktionen des Zeicheneditors →
            </Link>
          </p>
        </div>
      </section>

      <section id="ablauf" className="abschnitt">
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
            Wie die Schritte ineinandergreifen, zeigen die <Link href="/ablaeufe">Ablaufdiagramme</Link>; Schritt für
            Schritt erklärt in den <Link href="/handbuch/09-workflows">Standard-Workflows des Handbuchs</Link>.
          </p>
        </div>
      </section>

      <section className="abschnitt abschnitt-grau">
        <div className="breite">
          <h2>Einblicke</h2>
          <div className="ansichten">
            {ANSICHTEN.map((a) => (
              <figure key={a.bild}>
                <Link href={a.link}>
                  <img src={`/handbuch/bilder/${a.bild}.png`} alt={a.titel} width={1440} height={900} loading="lazy" />
                </Link>
                <figcaption>{a.titel}</figcaption>
              </figure>
            ))}
          </div>
        </div>
      </section>

      <section className="abschnitt abschnitt-band">
        <div className="breite band-inhalt">
          <div>
            <h2>Ein Verfahren von Anfang bis Ende</h2>
            <p>
              Das Handbuch zeigt jeden Schritt an einem vollständig bearbeiteten Verfahren: Einfamilienhaus mit
              Garage in Nordrhein-Westfalen, vom Flurstück bis zur Nachforderung der Bauaufsicht.
            </p>
          </div>
          <Link href="/handbuch/01-einstieg#das-beispielprojekt-in-diesem-handbuch" className="knopf hell">
            Zum Handbuch
          </Link>
        </div>
      </section>
    </main>
  );
}
