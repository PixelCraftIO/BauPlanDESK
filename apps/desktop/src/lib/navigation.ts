import type { ModulId } from "./module";
export interface NavZweig {
  id: string;
  titel: string;
  module: ModulId[];
  kinder?: NavZweig[];
  /** Nur Register: Nummer des Bereichs („Bereich 1“) und eine Zeile Erklärung. */
  ziffer?: string;
  untertitel?: string;
}
/**
 * Register der Projektakte: Übersicht sowie die beiden Bereiche Ermittlung
 * (alles, was Werte erzeugt) und Nachweis und Einreichung (alles, was Werte
 * verarbeitet und ausgibt). Jedes Register zeigt links nur seine eigene
 * Gliederung; Unterzweige erscheinen dort als Zwischenüberschriften.
 */
export const NAVIGATION: NavZweig[] = [
  {
    id: "akte",
    titel: "Übersicht",
    untertitel: "Stand, offene Punkte",
    module: [
      "projektuebersicht",
      "aufgaben",
      "projekttagebuch",
      "fachplaner",
      "protokoll",
    ],
  },
  {
    id: "ermittlung",
    titel: "Ermittlung",
    ziffer: "Bereich 1",
    untertitel: "Flächen, Höhen, Maße aus Unterlagen und Zeichnungen",
    module: [],
    kinder: [
      {
        id: "grundlagen",
        titel: "Grundlagen",
        module: [
          "uebersicht",
          "rechtsgrundlagen",
          "festsetzungen",
          "flurstuecke",
        ],
      },
      {
        id: "unterlagen",
        titel: "Unterlagen",
        module: [
          "dokumente",
          "planaufmass",
          "messwerte",
          "planliste",
          "unterlagen",
        ],
      },
      {
        id: "gebaeudemodell",
        titel: "Gebäudemodell",
        module: ["gebaeude", "raumbuch"],
      },
    ],
  },
  {
    id: "nachweis",
    titel: "Nachweis und Einreichung",
    ziffer: "Bereich 2",
    untertitel: "Berechnungen, Formulare, Ausgabe fürs Bauamt",
    module: [],
    kinder: [
      {
        id: "nutzung",
        titel: "Grundstück und Nutzung",
        module: [
          "grundstueck",
          "grz",
          "gfz",
          "geschosse",
          "volumen",
          "ueberbaubarkeit",
        ],
      },
      {
        id: "flaechen",
        titel: "Gebäude und Flächen",
        module: ["wohnflaechen", "din277", "vollgeschosse", "hoehen"],
      },
      {
        id: "weitere",
        titel: "Weitere Nachweise",
        module: [
          "stellplaetze",
          "abstand",
          "freiflaechen",
          "entwaesserung",
          "nachweise",
        ],
      },
      {
        id: "ausgabe",
        titel: "Formulare und Ausgabe",
        module: ["formulare", "baukennwerte", "druck"],
      },
      {
        id: "verfahren",
        titel: "Verfahren",
        module: [
          "verfahren",
          "einreichungspruefung",
          "einreichen",
          "aktensuche",
        ],
      },
      {
        id: "einreichungswerkzeuge",
        titel: "Einreichungswerkzeuge",
        module: ["bauantrag", "portale", "xbau", "einreichungen"],
      },
    ],
  },
];
// Sichtbare Kurztitel bleiben Teil des vollständigen zugänglichen Namens.
export const KURZTITEL: Partial<Record<ModulId, string>> = {
  festsetzungen: "Planfestsetzungen",
  flurstuecke: "Flurstücke",
  din277: "DIN-277-Flächen",
  hoehen: "Gebäudehöhen",
};
export function enthaelt(z: NavZweig, id: string): boolean {
  return (
    z.module.includes(id as ModulId) || !!z.kinder?.some((k) => enthaelt(k, id))
  );
}

/** Neue registrierte Module bleiben auch ohne eigene Navigationsanpassung erreichbar. */
export function navigationMitModulen(
  module: readonly { id: ModulId; gruppe: string }[],
) {
  const zweige = structuredClone(NAVIGATION);
  const bekannt = new Set<string>();
  for (const z of zweige) for (const id of moduleVon(z)) bekannt.add(id);
  const zweig = (register: string, kind: string) =>
    zweige.find((z) => z.id === register)!.kinder!.find((k) => k.id === kind)!;
  for (const m of module) {
    if (bekannt.has(m.id)) continue;
    const ziel =
      m.gruppe === "Berechnungen"
        ? zweig("nachweis", "weitere")
        : m.gruppe === "Unterlagen"
          ? zweig("ermittlung", "unterlagen")
          : m.gruppe === "Organisation"
            ? zweig("nachweis", "verfahren")
            : zweig("ermittlung", "grundlagen");
    ziel.module.push(m.id);
    bekannt.add(m.id);
  }
  return zweige;
}

/** Alle Module eines Registers in Anzeigereihenfolge. */
export function moduleVon(z: NavZweig): ModulId[] {
  return [...z.module, ...(z.kinder ?? []).flatMap(moduleVon)];
}

/** Register, zu dem eine Ansicht gehört; die Maßansicht zählt zum Baugrundstück. */
export function registerVon(
  register: NavZweig[],
  ansicht: string,
): NavZweig | undefined {
  const id = ansicht === "mass" ? "grundstueck" : ansicht;
  return register.find((z) => enthaelt(z, id));
}
