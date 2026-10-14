// Automatisch erzeugt: cargo run -p core-lib --example vertraege. Nicht von Hand ändern.
import type { BaugrundstuecksPlan, Planungsrecht } from '../api';
export type Baugebiet = "WS" | "WR" | "WA" | "WB" | "MD" | "MDW" | "MI" | "MU" | "MK" | "GE" | "GI" | "SO";
export type AbstandsflaecheInput = {
/**
 * Wandhöhe H in m (Geländeoberfläche bis Schnittpunkt Wand/Dachhaut)
 */
wandhoehe_m: number, gebiet: Baugebiet, };
export type ProjectId = string;
export type DocumentId = string;
export type Bundesland = "BW" | "BY" | "BE" | "BB" | "HB" | "HH" | "HE" | "MV" | "NI" | "NW" | "RP" | "SL" | "SN" | "ST" | "SH" | "TH";
export type BenutzerId = string;
export type JsonValue = number | string | boolean | Array<JsonValue> | { [key in string]?: JsonValue } | null;
export type Fachstand = { projekt_id: ProjectId, modul: string, objekt: string, schema_version: number, revision: number, daten: JsonValue, };
export type ProjektRecht = "kein" | "lesen" | "bearbeiten" | "leitung";
export type ProjektMitglied = { benutzer_id: BenutzerId, recht: ProjektRecht, };
export type ObjektArt = "gebaeude" | "aufgaben" | "unterlagen" | "dokumentdaten" | "raumbuch";
export type Dokumentbezug = { dokument_id: DocumentId, version: number, };
export type Gebaeude = { bezeichnung: string, grundstueck_ids: Array<string>, geschosse: Array<GebaeudeGeschoss>, quelle: Dokumentbezug | null, };
export type GebaeudeGeschoss = { id: string, bezeichnung: string, vollgeschoss: boolean | null, grundlage: string, };
export type AufgabenStatus = "offen" | "in_arbeit" | "erledigt";
export type Aufgabe = { titel: string, beschreibung: string, faellig: string | null, verantwortlich: BenutzerId | null, status: AufgabenStatus, bezug: string | null, };
export type UnterlagenStatus = "entwurf" | "pruefung" | "freigegeben";
export type Unterlage = { vorlage: ChecklistenQuelle | null, bezeichnung: string, erforderlich: boolean, dokument: Dokumentbezug | null, status: UnterlagenStatus, bemerkung: string, };
export type ChecklistenQuelle = { id: string, version: string, punkt: string, };
export type Checklistenpunkt = { id: string, bezeichnung: string, erforderlich: boolean, };
export type Checkliste = { id: string, version: string, titel: string, hinweis: string, unterlagen: Array<Checklistenpunkt>, };
export type Statushinweis = { bereich: string, text: string, };
export type Projektstatus = { ermittelt_am: string, offene_aufgaben: number, ueberfaellige_aufgaben: number, erforderliche_unterlagen: number, fehlende_unterlagen: number, ungepruefte_unterlagen: number, nachweise: number, veraltete_nachweise: number, offene_freigaben: number, exportierte_pakete: number, bestaetigte_einreichungen: number, hinweise: Array<Statushinweis>, };
export type Dokumentdaten = { dokument: Dokumentbezug, dokumentart: string, plannummer: string, revisionsindex: string, massstab: string, verfasser: string, datum: string | null, };
export type Raumbuch = { bezeichnung: string, gebaeude_id: string, geschoss_id: string | null, raeume: Array<Raum>, };
export type Raum = { id: string, nummer: string, bezeichnung: string, nutzung: string, flaeche_m2: number, hoehe_m: number | null, };
export type RaumSummen = { anzahl: number, flaeche_m2: number, volumen_m3: number | null, };
export type NutzungsArt = "grz" | "gfz" | "geschossigkeit" | "bmz";
export type NachweisEingabe = { "art": "nutzung", fach: NutzungsArt, plan: BaugrundstuecksPlan, } | { "art": "abstand", gegenstand: string, eingabe: AbstandsflaecheInput, };
export type RegelArchiv = { id: string, version: string, sha256: string, format: string, inhalt: string, pruefstatus: string, };
export type Nachweisstand = { id: string, bezeichnung: string, erstellt_am: string, erstellt_von: string, land: Bundesland, rechtskontext: Planungsrecht, projekt_version: number, eingabe: NachweisEingabe, grundlagen_hash: string, regeln: Array<RegelArchiv>, gebaeude_hash: string, anfrage_hash: string, rechenstand: string, ergebnis: JsonValue, vollstaendig: boolean, };
export type Freigabe = { nachweis_id: string, grundlagen_hash: string, freigegeben_am: string, freigegeben_von: string, bemerkung: string, };
export type NachweisAnsicht = { stand: Nachweisstand, aktuell: boolean, freigabe: Freigabe | null, };
