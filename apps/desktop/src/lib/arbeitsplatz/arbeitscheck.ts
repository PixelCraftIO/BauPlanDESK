import type {
  Arbeitspunkt,
  NachweisEingabe,
  UebersichtsZiel,
} from "../generated/plattform";
export const ziel = (
  bereich: string,
  objekt: string | null = null,
  register: string | null = null,
  eintrag: string | null = null,
): UebersichtsZiel => ({ bereich, objekt, register, eintrag });
/** Nur Gruppierung des Core-Lesemodells; keine Vollständigkeits- oder Rechtsentscheidung. */
export const pruefgruppen = [
  {
    id: "grundlagen",
    titel: "Grundlagen",
    bereich: "rechtsgrundlagen",
    register: null,
  },
  {
    id: "nachweise",
    titel: "Berechnungen und Freigaben",
    bereich: "nachweise",
    register: null,
  },
  {
    id: "unterlagen",
    titel: "Unterlagen und Versionen",
    bereich: "unterlagen",
    register: null,
  },
  {
    id: "verfahren",
    titel: "Anforderungen und Verfahren",
    bereich: "verfahren",
    register: "anforderungen",
  },
  {
    id: "einreichungen",
    titel: "Abgabe und Eingangsbelege",
    bereich: "portale",
    register: null,
  },
] as const;
export function punkteFuer(
  punkte: Arbeitspunkt[],
  thema: string,
  verfahren = "",
) {
  return punkte.filter(
    (p) =>
      p.thema === thema &&
      (!verfahren ||
        !["verfahren", "portale"].includes(p.ziel.bereich) ||
        !p.ziel.objekt ||
        p.ziel.objekt === verfahren),
  );
}
export function nachweisZiel(e: NachweisEingabe): UebersichtsZiel {
  switch (e.art) {
    case "fach":
      return ziel(e.plan.modul, e.objekt, "daten");
    case "abstand_projekt":
      return ziel("abstand", e.objekt);
    case "abstand":
      return ziel("abstand");
    case "nutzung":
      return ziel(
        { grz: "grz", gfz: "gfz", geschossigkeit: "geschosse", bmz: "volumen" }[
          e.fach
        ],
      );
  }
}
