import { leseAbfrage } from "./leseAbfrage";
import type { BauNvo, Bauteiltyp, Anrechnung } from "./generated/plattform";
export type { BauNvo, Bauteiltyp, Anrechnung } from "./generated/plattform";
// Typisierte Hülle um die Tauri-Commands. Das Frontend rechnet nichts selbst;
// alle Fachlogik liegt in core-lib (Rust).
import { invoke } from "@tauri-apps/api/core";
import type { ProjektRecht } from "./generated/plattform";
import { melden } from "./projektEreignisse";

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

/** Reihenfolge wie core-lib `zugriff::ProjektRecht`; nur für die Oberfläche. */
const PROJEKTRECHTE: ProjektRecht[] = ["kein", "lesen", "bearbeiten", "leitung"];
export const projektRechtMindestens = (r: ProjektRecht, min: ProjektRecht) =>
  PROJEKTRECHTE.indexOf(r) >= PROJEKTRECHTE.indexOf(min);

/**
 * Benutzer, wie ihn eine Projektakte für Knöpfe und Formulare sieht: Mit nur
 * lesendem Projektrecht gilt die Rolle „Lesen“ (wie ohne Schreiblizenz).
 * Maßgebend bleibt die Prüfung in core-lib.
 */
export function imProjekt(b: Benutzer, recht: ProjektRecht): Benutzer {
  return projektRechtMindestens(recht, "bearbeiten") ? b : { ...b, rolle: "lesen" };
}

export interface Benutzer {
  id: string;
  anmeldename: string;
  anzeigename: string;
  rolle: Rolle;
  aktiv: boolean;
  erstellt_am: string;
  /** Fehlt bei älteren Antworten (= keines). */
  kennwort?: Kennwortstatus;
  anmeldesperre_bis?: string | null;
  letzte_anmeldung?: string | null;
}

export type Kennwortstatus = "keines" | "erstkennwort" | "gesetzt";

export const KENNWORTSTATUS: Record<Kennwortstatus, string> = {
  keines: "kein Kennwort",
  erstkennwort: "Erstkennwort (muss geändert werden)",
  gesetzt: "eigenes Kennwort",
};

/** Optionen des Anmeldedialogs; die Regeln stehen in core-lib (anmeldung.rs). */
export interface Anmeldeoptionen {
  uebergang_moeglich: boolean;
  rechnerkonto: string;
  erster_administrator: boolean;
}

export interface Anmeldestatus {
  /** Netzwerkbetrieb: Anmeldung mit Anmeldename und Kennwort. */
  kennwortanmeldung: boolean;
  benutzer: Benutzer | null;
  /** Angemeldet, aber zuerst ein eigenes Kennwort setzen. */
  kennwort_aendern: Benutzer | null;
  bisheriges_kennwort_noetig: boolean;
  optionen: Anmeldeoptionen | null;
  /** Hinweis für Eingabefelder; geprüft wird in core-lib. */
  kennwort_mindestlaenge: number;
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
  projekt_geloescht: "Projekt gelöscht",
  dokument_abgelegt: "Dokument abgelegt",
  dokument_gesperrt: "Dokument gesperrt",
  dokument_entsperrt: "Sperre aufgehoben",
  sperre_aufgehoben: "Fremde Sperre aufgehoben",
  formular_ausgefuellt: "Formular ausgefüllt",
  benutzer_angelegt: "Benutzer angelegt",
  benutzer_geaendert: "Benutzer geändert",
  angemeldet: "Angemeldet",
  anmeldung_fehlgeschlagen: "Anmeldung fehlgeschlagen",
  kennwort_geaendert: "Kennwort geändert",
  lizenz_eingespielt: "Lizenz eingespielt",
  bauantrag_geaendert: "Bauantrag geändert",
  einreichung_erstellt: "Einreichungspaket erstellt",
  einreichung_bestaetigt: "Einreichung bestätigt",
  xbau_exportiert: "XBau-Bauantrag exportiert",
  kunde_angelegt: "Kunde angelegt",
  kunde_geaendert: "Kunde geändert",
  kunde_geloescht: "Kunde gelöscht",
  bauherren_geaendert: "Bauherren geändert",
  beteiligte_geaendert: "Beteiligte geändert",
  verfahren_angelegt: "Verfahren angelegt",
  verfahren_geaendert: "Verfahrensakte geändert",
  verfahrensstatus_geaendert: "Verfahrensstand geändert",
  verfahren_abgabe: "Abgabe / Übermittlung erfasst",
  verfahren_bescheid: "Bescheid erfasst / geändert",
  verfahren_nachforderung: "Nachforderung bearbeitet",
  verfahren_schritt: "Behördlichen Schritt bearbeitet",
  verfahren_ausfuehrung: "Bauausführung erfasst",
  verfahren_berichtigt: "Verfahrensangabe berichtigt",
};

/** Ergebnis von `projekt_loeschen` (core_lib::service::Projektloeschung). */
export interface Projektloeschung {
  dateien_geloescht: number;
  /** Ablagedateien, die nicht entfernt werden konnten. */
  dateien_verblieben: string[];
}

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

/** Erreichbarkeit eines Beteiligten; fehlt in älteren Daten. */
export interface Kommunikation {
  telefon: string;
  email: string;
}

export type Beteiligter =
  | {
      art: "person";
      id: string;
      vorname: string;
      familienname: string;
      anschrift: Anschrift;
      kommunikation?: Kommunikation;
    }
  | {
      art: "organisation";
      id: string;
      name: string;
      anschrift: Anschrift;
      /** Ansprechpartner als Freitext. */
      ansprechpartner?: string;
      kommunikation?: Kommunikation;
    };

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
  /** Gewählte Person aus `Buero.entwurfsverfasser`; null = die einzige des Büros. */
  entwurfsverfasser?: string | null;
  massnahmen: string[];
  anzahl_gebaeude: number;
  grundstueck: { anschrift: Anschrift; flurstuecke: Flurstueck[] };
  anlagen: Anlagenwahl[];
  /** Code der Landes-Codeliste der Verfahrensarten; leer/fehlend = keine Angabe. */
  verfahrensart?: string;
}

export interface GespeicherterBauantrag {
  antrag: Bauantrag;
  version: number;
  geaendert_von: string | null;
  geaendert_am: string | null;
  fehlende_angaben: string[];
  /** Prüfhinweise, die den Export nicht sperren (z. B. Zustimmungserklärung). */
  hinweise?: string[];
  /** Flurstücke stammen aus den Grundstücksinformationen und sind hier nur Anzeige. */
  flurstuecke_zentral?: boolean;
  /** Gebäudeanzahl stammt aus Gebäude/Geschosse und ist hier nur Anzeige. */
  gebaeude_zentral?: boolean;
  /** Geladene Landes-Codeliste der Verfahrensarten für das Bundesland des Projekts. */
  verfahrensarten?: Codeliste;
  /** Anlagenart je Dokument-ID aus der Dokumentkategorie (gilt ohne eigene Wahl). */
  anlagenart_vorschlaege?: Record<string, { anlageart: string; kategorie: string }>;
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

/** Rolle eines Kunden im Projekt neben dem Bauherrn (core-lib kunden::Beteiligtenrolle). */
export type Beteiligtenrolle =
  | "grundstueckseigentuemer"
  | "erbbauberechtigter"
  | "bevollmaechtigter"
  | "fachplaner";
export const BETEILIGTENROLLEN: { code: Beteiligtenrolle; name: string }[] = [
  { code: "grundstueckseigentuemer", name: "Grundstückseigentümer" },
  { code: "erbbauberechtigter", name: "Erbbauberechtigter" },
  { code: "bevollmaechtigter", name: "Bevollmächtigter" },
  { code: "fachplaner", name: "Fachplaner" },
];
export function beteiligtenrolleName(r: Beteiligtenrolle): string {
  return BETEILIGTENROLLEN.find((x) => x.code === r)?.name ?? r;
}
export interface Projektbeteiligter {
  rolle: Beteiligtenrolle;
  kunde: Kunde;
  /** Fachrichtung im Projekt (XBau `fachrichtung`); leer = nicht angegeben. */
  fachrichtung?: string;
}

/** Fachrichtungen der XBau-Schemaliste `beteiligterfachrichtung` 3.0 (siehe
 *  core-lib xbau::FACHRICHTUNGEN); andere Angaben gehen als Freitext. */
export const FACHRICHTUNGEN: { code: string; name: string }[] = [
  { code: "1", name: "Tragwerksplanung/Standsicherheit" },
  { code: "2", name: "Schallschutz" },
  { code: "3", name: "Brandschutz" },
  { code: "4", name: "Lüftung" },
  { code: "5", name: "Sicherheit" },
  { code: "6", name: "Wärmeschutz" },
];

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
        kommunikation: { telefon: k.telefon, email: k.email },
      }
    : {
        art: "organisation",
        id: k.id,
        name: k.firma,
        anschrift: { ...k.anschrift },
        ansprechpartner: k.ansprechpartner,
        kommunikation: { telefon: k.telefon, email: k.email },
      };
}

export interface Buero {
  name: string;
  anschrift: Anschrift;
  kennung: string;
  /** Zentrale Rufnummer und E-Mail des Büros; fehlen bei älteren Servern. */
  telefon: string;
  email: string;
  /** Bauvorlageberechtigte Personen des Büros. */
  entwurfsverfasser: Entwurfsverfasser[];
}

export interface Entwurfsverfasser {
  id: string;
  vorname: string;
  familienname: string;
  berufsbezeichnung: string;
  /** Mitgliedsnummer der Architekten- oder Ingenieurkammer. */
  mitgliedsnummer: string;
  kammerland: Bundesland | null;
  /** Nachweis der Bauvorlageberechtigung ohne Kammermitgliedschaft. */
  nachweis: string;
  /** Eigene Durchwahl, Mobil und E-Mail; leer = Angaben des Büros. */
  telefon: string;
  mobil: string;
  email: string;
}

export function entwurfsverfasserName(e: Entwurfsverfasser): string {
  const name = `${e.vorname} ${e.familienname}`.trim() || "Ohne Namen";
  return e.berufsbezeichnung.trim() ? `${name}, ${e.berufsbezeichnung.trim()}` : name;
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

/** Datum "JJJJ-MM-TT" (oder Zeitpunkt) deutsch anzeigen: „05.11.2026“. */
export const datumDe = (d: string | null | undefined) => {
  if (!d) return "";
  const t = /^\d{4}-\d{2}-\d{2}$/.test(d) ? new Date(`${d}T12:00:00`) : new Date(d);
  return Number.isNaN(t.getTime())
    ? d
    : t.toLocaleDateString("de-DE", { day: "2-digit", month: "2-digit", year: "numeric" });
};

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
  /** Zeilenkennung für Planbindungen (`bauteile.<id>.flaeche_m2`); bei Übernahme aus dem Aufmaß die Markierungs-ID. */
  id?: string;
  /** Grundfläche aus einer Markierung des Aufmaßes; setzt der Core, schließt `gebaeude_id` aus. */
  planbindung?: import("./generated/plattform").Planbindung;
  gebaeude_id?: string;
  bezeichnung: string;
  typ: Bauteiltyp;
  flaeche_m2: number;
  abweichende_anrechnung: Anrechnung | null;
  begruendung: string;
}
export interface Geschoss {
  flaeche_aus_grundlagen?: boolean;
  teilflaechen_geprueft?: boolean;
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
  /** Wirksamer Anrechnungsfaktor für Nichtvollgeschosse (core-lib); ältere Ergebnisse ohne Angabe. */
  nichtvollgeschoss_faktor?: number | null;
}
export interface FlaechenAbzug {
  /** Zeilenkennung für Planbindungen (`abzuege.<id>.flaeche_m2`); bei Übernahme aus dem Aufmaß die Markierungs-ID. */
  id?: string;
  art: string;
  flaeche_m2: number;
  grundlage: string;
  /** Fläche aus einer Markierung des Aufmaßes (Bedeutung passend zur Art, dem Flurstück zugeordnet); setzt der Core. */
  planbindung?: import("./generated/plattform").Planbindung;
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
  /** Flurstücke der Einheit beim letzten Speichern (für Umgruppierungen). */
  flurstueck_ids?: string[];
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
  /** Eindeutig beerbte, nicht mehr bestehende Berechnungen (Schlüssel). */
  vorgaenger?: string[];
}
/** Gebäude/Geschosse, die (noch) nicht in GRZ/GFZ eines Baugrundstücks eingehen. */
export interface Einbindungshinweis {
  schluessel: string;
  text: string;
}
export interface GebaeudeEinbindung {
  plan: BaugrundstuecksPlan;
  hinweise: Einbindungshinweis[];
}
export interface BaugrundstuecksPruefung {
  projekt_version: number;
  baugrundstuecke: {
    grundstueck: BaugrundstueckGruppe;
    pruefung: NutzungsPruefung;
    /** Gezeichnete und von Hand erfasste Zeilen derselben Fläche (Texte auch in `pruefung.hinweise`). */
    doppelerfassungen?: import("./generated/plattform").Doppelerfassung[];
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
  kennzahlen: Kennzahlen;
  bebauungsplan: boolean;
  /** Fachliche Hinweise des Core, z. B. fehlende BauNVO-Fassung des Plans. */
  hinweise: string[];
  nachweise: {
    art: string;
    gegenstand?: string | null;
    ist: number | null;
    soll: number | null;
    quelle: "festsetzung" | "orientierungswert" | "bau_nvo19" | "umgebung" | "keine";
    /** baufensterflaeche: nur Information; baufenster_lage: Urteil aus den Baugrenzen. */
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
  /** Schlüssel des Baugrundstücks, zu dem der Vorhabenumriss gehört. */
  grundstueck_schluessel?: string | null;
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

export interface Umgebungsreferenz {
  grz: number | null;
  gfz: number | null;
  vollgeschosse: number | null;
  grundlage: string;
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
  /** Vergleichswerte der näheren Umgebung (§ 34 BauGB); Orientierung ohne Urteil. */
  umgebung?: Umgebungsreferenz | null;
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

export interface Kennzahlen {
  grz: number;
  grz_gesamt: number;
  gfz: number;
  bmz: number | null;
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
  verfahren_id?: string | null;
  /** Person aus den Büroangaben; null = laut Bauantrag. */
  entwurfsverfasser_id?: string | null;
  /** Je Feld eine abweichend gewählte Projektdatenquelle (Kontextschlüssel). */
  feldquellen?: Record<string, string>;
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
  verfahren: { id: string; bezeichnung: string }[];
  /** Bauvorlageberechtigte Personen des Büros; fehlt bei älteren Servern. */
  entwurfsverfasser?: { id: string; bezeichnung: string }[];
  hinweise: string[];
}
/** Ausgefülltes Feld, dessen Wert von den aktuellen Projektdaten abweicht (Core-Abgleich). */
export interface VeralteterFeldwert {
  feld: string;
  quelle: string;
  wert: Feldwert;
  aktuell: Feldwert;
}
export interface FormularAnalyse {
  quellen?: FormularQuellen;
  dokument: DocumentVersion;
  felder: Formularfeld[];
  vorlage: { id: string; titel: string; version: string } | null;
  vorschlag: Record<string, Feldwert>;
  /** Fehlt bei älteren Servern. */
  veraltet?: VeralteterFeldwert[];
  /** Wirksame Projektdatenquelle je Feld; fehlt bei älteren Servern. */
  feldquellen?: Record<string, string>;
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

/** Unterlagen, die das Aufmaß öffnen kann (PDF-Blätter und DXF-Zeichnungen). */
export const istPlan = (dateiname: string) => /\.(pdf|dxf)$/i.test(dateiname);
/** IFC-Modell (STEP oder gepackt); IFC-XML wird nicht gelesen. */
export const istIfc = (dateiname: string) => /\.(ifc|ifczip)$/i.test(dateiname);
export const istDxf = (dateiname: string) => /\.dxf$/i.test(dateiname);

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
export interface ArchivierteEmail {
  dokument: DocumentVersion;
  inhalt: import("./generated/plattform").EmailInhalt;
}
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
export interface DokumentVergleich {
  kategorie?: import("./generated/plattform").KategorieStand | null;
  eingereichte_kategorie_erfasst?: boolean;
  eingereichte_kategorie?: import("./generated/plattform").KategorieStand | null;
  aktuell: DocumentVersion;
  zuletzt_eingereicht: DocumentVersion | null;
  paket_id: string | null;
  /** Verfahrensakte, wenn der Stand aus einem versendeten Abgabestand oder einer externen Einreichung stammt. */
  verfahren?: string | null;
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
/** Legt eine Dokumentversion ab und meldet sie der zugehörigen Projektakte. */
const ablage = (cmd: string, args: Record<string, unknown>) =>
  invoke<DocumentVersion>(cmd, args).then((v) => {
    melden({ projektId: v.project_id, bereich: "dokumente" });
    return v;
  });

export const api = {
  dokumentEntfernen: (dokumentId: string, basis: number) => invoke<void>("dokument_entfernen", { dokumentId, basis }),
  dokumentImExplorer: (dokumentId: string, version: number) => invoke<void>("dokument_im_explorer", { dokumentId, version }),
  emailLaden: (dokumentId: string, version: number) => invoke<ArchivierteEmail>("email_laden", { dokumentId, version }),
  emailAnhangAblegen: (dokumentId: string, version: number, index: number) => ablage("email_anhang_ablegen", { dokumentId, version, index }),
  /** Archivierte E-Mail als Kommunikation in eine Verfahrensakte übernehmen. */
  emailInVerfahren: (
    projektId: string,
    dokumentId: string,
    version: number,
    verfahren: string,
    basis: number,
    eingehend: boolean,
    /** Bestätigter Zugang bzw. Versand; ohne Angabe das Sendedatum. */
    datum: string | null = null,
  ) =>
    invoke<import("./generated/plattform").Fachstand>("email_in_verfahren", {
      projektId,
      dokumentId,
      version,
      verfahren,
      basis,
      eingehend,
      datum,
    }),
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
    ablage("ausgabe_wiederholen", { projektId, id }),
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
  beispielprojektStatus: () =>
    invoke<{ aktiv: boolean; kopie_vorhanden: boolean }>(
      "beispielprojekt_status",
    ),
  /** Startet die Anwendung im Profil des Beispielprojekts neu. */
  beispielprojektOeffnen: (neu: boolean) =>
    invoke<void>("beispielprojekt_oeffnen", { neu }),
  /** Startet die Anwendung im vorherigen Profil neu. */
  beispielprojektVerlassen: () => invoke<void>("beispielprojekt_verlassen"),
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
    ablage("formular_im_projekt_speichern", {
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
    ablage("arbeitskopie_uebernehmen", { id }),
  einreichungen: (projektId: string) =>
    invoke<Einreichung[]>("einreichungen", { projektId }),
  einreichungsVergleich: (projektId: string, empfaenger: string) =>
    invoke<DokumentVergleich[]>("einreichungs_vergleich", {
      projektId,
      empfaenger,
    }),
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
    ablage("bibliothek_uebernehmen", { vorlage, projektId }),
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
    ablage("formular_vorlage_erstellen", {
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
  kalenderaboAuskunft: () =>
    invoke<import("./generated/plattform").Aboauskunft>("kalenderabo_auskunft"),
  kalenderaboAnlegen: (umfang: import("./generated/plattform").Aboumfang) =>
    invoke<import("./generated/plattform").Aboschluessel>(
      "kalenderabo_anlegen",
      { umfang },
    ),
  kalenderaboWiderrufen: () => invoke<void>("kalenderabo_widerrufen"),
  planliste: (projektId: string) =>
    invoke<import("./generated/plattform").Planliste>("planliste", {
      projektId,
    }),
  planlayoutExportieren: (projektId: string, wunsch: import("./generated/plattform").Layoutwunsch, ziel: string | null) =>
    invoke<DocumentVersion | null>("planlayout_exportieren", { projektId, wunsch, ziel }),
  planaufmassDxf: (projektId: string, aufmassId: string, revision: number, nurSichtbareEbenen: boolean, ziel: string) =>
    invoke<void>("planaufmass_dxf", { projektId, aufmassId, revision, nurSichtbareEbenen, ziel }),
  planRevisionswolken: (projektId: string, wunsch: import("./generated/plattform").Wolkenwunsch, ziel: string) =>
    invoke<void>("plan_revisionswolken", { projektId, wunsch, ziel }),
  plankopfStempeln: (projektId: string, wunsch: import("./generated/plattform").Stempelwunsch, ziel: string) =>
    invoke<void>("plankopf_stempeln", { projektId, wunsch, ziel }),
  plannummernschema: () =>
    invoke<import("./generated/plattform").Plannummernschema>(
      "plannummernschema",
    ),
  plannummernschemaSpeichern: (
    schema: import("./generated/plattform").Plannummernschema,
  ) =>
    invoke<import("./generated/plattform").Plannummernschema>(
      "plannummernschema_speichern",
      { schema },
    ),
  bueroCockpit: () =>
    invoke<import("./generated/plattform").Buerocockpit>("buero_cockpit"),
  projektAnlegen: (projekt: Projektwerte) =>
    invoke<Project>("projekt_anlegen", { projekt }),
  projektOeffnen: (id: string) => invoke<Project>("projekt_laden", { id }),
  /** Löscht das Projekt endgültig; Prüfung von Rechten und Sperren im Core. */
  projektLoeschen: (id: string, version: number) =>
    invoke<Projektloeschung>("projekt_loeschen", { id, version }),
  /** Eigenes wirksames Projektrecht nach der Core-Regel (zugriff.rs). */
  projektRecht: (id: string) => invoke<ProjektRecht>("projekt_recht", { id }),
  /** Aufmaß des Projekts in einem eigenen Fenster öffnen oder nach vorn holen. */
  editorFenster: (id: string) => invoke<void>("editor_fenster_oeffnen", { id }),
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
  dokumenteNeueste: (projektId: string) => invoke<DocumentVersion[]>("dokumente_auflisten", { projektId, neueste: true }),
  historienseite: (dokumentId: string, vor: number | null = null) => invoke<DocumentVersion[]>("dokument_historie", { dokumentId, vor, limit: 50 }),
  dokumente: (projektId: string) =>
    leseAbfrage("dokumente",projektId,()=>invoke<DocumentVersion[]>("dokumente_auflisten", { projektId })),
  dokumentImportieren: (
    projektId: string,
    pfad: string,
    dokumentId: string | null = null,
    basis: number | null = null,
    kategorie: string | null = null,
  ) =>
    ablage("dokument_importieren", {
      projektId,
      dokumentId,
      basis,
      kategorie,
      pfad,
    }),
  historie: (dokumentId: string) =>
    invoke<DocumentVersion[]>("dokument_historie", { dokumentId }),
  versionWiederherstellen: (dokumentId: string, version: number) =>
    ablage("version_wiederherstellen", {
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
  anmeldestatus: () => invoke<Anmeldestatus>("anmeldestatus"),
  anmelden: (anmeldename: string, kennwort: string) =>
    invoke<Anmeldestatus>("anmelden", { anmeldename, kennwort }),
  anmeldenUebergang: () => invoke<Anmeldestatus>("anmelden_uebergang"),
  kennwortAendern: (bisher: string | null, neu: string) =>
    invoke<Anmeldestatus>("kennwort_aendern", { bisher, neu }),
  abmelden: () => invoke<Anmeldestatus>("abmelden"),
  erstkennwortVergeben: (benutzerId: string, erstkennwort: string) =>
    invoke<Benutzer>("erstkennwort_vergeben", { benutzerId, erstkennwort }),
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
  beteiligte: (projektId: string) =>
    invoke<Projektbeteiligter[]>("beteiligte", { projektId }),
  beteiligteSetzen: (
    projektId: string,
    beteiligte: { rolle: Beteiligtenrolle; kunde: string; fachrichtung?: string }[],
    version: number,
  ) => invoke<Project>("beteiligte_setzen", { projektId, beteiligte, version }),
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
  xbauCodelisteEinspielen: (pfad: string) =>
    invoke<Codeliste>("xbau_codeliste_einspielen", { pfad }),
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
  /** DXF-Zeichnung einer Dokumentversion (ohne Version: die aktuelle). */
  cadPlan: (dokumentId: string, version: number | null = null) => invoke<CadPlan>("cad_plan", { dokumentId, version }),
  projektNutzungPruefen: (projektId: string, eingabe: NutzungsEingabe) =>
    invoke<NutzungsPruefung>("projekt_nutzung_pruefen", { projektId, eingabe }),
  projektBaugrundstueckePruefen: (
    projektId: string,
    eingabe: BaugrundstuecksPlan,
    schluessel: string | null = null,
  ) =>
    invoke<BaugrundstuecksPruefung>("projekt_baugrundstuecke_pruefen", {
      projektId,
      eingabe,
      schluessel,
    }),
  baugrundstueckeGebaeudeEinbinden: (
    projektId: string,
    eingabe: BaugrundstuecksPlan,
    schluessel: string | null = null,
  ) =>
    invoke<GebaeudeEinbindung>("projekt_baugrundstuecke_gebaeude_einbinden", {
      projektId,
      eingabe,
      schluessel,
    }),
  baugrundstueckEinzelnSpeichern:(projektId:string,version:number,berechnung:BaugrundstueckBerechnung)=>invoke<Project>("baugrundstueck_einzeln_speichern",{projektId,version,berechnung}),
  baugrenzenPruefen: (geometrie: BaugrenzenGeometrie) =>
    invoke<BaugrenzenPruefung>("baugrenzen_pruefen", { geometrie }),
  abstandsflaeche: (
    land: Bundesland,
    eingabe: { wandhoehe_m: number; gebiet: Baugebiet },
  ) =>
    invoke<Berechnung<AbstandsflaecheErgebnis>>("abstandsflaeche", {
      land,
      eingabe,
    }),
  updateStatus: () => invoke<{eingerichtet:boolean;quelle:string;hinweis:string;verfuegbar?:UpdateInfo|null}>("update_status"),
  updatePruefen: () => invoke<UpdateInfo | null>("update_pruefen"),
  updateInstallieren: (version: string) => invoke<void>("update_installieren", {version}),
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
