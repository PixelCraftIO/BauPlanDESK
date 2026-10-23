import inhalt from "./handbuch.json" with { type: "json" };

export interface Abschnitt {
  titel?: string;
  text?: string[];
  schritte?: string[];
  liste?: string[];
  tabelle?: { spalten: string[]; zeilen: string[][] };
  hinweis?: string;
  abbildung?: { datei: string; alt: string; beschreibung: string };
}
export interface Kapitel {
  id: string;
  titel: string;
  gruppe: string;
  kurztext: string;
  bereiche: string[];
  abschnitte: Abschnitt[];
  siehe: string[];
  stichworte: string[];
}
export interface Handbuch {
  schemaVersion: number;
  titel: string;
  untertitel: string;
  stand: string;
  basis: string;
  kapitel: Kapitel[];
}
/** Eine Inhaltsquelle für Offline-Hilfe, Druck und exportierte Dokumentation. */
export const handbuch: Handbuch = inhalt;
export function kapitelFuerBereich(bereich: string): string {
  return handbuch.kapitel.find((k) => k.bereiche.includes(bereich))?.id ?? "start";
}
function normalisieren(s: string) {
  return s.toLocaleLowerCase("de-DE").normalize("NFD").replace(/\p{M}/gu, "").replaceAll("ß", "ss");
}
/** Volltextsuche einschließlich Schritten, Tabellen und deutschen Umlauten. */
export function sucheKapitel(suche: string): Kapitel[] {
  const woerter = normalisieren(suche.trim()).split(/\s+/).filter(Boolean);
  return handbuch.kapitel.filter((k) => {
    const text = normalisieren([k.titel, k.kurztext, ...k.stichworte,
      ...k.abschnitte.flatMap((a) => [a.titel ?? "", ...(a.text ?? []), ...(a.schritte ?? []),
        ...(a.liste ?? []), a.hinweis ?? "", a.abbildung?.alt ?? "", a.abbildung?.beschreibung ?? "", ...(a.tabelle?.spalten ?? []),
        ...(a.tabelle?.zeilen.flat() ?? [])])].join(" "));
    return woerter.every((w) => text.includes(w));
  });
}
