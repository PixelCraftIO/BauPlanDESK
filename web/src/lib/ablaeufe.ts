// Ablaufdiagramme der Webseite. Inhalte nach dem Benutzerhandbuch
// (Kapitel 5, 8 und 9) und dem Verhalten der Anwendung; Raster 300 px,
// Kästen 190 × 64 px, Platz für Pfeilbeschriftungen dazwischen.
import type { Diagramm } from "./diagramm";

const S = [20, 320, 620, 920];

/** Wie Werte durch eine Projektakte laufen, vom Plan bis zur Behörde. */
export const PROJEKT: Diagramm = {
  breite: 1130,
  hoehe: 400,
  knoten: [
    { id: "g", x: S[3], y: 16, titel: "Grundlagen", zeilen: ["Recht, B-Plan, Flurstücke"] },
    { id: "p", x: S[0], y: 150, titel: "Projektdateien", zeilen: ["PDF · DXF · IFC, versioniert"] },
    { id: "a", x: S[1], y: 150, titel: "Aufmaß & Zeicheneditor", zeilen: ["Markierungen mit Bedeutung"] },
    { id: "f", x: S[2], y: 150, titel: "Gebäude & Raumbuch", zeilen: ["Geschosse, Räume, Fachwerte"] },
    { id: "n", x: S[3], y: 150, titel: "Nachweise", zeilen: ["Baugrundstück, GRZ/GFZ …"], art: "akzent" },
    { id: "t", x: S[3], y: 316, titel: "Nachweisstand", zeilen: ["unveränderlich, freigegeben"], art: "daten" },
    { id: "d", x: S[2], y: 316, titel: "Formulare & Druck", zeilen: ["Formulare, Berechnungsheft"] },
    { id: "s", x: S[1], y: 316, titel: "Abgabestand", zeilen: ["eingefrorene Versionen"], art: "daten" },
    { id: "v", x: S[0], y: 316, titel: "Verfahrensakte", zeilen: ["Portal · XBau · Bescheid"] },
  ],
  kanten: [
    { von: "p", nach: "a", text: "misst auf" },
    { von: "a", nach: "f", text: "übernimmt", akzent: true },
    { von: "f", nach: "n", text: "liefert Flächen" },
    { von: "g", nach: "n", text: "Grenzwerte, Kataster" },
    { von: "n", nach: "t", text: "archiviert" },
    { von: "t", nach: "d", text: "füllt Kennwerte" },
    { von: "d", nach: "s", text: "neue Versionen" },
    { von: "s", nach: "v", text: "einreichen" },
    { von: "p", nach: "s", aus: "u", ausVersatz: 50, ein: "o", ueber: [[165, 262], [415, 262]], text: "Pläne", textAbschnitt: 1 },
    { von: "v", nach: "p", aus: "o", ausVersatz: -50, ein: "u", einVersatz: -50, gestrichelt: true, text: "Nachforderung" },
  ],
};

/** Aufmaß: vom Plan über die Markierung in ein Zielfeld, und zurück bei Änderungen. */
export const AUFMASS: Diagramm = {
  breite: 1130,
  hoehe: 420,
  knoten: [
    { id: "u", x: S[0], y: 16, titel: "Unterlage", zeilen: ["PDF, DXF oder Zeichnung (m)"] },
    { id: "m", x: S[1], y: 16, titel: "Maßstab", zeilen: ["1:n, zwei Punkte, Einheit"] },
    { id: "z", x: S[2], y: 16, titel: "Markierung zeichnen", zeilen: ["aus Aufgaben oder Vorlage"] },
    { id: "b", x: S[3], y: 16, titel: "Bedeutung & Zuordnung", zeilen: ["z. B. Raum → Raumbuch EG"] },
    { id: "e", x: S[3], y: 180, titel: "Übernehmen", zeilen: ["Vorschau: alt → neu"], art: "akzent" },
    { id: "f", x: S[2], y: 180, titel: "Zielfeld", zeilen: ["verknüpft, schreibgeschützt"] },
    { id: "r", x: S[1], y: 180, titel: "Berechnung", zeilen: ["Nachweis nutzt den Wert"] },
    { id: "c", x: S[3], y: 340, titel: "Chronik", zeilen: ["Zielfeld, alter & neuer Wert"], art: "daten" },
    { id: "q", x: S[2], y: 340, titel: "Planänderung", zeilen: ["Kontur, Maßstab, Version"], art: "hinweis" },
  ],
  kanten: [
    { von: "u", nach: "m", text: "kalibrieren" },
    { von: "m", nach: "z", text: "zeichnen" },
    { von: "z", nach: "b", text: "einordnen" },
    { von: "b", nach: "e", text: "speichern" },
    { von: "e", nach: "f", text: "setzt Wert", akzent: true },
    { von: "f", nach: "r", text: "rechnet" },
    { von: "e", nach: "c", aus: "u", ausVersatz: 40, ein: "o", einVersatz: 40, text: "vermerkt" },
    { von: "q", nach: "f", aus: "o", ausVersatz: -40, ein: "u", einVersatz: -40, gestrichelt: true, text: "meldet „veraltet“" },
    { von: "q", nach: "e", aus: "o", ausVersatz: 50, ein: "u", einVersatz: -50, ueber: [[765, 300], [965, 300]], gestrichelt: true, text: "neu übernehmen", textAbschnitt: 1 },
  ],
};

/** Handwert und Zeichnung: Übernahme mit vorhandener Handzeile. */
export const HANDWERT: Diagramm = {
  breite: 1130,
  hoehe: 330,
  knoten: [
    { id: "s", x: S[0], y: 40, titel: "Markierung übernehmen", zeilen: ["Abzug, Bauteil oder Raum"] },
    { id: "q", x: 315, y: 20, w: 200, h: 104, titel: "Handzeile da?", zeilen: ["gleiche Art, gleicher Ort"], art: "frage" },
    { id: "k", x: 620, y: 40, w: 420, titel: "Rückfrage", zeilen: ["„verknüpfen“: Handzeile erhält den Messwert", "oder bewusst eine neue Zeile anlegen"] },
    { id: "n", x: 320, y: 230, titel: "Neue Zeile", zeilen: ["mit der Markierung verknüpft"] },
    { id: "e", x: 735, y: 230, titel: "Eine Quelle je Feld", zeilen: ["Feld gesperrt, Chronik vermerkt"], art: "akzent" },
  ],
  kanten: [
    { von: "s", nach: "q" },
    { von: "q", nach: "k", text: "ja" },
    { von: "q", nach: "n", text: "nein" },
    { von: "k", nach: "e", aus: "u", ein: "o", einVersatz: 0, ausVersatz: 0, text: "Auswahl" },
    { von: "n", nach: "e", text: "speichern" },
  ],
};

/** Planänderung nach Messung und Berechnung (Handbuch, Workflow 4). */
export const PLANAENDERUNG: Diagramm = {
  breite: 1130,
  hoehe: 270,
  knoten: [
    { id: "1", x: S[0], y: 16, titel: "1 Neue Planversion", zeilen: ["als Version derselben Datei"], art: "hinweis" },
    { id: "2", x: S[1], y: 16, titel: "2 Planstand umstellen", zeilen: ["selbes Aufmaß, Passpunkte"] },
    { id: "3", x: S[2], y: 16, titel: "3 Verknüpfungen", zeilen: ["„aktuellen Stand übernehmen“"] },
    { id: "4", x: S[3], y: 16, titel: "4 Raumbuch", zeilen: ["„Räume aktualisieren“"] },
    { id: "5", x: S[3], y: 186, titel: "5 Nachweisstände", zeilen: ["Aktualität prüfen, freigeben"], art: "akzent" },
    { id: "6", x: S[2], y: 186, titel: "6 Druck & Formulare", zeilen: ["als neue Versionen ablegen"] },
    { id: "7", x: S[1], y: 186, titel: "7 Nachreichung", zeilen: ["neuer Abgabestand"], art: "daten" },
    { id: "0", x: S[0], y: 186, titel: "Bisheriger Plan", zeilen: ["liegt zum Vergleich darüber"] },
  ],
  kanten: [
    { von: "1", nach: "2" },
    { von: "2", nach: "3", text: "Konturen anpassen" },
    { von: "3", nach: "4" },
    { von: "4", nach: "5", text: "neu rechnen" },
    { von: "5", nach: "6" },
    { von: "6", nach: "7", text: "falls eingereicht" },
    { von: "1", nach: "0", gestrichelt: true, text: "alte Version" },
  ],
};

/** Verfahren von der Vorbereitung bis zur Bauausführung. */
export const VERFAHREN: Diagramm = {
  breite: 1130,
  hoehe: 430,
  knoten: [
    { id: "v", x: S[0], y: 16, titel: "Vorbereitung", zeilen: ["Anforderungen, Profil"] },
    { id: "a", x: S[1], y: 16, titel: "Abgabestand", zeilen: ["Versionen festhalten"], art: "daten" },
    { id: "e", x: S[2], y: 16, titel: "Einreichen", zeilen: ["Bauportal oder XBau"] },
    { id: "b", x: S[3], y: 16, titel: "Eingang belegt", zeilen: ["Portalbeleg, Kommunikation"] },
    { id: "q", x: S[3], y: 160, h: 104, titel: "Nachforderung?", zeilen: ["Behörde fordert nach"], art: "frage" },
    { id: "n", x: S[2], y: 180, titel: "Nachforderung", zeilen: ["Punkte, Fristen, Antworten"], art: "hinweis" },
    { id: "s", x: S[3], y: 350, titel: "Bescheid erfassen", zeilen: ["genehmigte Versionen"], art: "akzent" },
    { id: "u", x: S[2], y: 350, titel: "Bauausführung", zeilen: ["Baubeginn, Nutzungsaufnahme"] },
  ],
  kanten: [
    { von: "v", nach: "a", text: "Arbeitscheck" },
    { von: "a", nach: "e", text: "exportieren" },
    { von: "e", nach: "b", text: "Beleg" },
    { von: "b", nach: "q" },
    { von: "q", nach: "n", text: "ja" },
    { von: "n", nach: "a", aus: "l", ein: "u", ueber: [[415, 212]], text: "Nachreichung", textAbschnitt: 0 },
    { von: "q", nach: "s", text: "nein" },
    { von: "s", nach: "u", text: "Auflagen" },
  ],
};

/** IFC-Modell des Planers: vom Import bis zum Export für den Bauantrag. */
export const IFC: Diagramm = {
  breite: 1130,
  hoehe: 430,
  knoten: [
    { id: "i", x: S[0], y: 16, titel: "IFC-Datei", zeilen: ["vom Planer, bleibt unverändert"] },
    { id: "p", x: S[1], y: 16, titel: "Bericht & Prüfung", zeilen: ["Einheit, Georeferenz, Klassen"] },
    { id: "n", x: S[2], y: 16, titel: "Nacharbeit", zeilen: ["Befunde beheben"] },
    { id: "a", x: S[3], y: 16, titel: "Auswertung", zeilen: ["Grundfläche, BGF, Höhen"] },
    { id: "u", x: S[3], y: 186, titel: "Übernahme", zeilen: ["Gebäude, Pläne, Räume"], art: "akzent" },
    { id: "z", x: S[2], y: 186, titel: "Zeicheneditor", zeilen: ["Werte zuordnen, übernehmen"] },
    { id: "w", x: S[1], y: 186, titel: "Nachweise", zeilen: ["GRZ, GFZ, Wohnfläche …"] },
    { id: "x", x: S[3], y: 350, titel: "IFC4-Export", zeilen: ["dem Bauantrag beifügen"], art: "daten" },
    { id: "f", x: S[0], y: 350, titel: "Neue Fassung", zeilen: ["geändert, neu, entfallen"], art: "hinweis" },
  ],
  kanten: [
    { von: "i", nach: "p", text: "öffnen" },
    { von: "p", nach: "n", text: "Befunde" },
    { von: "n", nach: "a", text: "plausibel?" },
    { von: "a", nach: "u", text: "übernehmen" },
    { von: "u", nach: "z" },
    { von: "z", nach: "w" },
    { von: "u", nach: "x", aus: "u", ausVersatz: 40, ein: "o", einVersatz: 40, text: "schreibt" },
    { von: "f", nach: "i", gestrichelt: true, text: "neue Version" },
    { von: "f", nach: "u", aus: "r", ein: "u", einVersatz: -50, ueber: [[770, 382], [770, 300], [965, 300]], gestrichelt: true, text: "abgleichen", textAbschnitt: 0 },
  ],
};

/** Projektbetreuung: Termine aus allen Akten, Ausgaben für Büro und Bauherrn. */
export const BETREUUNG: Diagramm = {
  breite: 1130,
  hoehe: 440,
  knoten: [
    { id: "q1", x: S[0], y: 16, h: 56, titel: "Verfahrensakten", zeilen: ["Behördenfristen, Auflagen"] },
    { id: "q2", x: S[0], y: 92, h: 56, titel: "Aufgaben", zeilen: ["Zuständige, Fälligkeit"] },
    { id: "q3", x: S[0], y: 168, h: 56, titel: "Fachplaner", zeilen: ["zugesagte Lieferungen"] },
    { id: "q4", x: S[0], y: 244, h: 56, titel: "Projekttagebuch", zeilen: ["Wiedervorlagen"] },
    { id: "t", x: 455, y: 118, w: 220, h: 80, titel: "Termine aller Projekte", zeilen: ["eine Fristberechnung"], art: "akzent" },
    { id: "o1", x: S[3], y: 36, h: 56, titel: "Büro-Cockpit", zeilen: ["Startseite der App"] },
    { id: "o2", x: S[3], y: 130, h: 56, titel: "Kalender-Abo", zeilen: ["im Netzwerkbetrieb"] },
    { id: "o3", x: S[3], y: 224, h: 56, titel: "Tägliche E-Mail", zeilen: ["fällig, überfällig"] },
    { id: "v", x: S[0], y: 360, titel: "Verfahrensakte", zeilen: ["nur Behördenvorgänge"] },
    { id: "b", x: 470, y: 360, titel: "Sachstandsbericht", zeilen: ["Schreiben im Briefbogen"] },
    { id: "h", x: S[3], y: 360, titel: "Bauherr", zeilen: ["Stand und Fristen"], art: "daten" },
  ],
  kanten: [
    { von: "q1", nach: "t", aus: "r", ein: "l", ueber: [[310, 44], [310, 158]] },
    { von: "q2", nach: "t", aus: "r", ein: "l", ueber: [[310, 120], [310, 158]] },
    { von: "q3", nach: "t", aus: "r", ein: "l", ueber: [[310, 196], [310, 158]] },
    { von: "q4", nach: "t", aus: "r", ein: "l", ueber: [[310, 272], [310, 158]] },
    { von: "t", nach: "o1", aus: "r", ein: "l", ueber: [[800, 158], [800, 64]] },
    { von: "t", nach: "o2", aus: "r", ein: "l" },
    { von: "t", nach: "o3", aus: "r", ein: "l", ueber: [[800, 158], [800, 252]] },
    { von: "v", nach: "b", text: "ohne Büro-Interna" },
    { von: "b", nach: "h", text: "PDF" },
  ],
};
