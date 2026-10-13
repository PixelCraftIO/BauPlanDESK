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
  projektAnlegen: (projekt: {
    name: string;
    bundesland: Bundesland;
    grundstuecksflaeche_m2: number | null;
  }) => invoke<Project>("projekt_anlegen", { projekt }),
  abstandsflaeche: (land: Bundesland, eingabe: { wandhoehe_m: number; gebiet: Baugebiet }) =>
    invoke<Berechnung<AbstandsflaecheErgebnis>>("abstandsflaeche", { land, eingabe }),
  updatePruefen: () => invoke<UpdateInfo | null>("update_pruefen"),
  updateInstallieren: () => invoke<void>("update_installieren"),
};

export function fehlertext(e: unknown): string {
  if (e && typeof e === "object" && "meldung" in e) return (e as CmdError).meldung;
  return String(e);
}
