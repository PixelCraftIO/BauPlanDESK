import type { Project } from "../api";
export interface FachProps {
  projekt: Project;
  benutzerId: string;
  darf: boolean;
  darfFreigeben?: boolean;
  /** Übergibt einen vom Modul gespeicherten Projektstand an die Akte (etwa Flurstücke aus dem Aufmaß). */
  onprojekt?: (p: Project) => void;
  /** Wunsch, eine Unterlage (Dokument-ID) zu öffnen; `nummer` unterscheidet wiederholte Wünsche. Nur das Aufmaß wertet ihn aus. */
  unterlage?: { dokumentId: string; nummer: number } | null;
}
export const FACHMODULE = [
  {
    id: "planaufmass",
    titel: "Aufmaß",
    gruppe: "Unterlagen",
    laden: () => import("../planaufmass/Planaufmass.svelte"),
  },
  {
    id: "ueberbaubarkeit",
    titel: "Überbaubare Grundstücksfläche",
    gruppe: "Berechnungen",
    laden: () => import("../berechnungen/Ueberbaubarkeit.svelte"),
  },
  {
    id: "stellplaetze",
    titel: "Stellplatznachweis",
    gruppe: "Berechnungen",
    laden: () => import("../berechnungen/Stellplaetze.svelte"),
  },
  {
    id: "wohnflaechen",
    titel: "Wohnflächenberechnung",
    gruppe: "Berechnungen",
    laden: () => import("../berechnungen/Wohnflaechen.svelte"),
  },
  {
    id: "din277",
    titel: "DIN-277-Flächen / Rauminhalte",
    gruppe: "Berechnungen",
    laden: () => import("../berechnungen/Din277.svelte"),
  },
  {
    id: "vollgeschosse",
    titel: "Vollgeschossprüfung",
    gruppe: "Berechnungen",
    laden: () => import("../berechnungen/Vollgeschosse.svelte"),
  },
  {
    id: "hoehen",
    titel: "Gebäudehöhen / Dachneigung",
    gruppe: "Berechnungen",
    laden: () => import("../berechnungen/Hoehen.svelte"),
  },
  {
    id: "freiflaechen",
    titel: "Versiegelung / Freiflächen",
    gruppe: "Berechnungen",
    laden: () => import("../berechnungen/Freiflaechen.svelte"),
  },
  {
    id: "entwaesserung",
    titel: "Regenwasser / Rückhaltung",
    gruppe: "Berechnungen",
    laden: () => import("../berechnungen/Entwaesserung.svelte"),
  },
  {
    id: "verfahren",
    titel: "Verfahrensakte",
    gruppe: "Organisation",
    laden: () => import("../verfahren/Verfahrensakte.svelte"),
  },
  {
    id: "aktensuche",
    titel: "Suche / Wiedervorlagen",
    gruppe: "Organisation",
    laden: () => import("../verfahren/AktenSuche.svelte"),
  },
  {
    id: "gebaeude",
    titel: "Gebäude / Geschosse",
    gruppe: "Grundlagen",
    laden: () => import("./Gebaeude.svelte"),
  },
  {
    id: "raumbuch",
    titel: "Flächen- und Raumbuch",
    gruppe: "Berechnungen",
    laden: () => import("./Raumbuch.svelte"),
  },
  {
    id: "nachweise",
    titel: "Nachweisstände",
    gruppe: "Berechnungen",
    laden: () => import("./Nachweise.svelte"),
  },
  {
    id: "aufgaben",
    titel: "Aufgaben / Fristen",
    gruppe: "Organisation",
    laden: () => import("./Aufgaben.svelte"),
  },
  {
    id: "unterlagen",
    titel: "Unterlagenregister",
    gruppe: "Unterlagen",
    laden: () => import("./Unterlagen.svelte"),
  },
] as const;
