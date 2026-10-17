import type { ModulId } from "./module";
export interface NavZweig {
  id: string;
  titel: string;
  module: ModulId[];
  kinder?: NavZweig[];
}
export const NAVIGATION: NavZweig[] = [
  {
    id: "grundlagen",
    titel: "Projektgrundlagen",
    module: [
      "uebersicht",
      "rechtsgrundlagen",
      "festsetzungen",
      "flurstuecke",
      "gebaeude",
    ],
  },
  {
    id: "berechnungen",
    titel: "Berechnungen",
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
        module: [
          "raumbuch",
          "wohnflaechen",
          "din277",
          "vollgeschosse",
          "hoehen",
        ],
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
    ],
  },
  {
    id: "unterlagen",
    titel: "Unterlagen",
    module: ["dokumente", "formulare", "plaene", "druck", "unterlagen"],
  },
  {
    id: "verfahren",
    titel: "Verfahren",
    module: ["verfahren", "einreichen", "aufgaben", "aktensuche", "protokoll"],
    kinder: [
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
  const bekannt = new Set<string>(["projektuebersicht"]);
  for (const z of zweige) {
    for (const id of z.module) bekannt.add(id);
    for (const k of z.kinder ?? []) for (const id of k.module) bekannt.add(id);
  }
  for (const m of module) {
    if (bekannt.has(m.id)) continue;
    const gruppe =
      m.gruppe === "Berechnungen"
        ? "berechnungen"
        : m.gruppe === "Unterlagen"
          ? "unterlagen"
          : m.gruppe === "Organisation"
            ? "verfahren"
            : "grundlagen";
    const z = zweige.find((z) => z.id === gruppe)!;
    if (gruppe === "berechnungen")
      z.kinder!.find((k) => k.id === "weitere")!.module.push(m.id);
    else z.module.push(m.id);
    bekannt.add(m.id);
  }
  return zweige;
}
