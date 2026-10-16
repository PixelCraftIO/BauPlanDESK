import { FACHMODULE } from "./fachmodule/register";
import type { Project } from "./api";
export type ModulGruppe =
  | "Grundlagen"
  | "Berechnungen"
  | "Grundstücksinformationen"
  | "Unterlagen"
  | "Organisation";
export interface FachModul {
  id: string;
  titel: string;
  gruppe: ModulGruppe;
  voraussetzung?: "flaechen" | "plan";
}
const BASIS_MODULE = [
  { id: "projektuebersicht", titel: "Projektübersicht", gruppe: "Grundlagen" },
  { id: "uebersicht", titel: "Stammdaten", gruppe: "Grundlagen" },
  { id: "rechtsgrundlagen", titel: "Rechtsgrundlagen", gruppe: "Grundlagen" },
  {
    id: "festsetzungen",
    titel: "Bebauungsplanfestsetzungen",
    gruppe: "Grundlagen",
    voraussetzung: "plan",
  },
  {
    id: "flurstuecke",
    titel: "Flurstücke und Grundbuch",
    gruppe: "Grundstücksinformationen",
  },
  { id: "grundstueck", titel: "Baugrundstück", gruppe: "Berechnungen" },
  {
    id: "grz",
    titel: "GRZ",
    gruppe: "Berechnungen",
    voraussetzung: "flaechen",
  },
  {
    id: "gfz",
    titel: "GFZ",
    gruppe: "Berechnungen",
    voraussetzung: "flaechen",
  },
  {
    id: "geschosse",
    titel: "Geschossigkeit",
    gruppe: "Berechnungen",
    voraussetzung: "flaechen",
  },
  {
    id: "volumen",
    titel: "Bauvolumen / BMZ",
    gruppe: "Berechnungen",
    voraussetzung: "flaechen",
  },
  { id: "abstand", titel: "Abstandsflächen", gruppe: "Berechnungen" },
  { id: "formulare", titel: "Formulare", gruppe: "Unterlagen" },
  { id: "dokumente", titel: "Projektdateien", gruppe: "Unterlagen" },
  { id: "plaene", titel: "Pläne / Aufmaß", gruppe: "Unterlagen" },
  { id: "bauantrag", titel: "Bauantrag / XBau", gruppe: "Unterlagen" },
  { id: "druck", titel: "Drucken / PDF-Ausgabe", gruppe: "Unterlagen" },
  { id: "einreichungen", titel: "Einreichungen", gruppe: "Unterlagen" },
  { id: "protokoll", titel: "Chronik", gruppe: "Unterlagen" },
] as const satisfies readonly FachModul[];
export const MODULGRUPPEN = [
  "Grundlagen",
  "Grundstücksinformationen",
  "Berechnungen",
  "Organisation",
  "Unterlagen",
] as const;
export const MODULE = [...BASIS_MODULE, ...FACHMODULE] as const;
export type ModulId = (typeof MODULE)[number]["id"];
export function voraussetzung(
  projekt: Project,
  modul: FachModul,
): string | null {
  if (
    modul.voraussetzung === "plan" &&
    (projekt.planungsrecht.rechtsgrundlage !== "par30" ||
      ["kein", "ungeklaert"].includes(projekt.planungsrecht.bebauungsplan_art))
  )
    return "Nur bei vorhandenem Bebauungsplan";
  if (
    modul.voraussetzung === "flaechen" &&
    (!projekt.planungsrecht.rechtsgrundlage || !projekt.planungsrecht.bau_nvo)
  )
    return "Zuerst Rechtsgrundlage und BauNVO festlegen";
  return null;
}
