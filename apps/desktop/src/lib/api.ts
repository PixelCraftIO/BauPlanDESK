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

export type Rolle = "lesen" | "bearbeitung" | "projektleitung" | "administrator";

export const ROLLEN: { code: Rolle; name: string; beschreibung: string }[] = [
  { code: "lesen", name: "Lesen", beschreibung: "Projekte und Dokumente ansehen, rechnen" },
  { code: "bearbeitung", name: "Bearbeitung", beschreibung: "zusätzlich Projektdaten ändern, Dokumente ablegen und sperren" },
  { code: "projektleitung", name: "Projektleitung", beschreibung: "zusätzlich Projekte anlegen, Sperren anderer aufheben" },
  { code: "administrator", name: "Administrator", beschreibung: "zusätzlich Benutzer verwalten" },
];

export const rollenName = (r: Rolle) => ROLLEN.find((x) => x.code === r)?.name ?? r;

/** Nur für die Oberfläche (Knöpfe ausgrauen); geprüft wird in core-lib. */
export const mindestens = (r: Rolle, min: Rolle) =>
  ROLLEN.findIndex((x) => x.code === r) >= ROLLEN.findIndex((x) => x.code === min);

export interface Benutzer {
  id: string;
  anmeldename: string;
  anzeigename: string;
  rolle: Rolle;
  aktiv: boolean;
  erstellt_am: string;
}

export interface DocumentLock {
  document_id: string;
  project_id: string;
  benutzer_id: string;
  benutzer_name: string;
  seit: string;
}

export interface Protokolleintrag {
  id: number;
  zeit: string;
  benutzer_id: string;
  benutzer_name: string;
  aktion: string;
  project_id: string | null;
  document_id: string | null;
  details: string;
}

export const AKTIONEN: Record<string, string> = {
  projekt_angelegt: "Projekt angelegt",
  projekt_geaendert: "Projektdaten geändert",
  dokument_abgelegt: "Dokument abgelegt",
  dokument_gesperrt: "Dokument gesperrt",
  dokument_entsperrt: "Sperre aufgehoben",
  sperre_aufgehoben: "Fremde Sperre aufgehoben",
  formular_ausgefuellt: "Formular ausgefüllt",
  benutzer_angelegt: "Benutzer angelegt",
  benutzer_geaendert: "Benutzer geändert",
  lizenz_eingespielt: "Lizenz eingespielt",
};

export type Edition = "einzelplatz" | "buero" | "cloud";
export const EDITIONEN: Record<Edition, string> = { einzelplatz: "Einzelplatz", buero: "Büro", cloud: "Cloud" };

export interface Lizenz {
  lizenz_id: string;
  lizenznehmer: string;
  edition: Edition;
  arbeitsplaetze: number;
  ausgestellt_am: string;
  gueltig_bis: string | null;
  wartung_bis: string | null;
}

export interface Arbeitsplatzbelegung {
  arbeitsplatz: string;
  benutzer_name: string;
  belegt_seit: string;
  letzte_meldung: string;
}

export type Lizenzzustand =
  | { art: "lizenziert" }
  | { art: "testversion"; tage_uebrig: number }
  | { art: "testversion_abgelaufen" }
  | { art: "abgelaufen"; seit: string }
  | { art: "ungueltig"; grund: string }
  | { art: "kein_arbeitsplatz" };

export interface Lizenzstatus {
  zustand: Lizenzzustand;
  schreiben_erlaubt: boolean;
  meldung: string;
  lizenz: Lizenz | null;
  arbeitsplaetze_max: number;
  arbeitsplaetze: Arbeitsplatzbelegung[];
  eigener_arbeitsplatz: string | null;
}

/** Datum "JJJJ-MM-TT" deutsch anzeigen. */
export const datumDe = (d: string) => new Date(`${d}T00:00:00`).toLocaleDateString("de-DE");

export interface AppInfo {
  version: string;
  benutzer: Benutzer;
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

export type Laengeneinheit =
  | "millimeter" | "zentimeter" | "dezimeter" | "meter" | "kilometer" | "zoll" | "fuss";

/** Punkt in Zeichnungseinheiten als [x, y]. */
export type Punkt = [number, number];

export interface Plan {
  einheit: Laengeneinheit | null;
  ebenen: { name: string; farbe: string | null; sichtbar: boolean }[];
  linien: { ebene: number; farbe: string | null; punkte: Punkt[]; geschlossen: boolean }[];
  texte: {
    ebene: number;
    farbe: string | null;
    position: Punkt;
    hoehe: number;
    zeilenabstand: number;
    drehung: number;
    ausrichtung: "links" | "mitte" | "rechts";
    inhalt: string;
  }[];
  grenzen: { min: Punkt; max: Punkt } | null;
  hinweise: string[];
}

export interface CadPlan {
  dokument: DocumentVersion;
  plan: Plan;
}

export interface Messung {
  laenge_m: number;
  flaeche_m2: number | null;
  selbstschneidend: boolean;
}

export const EINHEITEN: { code: Laengeneinheit; name: string }[] = [
  { code: "meter", name: "Meter" },
  { code: "zentimeter", name: "Zentimeter" },
  { code: "millimeter", name: "Millimeter" },
  { code: "dezimeter", name: "Dezimeter" },
  { code: "kilometer", name: "Kilometer" },
  { code: "zoll", name: "Zoll" },
  { code: "fuss", name: "Fuß" },
];

/** Dateien, die der Plan-Viewer öffnen kann. */
export const istPlan = (dateiname: string) => /\.dxf$/i.test(dateiname);

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
  dokumentImportieren: (projektId: string, pfad: string, dokumentId: string | null = null, basis: number | null = null) =>
    invoke<DocumentVersion>("dokument_importieren", { projektId, dokumentId, basis, pfad }),
  historie: (dokumentId: string) => invoke<DocumentVersion[]>("dokument_historie", { dokumentId }),
  versionWiederherstellen: (dokumentId: string, version: number) =>
    invoke<DocumentVersion>("version_wiederherstellen", { dokumentId, version }),
  sperren: (dokumentId: string) => invoke<DocumentLock>("dokument_sperren", { dokumentId }),
  entsperren: (dokumentId: string) => invoke<void>("dokument_entsperren", { dokumentId }),
  sperrenAuflisten: (projektId: string) => invoke<DocumentLock[]>("sperren_auflisten", { projektId }),
  protokoll: (projektId: string) => invoke<Protokolleintrag[]>("protokoll", { projektId }),
  benutzerAuflisten: () => invoke<Benutzer[]>("benutzer_auflisten"),
  benutzerAnlegen: (neu: { anmeldename: string; anzeigename: string; rolle: Rolle }) =>
    invoke<Benutzer>("benutzer_anlegen", { neu }),
  benutzerAendern: (aenderung: { id: string; anzeigename: string; rolle: Rolle; aktiv: boolean }) =>
    invoke<Benutzer>("benutzer_aendern", { aenderung }),
  lizenz: () => invoke<Lizenzstatus>("lizenz_status"),
  lizenzEinspielen: (pfad: string) => invoke<Lizenzstatus>("lizenz_einspielen", { pfad }),
  arbeitsplatzBelegen: () => invoke<Lizenzstatus>("arbeitsplatz_belegen"),
  formularAnalysieren: (dokumentId: string) =>
    invoke<FormularAnalyse>("formular_analysieren", { dokumentId }),
  formularAusfuellen: (dokumentId: string, werte: Record<string, Feldwert>) =>
    invoke<DocumentVersion>("formular_ausfuellen", { dokumentId, werte }),
  dokumentExportieren: (dokumentId: string, ziel: string, version: number | null = null) =>
    invoke<void>("dokument_exportieren", { dokumentId, version, ziel }),
  cadPlan: (dokumentId: string) => invoke<CadPlan>("cad_plan", { dokumentId }),
  aufmass: (punkte: Punkt[], geschlossen: boolean, einheit: Laengeneinheit) =>
    invoke<Messung>("aufmass", { punkte, geschlossen, einheit }),
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
