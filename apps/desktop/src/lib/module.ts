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
  {
    id: "einreichen",
    titel: "Einreichen / Nachreichen",
    gruppe: "Organisation",
  },
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
  { id: "portale", titel: "Einreichen / Portal", gruppe: "Unterlagen" },
  { id: "xbau", titel: "XBau-Nachrichten", gruppe: "Unterlagen" },
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
/** Erfassung bleibt möglich; dieser Hinweis sperrt nur fachliche Auswertung. */
export function berechnungsvoraussetzung(projekt: Project): string | null {
  const fehlt: string[] = [];
  if (!projekt.planungsrecht.rechtsgrundlage)
    fehlt.push("die Rechtsgrundlage (§ 30, § 34 oder § 35 BauGB)");
  if (!projekt.planungsrecht.bau_nvo)
    fehlt.push("den für diesen Nachweis maßgebenden BauNVO-Stand");
  return fehlt.length
    ? `Unter Grundlagen → Rechtsgrundlagen ${fehlt.join(" und ")} klären und speichern. Bei § 34 keine beliebige BauNVO auswählen; den Berechnungsmaßstab fachlich klären. Bis dahin sind nur Entwürfe möglich.`
    : null;
}
export function voraussetzung(
  projekt: Project,
  modul: FachModul,
): string | null {
  if (
    modul.voraussetzung === "plan" &&
    (projekt.planungsrecht.rechtsgrundlage !== "par30" ||
      ["kein", "ungeklaert"].includes(projekt.planungsrecht.bebauungsplan_art))
  )
    return "Unter Grundlagen → Rechtsgrundlagen § 30 BauGB und die vorhandene Bebauungsplanart festlegen und speichern. Ohne Bebauungsplan sind Planfestsetzungen nicht anwendbar.";
  if (
    modul.voraussetzung === "flaechen" &&
    (!projekt.planungsrecht.rechtsgrundlage || !projekt.planungsrecht.bau_nvo)
  )
    return berechnungsvoraussetzung(projekt);
  if (
    modul.voraussetzung === "flaechen" &&
    !(
      projekt.planungsrecht.grundstuecksinformationen?.flurstuecke ??
      projekt.planungsrecht.baugrundstuecke?.flurstuecke ??
      []
    ).length
  )
    return "Unter Grundlagen → Flurstücke mindestens ein Vorhabenflurstück erfassen und speichern. Anschließend unter Berechnungen → Baugrundstück die Zuordnung und Bezugsflächen prüfen und speichern. Danach können die Nutzungswerte ermittelt werden.";
  return null;
}
