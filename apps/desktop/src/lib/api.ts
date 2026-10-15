import type { BauNvo, Bauteiltyp, Anrechnung } from "./generated/plattform";
export type { BauNvo, Bauteiltyp, Anrechnung } from "./generated/plattform";
// Typisierte Hülle um die Tauri-Commands. Das Frontend rechnet nichts selbst;
// alle Fachlogik liegt in core-lib (Rust).
import { invoke } from "@tauri-apps/api/core";

export type Bundesland =
  | "BW"
  | "BY"
  | "BE"
  | "BB"
  | "HB"
  | "HH"
  | "HE"
  | "MV"
  | "NI"
  | "NW"
  | "RP"
  | "SL"
  | "SN"
  | "ST"
  | "SH"
  | "TH";

export type Baugebiet =
  | "WS"
  | "WR"
  | "WA"
  | "WB"
  | "MD"
  | "MDW"
  | "MI"
  | "MU"
  | "MK"
  | "GE"
  | "GI"
  | "SO";

export interface RulePackInfo {
  land: Bundesland;
  version: string;
  rechtsstand: string;
  origin: string;
}

export type Rolle =
  | "lesen"
  | "bearbeitung"
  | "projektleitung"
  | "administrator";

export const ROLLEN: { code: Rolle; name: string; beschreibung: string }[] = [
  {
    code: "lesen",
    name: "Lesen",
    beschreibung: "Projekte und Dokumente ansehen, rechnen",
  },
  {
    code: "bearbeitung",
    name: "Bearbeitung",
    beschreibung:
      "zusätzlich Projektdaten ändern, Dokumente ablegen und sperren",
  },
  {
    code: "projektleitung",
    name: "Projektleitung",
    beschreibung: "zusätzlich Projekte anlegen, Sperren anderer aufheben",
  },
  {
    code: "administrator",
    name: "Administrator",
    beschreibung: "zusätzlich Benutzer verwalten",
  },
];

export const rollenName = (r: Rolle) =>
  ROLLEN.find((x) => x.code === r)?.name ?? r;

/** Nur für die Oberfläche (Knöpfe ausgrauen); geprüft wird in core-lib. */
export const mindestens = (r: Rolle, min: Rolle) =>
  ROLLEN.findIndex((x) => x.code === r) >=
  ROLLEN.findIndex((x) => x.code === min);

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
  ausgabeauftrag_geaendert: "Ordnerausgabe aktualisiert",
  projekt_geaendert: "Projektdaten geändert",
  dokument_abgelegt: "Dokument abgelegt",
  dokument_gesperrt: "Dokument gesperrt",
  dokument_entsperrt: "Sperre aufgehoben",
  sperre_aufgehoben: "Fremde Sperre aufgehoben",
  formular_ausgefuellt: "Formular ausgefüllt",
  benutzer_angelegt: "Benutzer angelegt",
  benutzer_geaendert: "Benutzer geändert",
  lizenz_eingespielt: "Lizenz eingespielt",
  bauantrag_geaendert: "Bauantrag geändert",
  einreichung_erstellt: "Einreichungspaket erstellt",
  einreichung_bestaetigt: "Einreichung bestätigt",
  xbau_exportiert: "XBau-Bauantrag exportiert",
  kunde_angelegt: "Kunde angelegt",
  kunde_geaendert: "Kunde geändert",
  kunde_geloescht: "Kunde gelöscht",
  bauherren_geaendert: "Bauherren geändert",
};

// --- XBau ------------------------------------------------------------------

export interface Anschrift {
  strasse: string;
  hausnummer: string;
  postleitzahl: string;
  ort: string;
}

export const leereAnschrift = (): Anschrift => ({
  strasse: "",
  hausnummer: "",
  postleitzahl: "",
  ort: "",
});

export type Beteiligter =
  | {
      art: "person";
      id: string;
      vorname: string;
      familienname: string;
      anschrift: Anschrift;
    }
  | { art: "organisation"; id: string; name: string; anschrift: Anschrift };

export interface Flurstueck {
  gemarkung: string;
  flur: string;
  zaehler: string;
  nenner: string;
}

export interface Anlagenwahl {
  dokument: string;
  anlageart: string;
  bezeichnung: string;
}

export interface Bauantrag {
  vorgang: string;
  bauaufsicht: { name: string; kennung: string };
  bezeichnung: string;
  gebuehrenpflichtiger: Beteiligter | null;
  massnahmen: string[];
  anzahl_gebaeude: number;
  grundstueck: { anschrift: Anschrift; flurstuecke: Flurstueck[] };
  anlagen: Anlagenwahl[];
}

export interface GespeicherterBauantrag {
  antrag: Bauantrag;
  version: number;
  geaendert_von: string | null;
  geaendert_am: string | null;
  fehlende_angaben: string[];
}

// --- Kunden ------------------------------------------------------------------

export type Kundentyp = "person" | "organisation";

export type Kundenkategorie =
  | "privat"
  | "bautraeger"
  | "baufirma"
  | "investor"
  | "planungsbuero"
  | "wohnungsunternehmen"
  | "oeffentliche_hand"
  | "sonstige";

export const KUNDENKATEGORIEN: { code: Kundenkategorie; name: string }[] = [
  { code: "privat", name: "Privatperson" },
  { code: "bautraeger", name: "Bauträger" },
  { code: "baufirma", name: "Baufirma" },
  { code: "investor", name: "Investor" },
  { code: "planungsbuero", name: "Planungsbüro" },
  { code: "wohnungsunternehmen", name: "Wohnungsunternehmen" },
  { code: "oeffentliche_hand", name: "Öffentliche Hand" },
  { code: "sonstige", name: "Sonstige" },
];

export const kategorieName = (k: Kundenkategorie) =>
  KUNDENKATEGORIEN.find((x) => x.code === k)?.name ?? k;

export interface Kundendaten {
  typ: Kundentyp;
  kategorie: Kundenkategorie;
  kundennummer: string;
  vorname: string;
  familienname: string;
  firma: string;
  ansprechpartner: string;
  anschrift: Anschrift;
  telefon: string;
  email: string;
  notizen: string;
}

export interface Kunde extends Kundendaten {
  id: string;
  version: number;
  created_at: string;
  updated_at: string;
}

export const leereKundendaten = (): Kundendaten => ({
  typ: "person",
  kategorie: "privat",
  kundennummer: "",
  vorname: "",
  familienname: "",
  firma: "",
  ansprechpartner: "",
  anschrift: leereAnschrift(),
  telefon: "",
  email: "",
  notizen: "",
});

/** Nur die bearbeitbaren Angaben eines Kunden. */
export function kundendaten(k: Kundendaten): Kundendaten {
  const {
    typ,
    kategorie,
    kundennummer,
    vorname,
    familienname,
    firma,
    ansprechpartner,
    anschrift,
    telefon,
    email,
    notizen,
  } = k;
  return {
    typ,
    kategorie,
    kundennummer,
    vorname,
    familienname,
    firma,
    ansprechpartner,
    anschrift: { ...anschrift },
    telefon,
    email,
    notizen,
  };
}

export function kundenname(k: Kundendaten): string {
  return k.typ === "person"
    ? [k.vorname, k.familienname].filter((s) => s.trim()).join(" ")
    : k.firma;
}

/** Eine Zeile Anschrift, z. B. „Lindenweg 4, 80331 München“. */
export function anschriftZeile(a: Anschrift): string {
  const strasse = `${a.strasse} ${a.hausnummer}`.trim();
  const ort = `${a.postleitzahl} ${a.ort}`.trim();
  return [strasse, ort].filter(Boolean).join(", ");
}

/** Suche über Name, Kundennummer, Ansprechpartner, Ort, E-Mail und Telefon. */
export function kundePasst(k: Kunde, suche: string): boolean {
  const s = suche.trim().toLocaleLowerCase("de");
  if (!s) return true;
  return [
    kundenname(k),
    k.kundennummer,
    k.ansprechpartner,
    k.anschrift.ort,
    k.anschrift.postleitzahl,
    k.email,
    k.telefon,
  ].some((f) => f.toLocaleLowerCase("de").includes(s));
}

/** Kunde als Beteiligter im Bauantrag (z. B. als Gebührenpflichtiger). */
export function alsBeteiligter(k: Kunde): Beteiligter {
  return k.typ === "person"
    ? {
        art: "person",
        id: k.id,
        vorname: k.vorname,
        familienname: k.familienname,
        anschrift: { ...k.anschrift },
      }
    : {
        art: "organisation",
        id: k.id,
        name: k.firma,
        anschrift: { ...k.anschrift },
      };
}

export interface Buero {
  name: string;
  anschrift: Anschrift;
  kennung: string;
}

export interface Codeliste {
  uri: string;
  version: string;
  name: string;
  eintraege: { code: string; beschreibung: string }[];
}

export const CODELISTE_ANLAGEN = "urn:xoev-de:xbau:codeliste:anlagen";

/** Art der Maßnahme (XBau-interne Codeliste, siehe core-lib xbau::MASSNAHMEN). */
export const MASSNAHMEN: { code: string; name: string }[] = [
  { code: "10", name: "Baumaßnahme" },
  { code: "11", name: "Errichtung" },
  { code: "12", name: "Änderung" },
  { code: "13", name: "Nutzungsänderung (ohne bauliche Änderung)" },
  { code: "14", name: "Nutzungsänderung (mit baulicher Änderung)" },
  { code: "20", name: "Abgrabung" },
  { code: "30", name: "Beseitigung" },
];

export type Edition = "einzelplatz" | "buero" | "cloud";
export const EDITIONEN: Record<Edition, string> = {
  einzelplatz: "Einzelplatz",
  buero: "Büro",
  cloud: "Cloud",
};

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
  | { art: "kein_arbeitsplatz" }
  | { art: "nicht_bestaetigt"; grund: string }
  | { art: "bestaetigung_abgelaufen"; seit: string };

/** Abo-Lizenz über den Lizenzserver. */
export interface AboStatus {
  lizenzschluessel: string;
  bestaetigt_um: string | null;
  offline_gueltig_bis: string | null;
}

export interface Lizenzstatus {
  zustand: Lizenzzustand;
  schreiben_erlaubt: boolean;
  meldung: string;
  lizenz: Lizenz | null;
  arbeitsplaetze_max: number;
  arbeitsplaetze: Arbeitsplatzbelegung[];
  eigener_arbeitsplatz: string | null;
  abo: AboStatus | null;
}

/** Datum "JJJJ-MM-TT" deutsch anzeigen. */
export const datumDe = (d: string) =>
  new Date(`${d}T00:00:00`).toLocaleDateString("de-DE");

export interface AppInfo {
  version: string;
  benutzer: Benutzer;
  modus: string;
  storage: string;
  regelpakete: RulePackInfo[];
  cache: {
    eintraege: number;
    belegt_bytes: number;
    treffer: number;
    downloads: number;
  } | null;
}

export interface Festsetzungen {
  nichtvollgeschoss_faktor: number | null;
  nichtvollgeschoss_festsetzung: string;
  baugebiet: Baugebiet | null;
  grz: number | null;
  grz_gesamt: number | null;
  gfz: number | null;
  bmz: number | null;
  grundflaeche_m2: number | null;
  geschossflaeche_m2: number | null;
  ueberbaubare_flaeche_m2: number | null;
  vollgeschosse: number | null;
  geschossigkeit: "maximal" | "zwingend";
}

export interface Bauteil {
  bezeichnung: string;
  typ: Bauteiltyp;
  flaeche_m2: number;
  abweichende_anrechnung: Anrechnung | null;
  begruendung: string;
}
export interface Geschoss {
  gebaeude_id?: string | null;
  geschoss_id?: string | null;
  bezeichnung: string;
  vollgeschoss: boolean;
  aussenmass_m2: number;
  aufenthalt_m2: number;
  waende_m2: number;
  erschliessung_m2: number;
}
export interface Gemeinschaftsanteil {
  bezeichnung: string;
  gesamtflaeche_m2: number;
  anteil_prozent: number;
  festsetzung: string;
  im_plangebiet: boolean;
  ausserhalb_baugrundstueck: boolean;
}
export interface GrundstueckAufstellung {
  basisflaeche_m2: number;
  nicht_massgebend_m2: number;
  abzug_begruendung: string;
  gemeinschaftsanteile: Gemeinschaftsanteil[];
}
export interface GrundstueckErgebnis {
  basis_m2: number;
  abzug_m2: number;
  zuschlag_m2: number;
  massgebend_m2: number;
  anteile_m2: number[];
}
export interface FlaechenAufstellung {
  grundstueck?: GrundstueckAufstellung | null;
  bau_nvo: BauNvo;
  bauteile: Bauteil[];
  geschosse: Geschoss[];
  nichtvollgeschoss_faktor: number;
  nichtvollgeschoss_festsetzung: string;
}
export interface FlaechenErgebnis {
  bauteile_vorhanden: boolean;
  geschosse_vorhanden: boolean;
  regelwerk_id: string;
  regelwerk_version: string;
  grundstueck: GrundstueckErgebnis;
  mass: MassDerNutzung;
  vollgeschosse: number;
  vollgeschosse_je_gebaeude: Record<string, number>;
  grz_ii: boolean;
  bauteile: { anrechnung: Anrechnung; flaeche_m2: number }[];
  geschosse: number[];
}
export interface FlaechenAbzug {
  art: string;
  flaeche_m2: number;
  grundlage: string;
}
export interface GrundstuecksFlurstueck {
  id: string;
  gemarkung: string;
  flur: string;
  nummer: string;
  grundbuchbezirk: string;
  grundbuchblatt: string;
  laufende_nummer: number;
  flaeche_m2: number;
  abzuege: FlaechenAbzug[];
  abweichende_zuordnung: { kennung: string; grundlage: string } | null;
}
export interface BaugrundstueckBerechnung {
  schluessel: string;
  eingabe: NutzungsEingabe;
  eigene_festsetzungen: Festsetzungen | null;
}
export interface BaugrundstuecksPlan {
  flurstuecke: GrundstuecksFlurstueck[];
  berechnungen: BaugrundstueckBerechnung[];
}
export interface BaugrundstueckGruppe {
  schluessel: string;
  bezeichnung: string;
  flurstueck_ids: string[];
  eigentumsflaeche_m2: number;
  abzuege_m2: number;
  bauland_m2: number;
}
export interface BaugrundstuecksPruefung {
  projekt_version: number;
  baugrundstuecke: {
    grundstueck: BaugrundstueckGruppe;
    pruefung: NutzungsPruefung;
  }[];
}
export interface NutzungsEingabe {
  aufstellung?: FlaechenAufstellung | null;
  mass: MassDerNutzung;
  vollgeschosse: number | null;
  gebiet: Baugebiet;
}

export interface NutzungsPruefung {
  aufstellung: FlaechenErgebnis | null;
  projekt_version: number;
  kennzahlen: Pruefergebnis["kennzahlen"];
  bebauungsplan: boolean;
  nachweise: {
    art: string;
    gegenstand?: string | null;
    ist: number | null;
    soll: number | null;
    quelle: "festsetzung" | "orientierungswert" | "bau_nvo19" | "keine";
    zwingend: boolean;
    eingehalten: boolean | null;
    abweichung: number | null;
    ueberschreitung: number | null;
    ueberschreitung_prozent: number | null;
    flaechenueberschreitung_m2: number | null;
  }[];
}

export interface BaugrenzenGeometrie {
  dokument_id: string;
  dokument_version: number;
  einheit: Laengeneinheit;
  baufenster: Punkt[];
  vorhaben: Punkt[];
}

export interface Aussenflaeche {
  aussenring: Punkt[];
  innenringe: Punkt[][];
}
export interface BaugrenzenPruefung {
  baufenster_m2: number;
  vorhaben_m2: number;
  ausserhalb_m2: number;
  ausserhalb_prozent: number;
  eckpunkt_ueberstand_m: number;
  eingehalten: boolean;
  aussenflaechen: Aussenflaeche[];
}

export interface Planungsrecht {
  bau_nvo?: BauNvo | null;
  rechtsgrundlage: "par30" | "par34" | "par35" | null;
  bebauungsplan_art:
    | "ungeklaert"
    | "kein"
    | "qualifiziert"
    | "einfach"
    | "vorhabenbezogen";
  ergaenzende_grundlage: "par34" | "par35" | null;
  aussenbereich_vorhaben: "privilegiert" | "sonstiges" | null;
  plan_bezeichnung: string;
  plan_nummer: string;
  plan_rechtskraft: string;
  gemeinde: string;
  standort: string;
  vorhaben: string;
  nutzung: string;
  festsetzungen: string;
  begruendung: string;
  festgesetzte_werte: Festsetzungen;
  berechnung: NutzungsEingabe | null;
  baugrundstuecke: BaugrundstuecksPlan | null;
  grundstuecksinformationen?: import('./generated/plattform').Grundstuecksinformationen | null;
  baugrundstuecksermittlung?: import('./generated/plattform').BaugrundstuecksErmittlung | null;
  baugrenzen: BaugrenzenGeometrie | null;
}

export const rechtsgrundlageName = (g: Planungsrecht["rechtsgrundlage"]) =>
  g
    ? { par30: "§ 30 BauGB", par34: "§ 34 BauGB", par35: "§ 35 BauGB" }[g]
    : "Noch nicht festgelegt";

export const bebauungsplanName = (art: Planungsrecht["bebauungsplan_art"]) =>
  ({
    ungeklaert: "Noch ungeklärt",
    kein: "Kein Bebauungsplan",
    qualifiziert: "Qualifizierter Bebauungsplan",
    einfach: "Einfacher Bebauungsplan",
    vorhabenbezogen: "Vorhabenbezogener Bebauungsplan",
  })[art];

export interface Project {
  id: string;
  name: string;
  bundesland: Bundesland;
  grundstuecksflaeche_m2: number | null;
  version: number;
  created_at: string;
  updated_at: string;
  planungsrecht: Planungsrecht;
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
  projektordner_pfad?: string | null;
  ablage_hinweis?: string | null;
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
  kennzahlen: {
    grz: number;
    grz_gesamt: number;
    gfz: number;
    bmz: number | null;
  };
  grenzwerte: { grz: number; gfz: number | null; bmz: number | null };
  grz_gesamt_max: number;
  grz_eingehalten: boolean;
  grz_gesamt_eingehalten: boolean;
  gfz_eingehalten: boolean | null;
  bmz_eingehalten: boolean | null;
}

export type Feldart =
  | "text"
  | "ankreuzfeld"
  | "optionsfeld"
  | "auswahl"
  | "sonstiges";
export type Feldwert = string | boolean;

export interface Formularfeld {
  name: string;
  beschriftung: string | null;
  art: Feldart;
  wert: Feldwert | null;
  optionen: string[];
  schreibgeschuetzt: boolean;
  max_laenge: number | null;
  mehrzeilig?: boolean;
}

export interface FormularBezug {
  grundstueck_id: string | null;
  bauherr_id: string | null;
}
export interface FormularDatenstand {
  projekt_id: string;
  projekt_version: number;
  bezug: FormularBezug;
  daten_hash: string;
  werte: Record<string, string>;
}
export interface FormularQuellen {
  datenstand: FormularDatenstand;
  grundstuecke: { id: string; bezeichnung: string }[];
  bauherren: { id: string; bezeichnung: string }[];
  hinweise: string[];
}
export interface FormularAnalyse {
  quellen?: FormularQuellen;
  dokument: DocumentVersion;
  felder: Formularfeld[];
  vorlage: { id: string; titel: string; version: string } | null;
  vorschlag: Record<string, Feldwert>;
}

export type Laengeneinheit =
  | "millimeter"
  | "zentimeter"
  | "dezimeter"
  | "meter"
  | "kilometer"
  | "zoll"
  | "fuss";

/** Punkt in Zeichnungseinheiten als [x, y]. */
export type Punkt = [number, number];

export interface Plan {
  einheit: Laengeneinheit | null;
  ebenen: { name: string; farbe: string | null; sichtbar: boolean }[];
  linien: {
    ebene: number;
    farbe: string | null;
    punkte: Punkt[];
    geschlossen: boolean;
  }[];
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

export interface PdfSeite {
  nummer: number;
  breite_mm: number;
  hoehe_mm: number;
  rotation: number;
  platzierbar: boolean;
}
export interface EigenesPdfFeld {
  name: string;
  beschriftung: string;
  art: "text" | "ankreuzfeld";
  seite: number;
  x_mm: number;
  y_mm: number;
  breite_mm: number;
  hoehe_mm: number;
  schriftgroesse_pt: number;
  mehrzeilig?: boolean;
}
export interface PdfVorschauDaten {
  dokument: DocumentVersion;
  pdf_base64: string;
  seiten: PdfSeite[];
  felder: Formularfeld[];
}
export interface Sicherungsmanifest {
  schema_version: number;
  erstellt_unix: number;
  dateien: { name: string; sha256: string; bytes: number }[];
}
export interface AblageStatus {
  modus: string;
  root: string | null;
  datenbank: string;
  wechsel_erlaubt: boolean;
  hinweis: string;
}
export interface ProjektordnerExport {
  ordner: string;
  dateien: number;
  neu: number;
}

export interface BibliotheksVorlage {
  felder?: number | null;
  name: string;
  sha256: string;
  size: number;
}
export interface Formularbibliothek {
  root: string;
  vorlagen: BibliotheksVorlage[];
}
export type {
  Dokumentkategorie,
  KategorienKatalog,
  KategorieZuordnung,
  DokumentKategorie,
  KategorieStand,
} from "./generated/plattform";
export interface BibliothekVorschau {
  pdf_base64: string;
  seiten: PdfSeite[];
  felder: Formularfeld[];
}
export interface Arbeitskopie {
  stand: { id: string; dokument: DocumentVersion };
  pfad: string;
  geaendert: boolean;
}
export interface PaketPruefstand {
  schema_version: number;
  ermittelt_am: string;
  nicht_enthaltene_pflichtunterlagen: string[];
  ungepruefte_unterlagen: string[];
  hinweise: string[];
  nachweise: import("./generated/plattform").NachweisAnsicht[];
}
export interface Einreichung {
  stand: {
    pruefstand?: PaketPruefstand | null;
    kategorien?: Record<string, import("./generated/plattform").KategorieStand | null>;
    id: string;
    projekt_id: string;
    titel: string;
    empfaenger: string;
    erstellt_am: string;
    erstellt_von: string;
    dokumente: DocumentVersion[];
  };
  zip_key: string;
  zip_hash: string;
  size: number;
  version: number;
  einreichung: {
    datum: string;
    notiz: string;
    bestaetigt_am: string;
    bestaetigt_von: string;
  } | null;
}
export interface DokumentAuswahl {
  dokument_id: string;
  version: number;
}
export interface NeuesPaket {
  titel: string;
  empfaenger: string;
  dokumente: DokumentAuswahl[];
}
export interface DokumentVergleich {
  kategorie?: import("./generated/plattform").KategorieStand | null;
  eingereichte_kategorie?: import("./generated/plattform").KategorieStand | null;
  aktuell: DocumentVersion;
  zuletzt_eingereicht: DocumentVersion | null;
  paket_id: string | null;
  eingereicht_am: string | null;
  status:
    | "neu"
    | "bereits_eingereicht"
    | "gleicher_inhalt"
    | "geaendert"
    | "aelter";
}
export interface BibliothekQuelle {
  projektId: string;
  vorlage: BibliotheksVorlage;
}
export interface BibliothekMaske {
  schema: Omit<FormularAnalyse, "dokument">;
  pdf_base64: string;
  seiten: PdfSeite[];
}
export const api = {
  kategorienkatalog: () =>
    invoke<import("./generated/plattform").KategorienKatalog>("kategorienkatalog"),
  kategorienkatalogSpeichern: (
    katalog: import("./generated/plattform").KategorienKatalog,
  ) => invoke<import("./generated/plattform").KategorienKatalog>(
    "kategorienkatalog_speichern", { katalog },
  ),
  dokumentkategorien: (projektId: string) =>
    invoke<import("./generated/plattform").DokumentKategorie[]>(
      "dokumentkategorien", { projektId },
    ),
  dokumentkategorieSpeichern: (
    dokumentId: string,
    zuordnung: import("./generated/plattform").KategorieZuordnung,
  ) => invoke<import("./generated/plattform").DokumentKategorie>(
    "dokumentkategorie_speichern", { dokumentId, zuordnung },
  ),
  vorlagenkategorie: (hash: string) =>
    invoke<import("./generated/plattform").KategorieZuordnung>(
      "vorlagenkategorie", { hash },
    ),
  vorlagenkategorieSpeichern: (
    hash: string,
    zuordnung: import("./generated/plattform").KategorieZuordnung,
  ) => invoke<import("./generated/plattform").KategorieZuordnung>(
    "vorlagenkategorie_speichern", { hash, zuordnung },
  ),
  projektBereichSpeichern: (
    basis: Project,
    bereich:
      | "stammdaten"
      | "grundlagen"
      | "festsetzungen"
      | "grundstuecke"
      | "grundstuecksinformationen"
      | "projektdaten",
    aenderung: ProjectUpdate,
  ) =>
    invoke<Project>("projekt_bereich_speichern", { basis, bereich, aenderung }),
  grundstueckskontext: (projektId: string) =>
    invoke<GrundstueckKontext[]>("grundstueckskontext", { projektId }),
  ausgabeauftraege: (projektId: string) =>
    invoke<import("./generated/plattform").Ausgabeauftrag[]>(
      "ausgabeauftraege",
      { projektId },
    ),
  ausgabeWiederholen: (projektId: string, id: string) =>
    invoke<DocumentVersion>("ausgabe_wiederholen", { projektId, id }),
  sicherungsstatus: () =>
    invoke<{ lokal: boolean; profil: string; hinweis: string }>(
      "sicherungsstatus",
    ),
  sicherungErstellen: (ziel: string) =>
    invoke<Sicherungsmanifest>("sicherung_erstellen", { ziel }),
  sicherungPruefen: (pfad: string) =>
    invoke<Sicherungsmanifest>("sicherung_pruefen", { pfad }),
  sicherungWiederherstellen: (pfad: string, ziel: string) =>
    invoke<string>("sicherung_wiederherstellen", { pfad, ziel }),
  profilAktivieren: (pfad: string) =>
    invoke<void>("profil_aktivieren", { pfad }),
  standardprofilAktivieren: () => invoke<void>("standardprofil_aktivieren"),
  projektordnerStatus: () => invoke<string>("projektordner_status"),
  projektordnerWaehlen: (pfad: string) =>
    invoke<void>("projektordner_waehlen", { pfad }),
  baugrundstueckeErmitteln: (projektId: string, plan: BaugrundstuecksPlan) =>
    invoke<BaugrundstueckGruppe[]>("baugrundstuecke_ermitteln", {
      projektId,
      plan,
    }),
  bibliothekFormular: (quelle: BibliothekQuelle, bezug?: FormularBezug) =>
    invoke<BibliothekMaske>("bibliothek_formular", {
      bezug,
      projektId: quelle.projektId,
      vorlage: quelle.vorlage,
    }),
  bibliothekFormularSpeichern: (
    quelle: BibliothekQuelle,
    werte: Record<string, Feldwert>,
    vorgang?: string,
    datenstand?: FormularDatenstand,
  ) =>
    invoke<DocumentVersion>("bibliothek_formular_speichern", {
      projektId: quelle.projektId,
      vorlage: quelle.vorlage,
      werte,
      vorgang,
      datenstand,
    }),
  bibliothekFelderErstellen: (
    vorlage: BibliotheksVorlage,
    felder: EigenesPdfFeld[],
  ) =>
    invoke<BibliotheksVorlage>("bibliothek_felder_erstellen", {
      vorlage,
      felder,
    }),
  formularImProjektSpeichern: (
    dokumentId: string,
    basis: number,
    werte: Record<string, Feldwert>,
    datenstand?: FormularDatenstand,
  ) =>
    invoke<DocumentVersion>("formular_im_projekt_speichern", {
      datenstand,
      dokumentId,
      basis,
      werte,
    }),
  dokumentExternOeffnen: (dokumentId: string, version: number) =>
    invoke<Arbeitskopie>("dokument_extern_oeffnen", { dokumentId, version }),
  arbeitskopien: (projektId: string) =>
    invoke<Arbeitskopie[]>("arbeitskopien", { projektId }),
  arbeitskopieOeffnen: (id: string) =>
    invoke<void>("arbeitskopie_oeffnen", { id }),
  arbeitskopieUebernehmen: (id: string) =>
    invoke<DocumentVersion>("arbeitskopie_uebernehmen", { id }),
  einreichungen: (projektId: string) =>
    invoke<Einreichung[]>("einreichungen", { projektId }),
  einreichungsVergleich: (projektId: string, empfaenger: string) =>
    invoke<DokumentVergleich[]>("einreichungs_vergleich", {
      projektId,
      empfaenger,
    }),
  paketPruefstand: (projektId: string, dokumente: NeuesPaket["dokumente"]) =>
    invoke<PaketPruefstand>("paket_pruefstand", { projektId, dokumente }),
  einreichungErstellen: (
    projektId: string,
    paket: NeuesPaket,
    vorgang?: string,
  ) =>
    invoke<Einreichung>("einreichung_erstellen", { projektId, paket, vorgang }),
  einreichungBestaetigen: (
    id: string,
    basis: number,
    datum: string,
    notiz: string,
  ) =>
    invoke<Einreichung>("einreichung_bestaetigen", { id, basis, datum, notiz }),
  einreichungExportieren: (id: string, ziel: string) =>
    invoke<void>("einreichung_exportieren", { id, ziel }),
  formularbibliothek: () => invoke<Formularbibliothek>("formularbibliothek"),
  formularordnerWaehlen: (pfad: string) =>
    invoke<void>("formularordner_waehlen", { pfad }),
  bibliothekImportieren: (pfad: string) =>
    invoke<BibliotheksVorlage>("bibliothek_importieren", { pfad }),
  bibliothekVorschau: (vorlage: BibliotheksVorlage) =>
    invoke<BibliothekVorschau>("bibliothek_vorschau", { vorlage }),
  bibliothekUebernehmen: (vorlage: BibliotheksVorlage, projektId: string) =>
    invoke<DocumentVersion>("bibliothek_uebernehmen", { vorlage, projektId }),
  bibliothekVeroeffentlichen: (dokumentId: string, version: number) =>
    invoke<BibliotheksVorlage>("bibliothek_veroeffentlichen", {
      dokumentId,
      version,
    }),
  formularLiveVorschau: (pdfBase64: string, werte: Record<string,Feldwert>) =>
    invoke<string>("formular_livevorschau", {pdfBase64,werte}),
  pdfVorschau: (dokumentId: string, version: number | null = null) =>
    invoke<PdfVorschauDaten>("pdf_vorschau", { dokumentId, version }),
  formularVorlageErstellen: (
    dokumentId: string,
    basis: number,
    felder: EigenesPdfFeld[],
  ) =>
    invoke<DocumentVersion>("formular_vorlage_erstellen", {
      dokumentId,
      basis,
      felder,
    }),
  ablageStatus: () => invoke<AblageStatus>("ablage_status"),
  ablageWechseln: (pfad: string) => invoke<number>("ablage_wechseln", { pfad }),
  projektordnerExportieren: (
    projektId: string,
    ziel: string,
    historie = false,
  ) =>
    invoke<ProjektordnerExport>("projektordner_exportieren", {
      projektId,
      ziel,
      historie,
    }),
  appInfo: () => invoke<AppInfo>("app_info"),
  projekte: () => invoke<Project[]>("projekte_auflisten"),
  projektAnlegen: (projekt: Projektwerte) =>
    invoke<Project>("projekt_anlegen", { projekt }),
  projektOeffnen: (id: string) => invoke<Project>("projekt_laden", { id }),
  projektAktivieren: (id: string) => invoke<Project>("projekt_oeffnen", { id }),
  projektSchliessen: () => invoke<void>("projekt_schliessen"),
  projektAktualisieren: (
    aenderung: Projektwerte & {
      id: string;
      expected_version: number;
      planungsrecht?: Planungsrecht;
    },
  ) => invoke<Project>("projekt_aktualisieren", { aenderung }),
  zuletzt: () => invoke<ZuletztEintrag[]>("zuletzt_geoeffnet"),
  dokumente: (projektId: string) =>
    invoke<DocumentVersion[]>("dokumente_auflisten", { projektId }),
  dokumentImportieren: (
    projektId: string,
    pfad: string,
    dokumentId: string | null = null,
    basis: number | null = null,
    kategorie: string | null = null,
  ) =>
    invoke<DocumentVersion>("dokument_importieren", {
      projektId,
      dokumentId,
      basis,
      kategorie,
      pfad,
    }),
  historie: (dokumentId: string) =>
    invoke<DocumentVersion[]>("dokument_historie", { dokumentId }),
  versionWiederherstellen: (dokumentId: string, version: number) =>
    invoke<DocumentVersion>("version_wiederherstellen", {
      dokumentId,
      version,
    }),
  sperren: (dokumentId: string) =>
    invoke<DocumentLock>("dokument_sperren", { dokumentId }),
  entsperren: (dokumentId: string) =>
    invoke<void>("dokument_entsperren", { dokumentId }),
  sperrenAuflisten: (projektId: string) =>
    invoke<DocumentLock[]>("sperren_auflisten", { projektId }),
  protokoll: (projektId: string) =>
    invoke<Protokolleintrag[]>("protokoll", { projektId }),
  benutzerAuflisten: () => invoke<Benutzer[]>("benutzer_auflisten"),
  benutzerAnlegen: (neu: {
    anmeldename: string;
    anzeigename: string;
    rolle: Rolle;
  }) => invoke<Benutzer>("benutzer_anlegen", { neu }),
  benutzerAendern: (aenderung: {
    id: string;
    anzeigename: string;
    rolle: Rolle;
    aktiv: boolean;
  }) => invoke<Benutzer>("benutzer_aendern", { aenderung }),
  lizenz: () => invoke<Lizenzstatus>("lizenz_status"),
  lizenzEinspielen: (pfad: string) =>
    invoke<Lizenzstatus>("lizenz_einspielen", { pfad }),
  arbeitsplatzBelegen: () => invoke<Lizenzstatus>("arbeitsplatz_belegen"),
  lizenzAktivieren: (schluessel: string) =>
    invoke<Lizenzstatus>("lizenz_aktivieren", { schluessel }),
  lizenzBestaetigen: () => invoke<Lizenzstatus>("lizenz_bestaetigen"),
  lizenzAbmelden: () => invoke<Lizenzstatus>("lizenz_abmelden"),
  kunden: () => invoke<Kunde[]>("kunden_auflisten"),
  kundeAnlegen: (daten: Kundendaten) =>
    invoke<Kunde>("kunde_anlegen", { daten }),
  kundeAendern: (kundeId: string, daten: Kundendaten, version: number) =>
    invoke<Kunde>("kunde_aendern", { kundeId, daten, version }),
  kundeLoeschen: (kundeId: string, version: number) =>
    invoke<void>("kunde_loeschen", { kundeId, version }),
  kundeProjekte: (kundeId: string) =>
    invoke<Project[]>("kunde_projekte", { kundeId }),
  bauherren: (projektId: string) => invoke<Kunde[]>("bauherren", { projektId }),
  bauherrenSetzen: (projektId: string, kunden: string[], version: number) =>
    invoke<Project>("bauherren_setzen", { projektId, kunden, version }),
  bauantrag: (projektId: string) =>
    invoke<GespeicherterBauantrag>("bauantrag", { projektId }),
  bauantragSpeichern: (projektId: string, antrag: Bauantrag, version: number) =>
    invoke<GespeicherterBauantrag>("bauantrag_speichern", {
      projektId,
      antrag,
      version,
    }),
  buero: () => invoke<Buero>("buero"),
  bueroSpeichern: (buero: Buero) => invoke<Buero>("buero_speichern", { buero }),
  xbauCodelisten: () => invoke<Codeliste[]>("xbau_codelisten"),
  xbauExportieren: (projektId: string, ziel: string) =>
    invoke<{ dateiname: string; groesse_bytes: number }>("xbau_exportieren", {
      projektId,
      ziel,
    }),
  formularAnalysieren: (dokumentId: string, bezug?: FormularBezug) =>
    invoke<FormularAnalyse>("formular_analysieren", { dokumentId, bezug }),
  formularAusfuellen: (
    dokumentId: string,
    werte: Record<string, Feldwert>,
    basis: number | null = null,
  ) =>
    invoke<DocumentVersion>("formular_ausfuellen", {
      dokumentId,
      werte,
      basis,
    }),
  dokumentExportieren: (
    dokumentId: string,
    ziel: string,
    version: number | null = null,
  ) => invoke<void>("dokument_exportieren", { dokumentId, version, ziel }),
  cadPlan: (dokumentId: string) => invoke<CadPlan>("cad_plan", { dokumentId }),
  projektNutzungPruefen: (projektId: string, eingabe: NutzungsEingabe) =>
    invoke<NutzungsPruefung>("projekt_nutzung_pruefen", { projektId, eingabe }),
  projektBaugrundstueckePruefen: (
    projektId: string,
    eingabe: BaugrundstuecksPlan,
  ) =>
    invoke<BaugrundstuecksPruefung>("projekt_baugrundstuecke_pruefen", {
      projektId,
      eingabe,
    }),
  baugrenzenPruefen: (geometrie: BaugrenzenGeometrie) =>
    invoke<BaugrenzenPruefung>("baugrenzen_pruefen", { geometrie }),
  aufmass: (punkte: Punkt[], geschlossen: boolean, einheit: Laengeneinheit) =>
    invoke<Messung>("aufmass", { punkte, geschlossen, einheit }),
  massDerNutzung: (eingabe: MassDerNutzung, gebiet: Baugebiet) =>
    invoke<Pruefergebnis>("mass_der_nutzung", {
      eingabe,
      gebiet,
      festsetzungen: null,
    }),
  abstandsflaeche: (
    land: Bundesland,
    eingabe: { wandhoehe_m: number; gebiet: Baugebiet },
  ) =>
    invoke<Berechnung<AbstandsflaecheErgebnis>>("abstandsflaeche", {
      land,
      eingabe,
    }),
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
  if (e && typeof e === "object" && "meldung" in e)
    return (e as CmdError).meldung;
  return String(e);
}

export interface GrundstueckKontext {
  id: string;
  projekt_id: string;
  projekt_version: number;
  schluessel: string;
  bezeichnung: string;
  flurstueck_ids: string[];
  eigentumsflaeche_m2: number;
  abzug_m2: number;
  zuschlag_m2: number;
  bezugsflaeche_m2: number | null;
  quelle: string;
}

export type ProjectUpdate = Projektwerte & {
  id: string;
  expected_version: number;
  planungsrecht?: Planungsrecht;
};
