import type { Metadata } from "next";
import Link from "next/link";
import { Ablaufdiagramm } from "@/lib/diagramm";
import { AUFMASS, BETREUUNG, HANDWERT, IFC, PLANAENDERUNG, PROJEKT, VERFAHREN } from "@/lib/ablaeufe";

export const metadata: Metadata = { title: "Abläufe" };

// Inhalte nach dem Benutzerhandbuch: Kapitel 5 (Aufmaß, IFC), 8 (Verfahren,
// Projektbetreuung) und 9 (Standard-Workflows). Die Toleranzen stehen in
// regelwerke/aufmass-toleranzen.json.

const KAPITEL = [
  { id: "ueberblick", titel: "Überblick" },
  { id: "aufmass", titel: "Aufmaß und Übernahme" },
  { id: "handwert", titel: "Handwert und Zeichnung" },
  { id: "planaenderung", titel: "Planänderung" },
  { id: "verfahren", titel: "Verfahren" },
  { id: "ifc", titel: "IFC-Modell" },
  { id: "betreuung", titel: "Fristen und Bauherr" },
];

/** Abweichung der gezeichneten von der amtlichen Flurstücksfläche. */
function Toleranzskala() {
  // Zonen nicht maßstäblich, damit die Beschriftungen Platz haben.
  const x = (p: number) => ({ 0: 40, 1: 200, 5: 480, 8: 680 })[p as 0 | 1 | 5 | 8];
  return (
    <figure className="toleranz">
      <svg
        viewBox="0 0 720 110"
        role="img"
        aria-label="Bis 1 Prozent oder 1 Quadratmeter Abweichung unauffällig, bis 5 Prozent Warnung, darüber Fehler"
      >
        <rect x={x(0)} y={20} width={x(1) - x(0)} height={40} className="tol-ok" />
        <rect x={x(1)} y={20} width={x(5) - x(1)} height={40} className="tol-warn" />
        <rect x={x(5)} y={20} width={x(8) - x(5)} height={40} className="tol-fehler" />
        <text x={(x(0) + x(1)) / 2} y={45} textAnchor="middle" className="tol-text">
          ruhig
        </text>
        <text x={(x(1) + x(5)) / 2} y={45} textAnchor="middle" className="tol-text">
          Warnung
        </text>
        <text x={(x(5) + x(8)) / 2} y={45} textAnchor="middle" className="tol-text">
          Fehler
        </text>
        {[0, 1, 5].map((p) => (
          <g key={p}>
            <line x1={x(p)} y1={60} x2={x(p)} y2={70} className="tol-strich" />
            <text x={x(p)} y={86} textAnchor="middle" className="tol-skala">
              {p === 1 ? "1 % bzw. 1 m²" : `${p} %`}
            </text>
          </g>
        ))}
        <text x={x(8)} y={86} textAnchor="end" className="tol-skala">
          Abweichung von der Katasterfläche
        </text>
      </svg>
      <figcaption>
        Gezeichnete Flurstücksgrenzen ersetzen nie die amtliche Fläche; sie dienen als Kontrolle. Bis 1 % (mindestens
        1 m²) bleibt die Anzeige ruhig, bis 5 % erscheint eine Warnung, darüber ein Fehler.
      </figcaption>
    </figure>
  );
}

function Legende() {
  return (
    <ul className="legende" aria-label="Legende">
      <li>
        <span className="legende-muster akzent" /> Kernschritt
      </li>
      <li>
        <span className="legende-muster daten" /> gespeicherter, unveränderlicher Stand
      </li>
      <li>
        <span className="legende-muster hinweis" /> Ereignis von außen
      </li>
      <li>
        <span className="legende-muster linie" /> Rückweg bei Änderungen
      </li>
    </ul>
  );
}

export default function Ablaeufe() {
  return (
    <main>
      <section className="seitenkopf">
        <div className="breite">
          <p className="oberzeile">Abläufe</p>
          <h1>Wie die Funktionen in einem Projekt ineinandergreifen</h1>
          <p className="einleitung">
            In BauPlanIO hat jeder Wert genau eine Quelle und bleibt mit ihr verknüpft. Die Diagramme zeigen, wohin ein
            Messwert wandert, was bei einer Planänderung passiert und wie das Verfahren bis zum Bescheid geführt wird.
          </p>
          <ul className="sprungmarken">
            {KAPITEL.map((k) => (
              <li key={k.id}>
                <a href={`#${k.id}`}>{k.titel}</a>
              </li>
            ))}
          </ul>
          <Legende />
        </div>
      </section>

      <div className="breite">
        <section id="ueberblick" className="ablaufkapitel">
          <h2>Überblick: vom Plan zur Behörde</h2>
          <p>
            Pläne und Modelle liegen versioniert in den Projektdateien. Was im Aufmaß gemessen wird, landet in Gebäude
            und Raumbuch, von dort in den Nachweisen. Archivierte Nachweisstände füllen Formulare und Berechnungsheft,
            und der Abgabestand hält genau die Versionen fest, die eingereicht werden.
          </p>
          <Ablaufdiagramm
            id="projekt"
            diagramm={PROJEKT}
            titel="Projektdateien, Aufmaß, Gebäude und Raumbuch, Nachweise, Nachweisstand, Formulare, Abgabestand und Verfahrensakte in einer Kette"
            beschreibung={
              <>
                Eine Nachforderung der Behörde führt über eine neue Dokumentversion wieder an den Anfang der Kette.
                Jede Übernahme und jeder Stand steht in der Chronik. Schritt für Schritt:{" "}
                <Link href="/handbuch/09-workflows#workflow-1-vom-grundstück-zur-erstabgabe">Workflow 1</Link>.
              </>
            }
          />
        </section>

        <section id="aufmass" className="ablaufkapitel">
          <h2>Aufmaß und Übernahme</h2>
          <p>
            Eine Markierung im Zeicheneditor wird erst durch Bedeutung und Zuordnung zum Messwert für ein bestimmtes
            Zielfeld. Die Übernahme zeigt alten und neuen Wert, setzt den Wert ein und sperrt das Feld, solange die
            Verknüpfung besteht.
          </p>
          <Ablaufdiagramm
            id="aufmass"
            diagramm={AUFMASS}
            titel="Unterlage, Maßstab, Markierung, Zuordnung, Übernahme in ein gesperrtes Zielfeld; Planänderungen melden das Feld als veraltet"
            beschreibung={
              <>
                Ändert sich eine Kontur, der Maßstab oder die Dokumentversion, meldet das Zielfeld beim Öffnen
                „veraltet“. Mit „Aktuellen Aufmaßstand übernehmen“ wird der neue Wert eingesetzt.{" "}
                <Link href="/zeicheneditor">Zum Zeicheneditor</Link>
              </>
            }
          />
        </section>

        <section id="handwert" className="ablaufkapitel">
          <h2>Handwert und Zeichnung</h2>
          <p>
            Oft steht ein Wert schon von Hand in der Akte, bevor er gezeichnet wird. Die Übernahme erkennt passende
            Handzeilen und fragt nach, statt still eine zweite Zeile anzulegen. So zählt keine Fläche doppelt im
            Baugrundstück oder in der GRZ.
          </p>
          <Ablaufdiagramm
            id="handwert"
            diagramm={HANDWERT}
            titel="Bei passender Handzeile fragt die Übernahme, ob verknüpft oder eine neue Zeile angelegt wird; danach hat das Feld eine Quelle"
            beschreibung={
              <>
                „Verknüpfung lösen“ macht den Wert wieder zur Handangabe und behält ihn. Stehen eine gezeichnete und
                eine fast gleich große Handzeile (höchstens 1 m² oder 2 % Unterschied) nebeneinander, meldet die
                Baugrundstücksprüfung „Handzeile entfernen“.
              </>
            }
          />
          <Toleranzskala />
        </section>

        <section id="planaenderung" className="ablaufkapitel">
          <h2>Planänderung einarbeiten</h2>
          <p>
            Ändert sich der Entwurf, nachdem schon gemessen und gerechnet wurde, zeigt die Akte jeden betroffenen Wert
            an. Nichts wird stillschweigend überschrieben.
          </p>
          <Ablaufdiagramm
            id="planaenderung"
            diagramm={PLANAENDERUNG}
            titel="Sieben Schritte von der neuen Planversion bis zur Nachreichung"
            beschreibung={
              <>
                Das alte Aufmaß bleibt bei der alten Version und nimmt keine Übernahmen mehr an. Nachweisstände mit
                „Grundlagen geändert“ werden neu berechnet und freigegeben. Einzelheiten:{" "}
                <Link href="/handbuch/09-workflows#workflow-4-planänderung-einarbeiten">Workflow 4</Link>.
              </>
            }
          />
        </section>

        <section id="verfahren" className="ablaufkapitel">
          <h2>Verfahren bis zum Bescheid</h2>
          <p>
            Die Verfahrensakte führt Behörde, Aktenzeichen, Abgabestände, Kommunikation und Bescheide. Export, Versand,
            bestätigter Eingang und Genehmigung sind getrennte Zustände mit jeweils eigenem Beleg.
          </p>
          <Ablaufdiagramm
            id="verfahren"
            diagramm={VERFAHREN}
            titel="Vorbereitung, Abgabestand, Einreichen, Eingang; bei Nachforderung zurück zum Abgabestand, sonst Bescheid und Ausführung"
            beschreibung={
              <>
                Jede Nachforderung wird Punkt für Punkt mit Frist geführt und über einen neuen Abgabestand
                nachgereicht. Im Bescheid werden die ausdrücklich genehmigten Versionen zugeordnet, nicht pauschal die
                neuesten. Mehr in{" "}
                <Link href="/handbuch/08-verfahren">Kapitel 8</Link> und{" "}
                <Link href="/handbuch/09-workflows#workflow-2-nachforderung-beantworten-und-nachreichen">Workflow 2</Link>.
              </>
            }
          />
        </section>

        <section id="ifc" className="ablaufkapitel">
          <h2>IFC-Modell des Planers</h2>
          <p>
            Das Modell aus der Planungssoftware wird geprüft und für den Bauantrag nachgearbeitet, ohne die Datei des
            Planers zu verändern. Die Übernahme legt Gebäude, Zeichnungen und Raumbücher an, die dann wie jedes Aufmaß
            weiterlaufen.
          </p>
          <Ablaufdiagramm
            id="ifc"
            diagramm={IFC}
            titel="IFC-Datei, Prüfung, Nacharbeit, Auswertung, Übernahme in Zeicheneditor und Nachweise, Export als IFC4"
            beschreibung={
              <>
                Liefert der Planer eine neue Fassung, zeigt der Abgleich geänderte, neue und entfallene Bauteile.
                Einzelheiten:{" "}
                <Link href="/handbuch/09-workflows#workflow-6-ifc-modell-des-planers-für-den-bauantrag-nutzen">
                  Workflow 6
                </Link>
                .
              </>
            }
          />
        </section>

        <section id="betreuung" className="ablaufkapitel">
          <h2>Fristen im Büro, Bericht für den Bauherrn</h2>
          <p>
            Termine werden an einer Stelle berechnet und an mehreren gezeigt. Der Sachstandsbericht für den Bauherrn
            enthält nur, was die Behörde betrifft.
          </p>
          <Ablaufdiagramm
            id="betreuung"
            diagramm={BETREUUNG}
            titel="Fristen aus Verfahrensakten, Aufgaben, Fachplanern und Tagebuch laufen in Büro-Cockpit, Kalender-Abo und E-Mail; der Sachstandsbericht geht an den Bauherrn"
            beschreibung={
              <>
                Kalender-Abo und tägliche E-Mail gibt es im Netzwerkbetrieb. Aufgaben, Tagebuch und Fachplaner bleiben
                bürointern und erscheinen nicht im Sachstandsbericht. Mehr in{" "}
                <Link href="/handbuch/08-verfahren#büro-cockpit-kalender-abo-und-tägliche-e-mail">Kapitel 8</Link>.
              </>
            }
          />
        </section>
      </div>
    </main>
  );
}
