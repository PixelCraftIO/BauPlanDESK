// Typisierte Hülle um die Tauri-Commands. Das Frontend rechnet nichts selbst;
// alle Fachlogik liegt in core-lib (Rust).
import { invoke } from "@tauri-apps/api/core";

export type Bundesland =
  | "BW" | "BY" | "BE" | "BB" | "HB" | "HH" | "HE" | "MV"
  | "NI" | "NW" | "RP" | "SL" | "SN" | "ST" | "SH" | "TH";

export type Baugebiet =
  | "WS" | "WR" | "WA" | "WB" | "MD" | "MDW" | "MI" | "MU" | "MK" | "GE" | "GI" | "SO";

export interface RulePackInfo {
  land: Bundesland;
  version: string;
  rechtsstand: string;
  origin: string;
}

export interface AppInfo {
  version: string;
  modus: string;
  storage: string;
  regelpakete: RulePackInfo[];
  cache: { eintraege: number; belegt_bytes: number; treffer: number; downloads: number } | null;
}

export interface Project {
  id: string;
  name: string;
  bundesland: Bundesland;
  grundstuecksflaeche_m2: number | null;
  version: number;
  created_at: string;
  updated_at: string;
}

export interface AbstandsflaecheErgebnis {
  tiefe_m: number;
  faktor: number;
  mindesttiefe_m: number;
  rechtsgrundlage: string;
}

export interface Berechnung<T> {
  ergebnis: T;
  regelpaket: RulePackInfo;
}

export interface Projektwerte {
  name: string;
  bundesland: Bundesland;
  grundstuecksflaeche_m2: number | null;
}

export interface DocumentVersion {
  document_id: string;
  project_id: string;
  version: number;
  file_name: string;
  storage_key: string;
  sha256: string;
  size_bytes: number;
  created_by: string;
  created_at: string;
}

export interface ZuletztEintrag {
  id: string;
  name: string;
}

export interface MassDerNutzung {
  grundstuecksflaeche_m2: number;
  grundflaeche_m2: number;
  grundflaeche_nebenanlagen_m2: number;
  geschossflaeche_m2: number;
  baumasse_m3: number | null;
}

export interface Pruefergebnis {
  kennzahlen: { grz: number; grz_gesamt: number; gfz: number; bmz: number | null };
  grenzwerte: { grz: number; gfz: number | null; bmz: number | null };
  grz_gesamt_max: number;
  grz_eingehalten: boolean;
  grz_gesamt_eingehalten: boolean;
  gfz_eingehalten: boolean | null;
  bmz_eingehalten: boolean | null;
}

export type Feldart = "text" | "ankreuzfeld" | "optionsfeld" | "auswahl" | "sonstiges";
export type Feldwert = string | boolean;

export interface Formularfeld {
  name: string;
  beschriftung: string | null;
  art: Feldart;
  wert: Feldwert | null;
  optionen: string[];
  schreibgeschuetzt: boolean;
  max_laenge: number | null;
}

export interface FormularAnalyse {
  dokument: DocumentVersion;
  felder: Formularfeld[];
  vorlage: { id: string; titel: string; version: string } | null;
  vorschlag: Record<string, Feldwert>;
}

export interface CmdError {
  art: string;
  meldung: string;
}

export interface UpdateInfo {
  version: string;
  notizen: string | null;
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  projekte: () => invoke<Project[]>("projekte_auflisten"),
  projektAnlegen: (projekt: Projektwerte) => invoke<Project>("projekt_anlegen", { projekt }),
  projektOeffnen: (id: string) => invoke<Project>("projekt_oeffnen", { id }),
  projektSchliessen: () => invoke<void>("projekt_schliessen"),
  projektAktualisieren: (aenderung: Projektwerte & { id: string; expected_version: number }) => invoke<Project>("projekt_aktualisieren", { aenderung }),
  zuletzt: () => invoke<ZuletztEintrag[]>("zuletzt_geoeffnet"),
  dokumente: (projektId: string) =>
    invoke<DocumentVersion[]>("dokumente_auflisten", { projektId }),
  dokumentImportieren: (projektId: string, pfad: string, dokumentId: string | null = null) =>
    invoke<DocumentVersion>("dokument_importieren", { projektId, dokumentId, pfad }),
  formularAnalysieren: (dokumentId: string) =>
    invoke<FormularAnalyse>("formular_analysieren", { dokumentId }),
  formularAusfuellen: (dokumentId: string, werte: Record<string, Feldwert>) =>
    invoke<DocumentVersion>("formular_ausfuellen", { dokumentId, werte }),
  dokumentExportieren: (dokumentId: string, ziel: string) =>
    invoke<void>("dokument_exportieren", { dokumentId, ziel }),
  massDerNutzung: (eingabe: MassDerNutzung, gebiet: Baugebiet) =>
    invoke<Pruefergebnis>("mass_der_nutzung", { eingabe, gebiet, festsetzungen: null }),
  abstandsflaeche: (land: Bundesland, eingabe: { wandhoehe_m: number; gebiet: Baugebiet }) =>
    invoke<Berechnung<AbstandsflaecheErgebnis>>("abstandsflaeche", { land, eingabe }),
  updatePruefen: () => invoke<UpdateInfo | null>("update_pruefen"),
  updateInstallieren: () => invoke<void>("update_installieren"),
};

export const LAENDER: { code: Bundesland; name: string }[] = [
  { code: "BW", name: "Baden-Württemberg" },
  { code: "BY", name: "Bayern" },
  { code: "BE", name: "Berlin" },
  { code: "BB", name: "Brandenburg" },
  { code: "HB", name: "Bremen" },
  { code: "HH", name: "Hamburg" },
  { code: "HE", name: "Hessen" },
  { code: "MV", name: "Mecklenburg-Vorpommern" },
  { code: "NI", name: "Niedersachsen" },
  { code: "NW", name: "Nordrhein-Westfalen" },
  { code: "RP", name: "Rheinland-Pfalz" },
  { code: "SL", name: "Saarland" },
  { code: "SN", name: "Sachsen" },
  { code: "ST", name: "Sachsen-Anhalt" },
  { code: "SH", name: "Schleswig-Holstein" },
  { code: "TH", name: "Thüringen" },
];

export const GEBIETE: { code: Baugebiet; name: string }[] = [
  { code: "WS", name: "Kleinsiedlungsgebiet" },
  { code: "WR", name: "Reines Wohngebiet" },
  { code: "WA", name: "Allgemeines Wohngebiet" },
  { code: "WB", name: "Besonderes Wohngebiet" },
  { code: "MD", name: "Dorfgebiet" },
  { code: "MDW", name: "Dörfliches Wohngebiet" },
  { code: "MI", name: "Mischgebiet" },
  { code: "MU", name: "Urbanes Gebiet" },
  { code: "MK", name: "Kerngebiet" },
  { code: "GE", name: "Gewerbegebiet" },
  { code: "GI", name: "Industriegebiet" },
  { code: "SO", name: "Sondergebiet (Erholung)" },
];

export function landName(code: Bundesland): string {
  return LAENDER.find((l) => l.code === code)?.name ?? code;
}

export function istKonflikt(e: unknown): boolean {
  return !!e && typeof e === "object" && (e as CmdError).art === "konflikt";
}

export function fehlertext(e: unknown): string {
  if (e && typeof e === "object" && "meldung" in e) return (e as CmdError).meldung;
  return String(e);
}
