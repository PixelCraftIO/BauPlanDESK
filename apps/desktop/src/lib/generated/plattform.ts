// Automatisch erzeugt: cargo run -p core-lib --example vertraege. Nicht von Hand ändern.
import type { BaugrundstuecksPlan, BaugrundstueckBerechnung, FlaechenAbzug, Planungsrecht } from '../api';
export type Grundstuecksinformationen = { flurstuecke: Array<FlurstuecksInfo>, };
export type FlurstuecksInfo = { id: string, gemarkung: string, flur: string, nummer: string, flaeche_m2: number, grundbuchbezirk: string, grundbuchblatt: string, laufende_nummer: number | null, grundbuchauszug: Dokumentbezug | null, eigentuemer: string, bemerkung: string, };
export type FlurstuecksAnsatz = { flurstueck_id: string, abzuege: Array<FlaechenAbzug>, abweichende_zuordnung: { kennung: string; grundlage: string } | null, };
export type BaugrundstuecksErmittlung = { ansaetze: Array<FlurstuecksAnsatz>, berechnungen: Array<BaugrundstueckBerechnung>,
/**
 * Bei Speicherung durch den Core gesetzt; keine zweite Kopie der Katasterflächen.
 */
quellen_hash: string | null, };
export type Baugebiet = "WS" | "WR" | "WA" | "WB" | "MD" | "MDW" | "MI" | "MU" | "MK" | "GE" | "GI" | "SO";
export type BauNvo = "1962" | "1968" | "1977" | "1990" | "2017";
export type Bauteiltyp = "hauptanlage" | "terrasse_hauptanlage" | "garage" | "stellplatz" | "zufahrt" | "nebenanlage14" | "unterirdisch" | "balkon_loggia" | "terrasse_nebenanlage";
export type Anrechnung = "grz" | "grz_ii" | "keine";
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
export type AusgabeStatus = "laeuft" | "fertig" | "fehler";
export type Ausgabeauftrag = { id: string, dokument_id: DocumentId, dokument_version: number, ziel_root: string, relativer_pfad: string, sha256: string, status: AusgabeStatus, versuche: number, beansprucht_bis: string, versuch_id: string, naechster_versuch: string, fehler: string | null, aktualisiert_am: string, };
export type Dokumentkategorie = { id: string, name: string, eltern_id: string | null, aktiv: boolean, };
export type KategorienKatalog = { revision: number, kategorien: Array<Dokumentkategorie>, };
export type KategorieStand = { id: string, name: string, hauptkategorie: string | null, };
export type KategorieZuordnung = { revision: number, kategorie: string | null, };
export type DokumentKategorie = { dokument_id: DocumentId, revision: number, kategorie: string | null, };
export type MailAdresse = { name: string, adresse: string, };
export type MailAnhang = { name: string, medientyp: string | null, groesse_bytes: number | null, eingebettete_nachricht: boolean, };
export type EmailInhalt = { format: string, betreff: string, absender: Array<MailAdresse>, empfaenger: Array<MailAdresse>, cc: Array<MailAdresse>, bcc: Array<MailAdresse>,
/**
 * RFC3339, soweit ein Datum in der Originaldatei vorhanden und lesbar ist.
 */
datum: string | null, nachrichten_id: string | null, text: string, anhaenge: Array<MailAnhang>, hinweise: Array<string>, };
export type Verfahrensart = "bauantrag" | "vorbescheid" | "aenderungsantrag" | "teilbaugenehmigung" | "freistellung" | "sonstiges";
export type Verfahrensstatus = "vorbereitung" | "in_pruefung" | "ruhend" | "entschieden" | "zurueckgenommen";
export type Behoerde = { id: string, name: string, kennung: string, kontakt: string, };
export type Bezug = { "art": "dokument", dokument: Dokumentbezug, } | { "art": "nachweis", id: string, } | { "art": "kommunikation", id: string, } | { "art": "nachforderung", id: string, punkt: string, } | { "art": "anforderung", id: string, } | { "art": "abgabe", id: string, } | { "art": "bescheid", id: string, } | { "art": "auflage", bescheid: string, id: string, };
export type Kommunikationsart = "email" | "brief" | "portal" | "telefon" | "besprechung";
export type Kommunikation = { id: string, art: Kommunikationsart, eingehend: boolean, datum: string, betreff: string, beteiligte: string, inhalt: string, original: Dokumentbezug | null, bezuege: Array<Bezug>, };
export type Punktstatus = "offen" | "in_arbeit" | "vorbereitet" | "eingereicht" | "bestaetigt" | "entfallen";
export type Frist = { datum: string, ausloeser: string, beleg: Dokumentbezug | null, erinnerung_tage: number, grund: string, };
export type Forderungspunkt = { id: string, titel: string, grundlage: string, verantwortlich: string | null, frist: Frist | null, status: Punktstatus, antwort: string, dokumente: Array<Dokumentbezug>, abgaben: Array<string>, bestaetigung: string | null, };
export type Nachforderung = { id: string, kommunikation: string, punkte: Array<Forderungspunkt>, };
export type Entscheidung = "genehmigt" | "teilweise_genehmigt" | "abgelehnt" | "sonstiges";
export type Auflage = { id: string, titel: string, ausloeser: string, verantwortlich: string | null, frist: Frist | null, erledigt: boolean, nachweise: Array<Dokumentbezug>, };
export type Bescheid = { id: string, entscheidung: Entscheidung, datum: string, zugestellt: string | null, original: Dokumentbezug, genehmigte_unterlagen: Array<Dokumentbezug>, auflagen: Array<Auflage>, ersetzt: string | null, };
export type Erforderlichkeit = "ungeklaert" | "erforderlich" | "nicht_erforderlich";
export type Anforderung = { id: string, titel: string, erforderlich: Erforderlichkeit, grund: string, quelle: string, dokumente: Array<Dokumentbezug>, freigegeben: boolean, verantwortlich: string | null, };
export type Dokumentbeziehung = { id: string, vorher: Dokumentbezug, nachher: Dokumentbezug, grund: string, bezug: Bezug | null, };
export type Aktenereignis = { id: string, datum: string, erfasst_am: string, erfasst_von: string, text: string, beleg: Dokumentbezug | null, berichtigt: string | null, };
export type Erinnerung = { verfahren: string, objekt: string, titel: string, datum: string, ueberfaellig: boolean, verantwortlich: string | null, };
export type Verfahrensakte = { bezeichnung: string, art: Verfahrensart, status: Verfahrensstatus, behoerde: Behoerde, aktenzeichen: string, verantwortlich: string | null, vorgaenger: string | null, profil: Profilstand | null, kommunikation: Array<Kommunikation>, nachforderungen: Array<Nachforderung>, anforderungen: Array<Anforderung>, dokumentbeziehungen: Array<Dokumentbeziehung>, abgaben: Array<Abgabestand>, bescheide: Array<Bescheid>, ereignisse: Array<Aktenereignis>, };
export type Vorhabenprofil = { bundesland: Bundesland, art: Verfahrensart, gebaeudeklasse: number | null, sonderbau: boolean | null, nutzung: string, };
export type Profilregel = { id: string, titel: string, quelle: string, sonderbau: boolean | null, ab_gebaeudeklasse: number | null, nutzung: string | null, };
export type Anforderungskatalog = { id: string, version: string, titel: string, hinweis: string, bundesland: Bundesland | null, art: Verfahrensart | null, regeln: Array<Profilregel>, };
export type Profilstand = { vorhaben: Vorhabenprofil, katalog: Anforderungskatalog, bestaetigt_von: string | null, bestaetigt_am: string | null, };
export type Versandstatus = "versendet" | "eingegangen" | "zurueckgewiesen";
export type Versandereignis = { id: string, status: Versandstatus, datum: string, weg: string, referenz: string, notiz: string, beleg: Dokumentbezug | null, berichtigt: string | null, };
export type Abgabedokument = { dokument: Dokumentbezug, name: string, sha256: string, bytes: number, };
export type Artefakt = { format: string, dateiname: string, storage_key: string, sha256: string, bytes: number, };
export type Abgabestand = { id: string, auftrag_hash: string, titel: string, erstellt_am: string, erstellt_von: string, grundlagen: JsonValue, dokumente: Array<Abgabedokument>, anforderungen: Array<Anforderung>, hinweise: Array<string>, artefakte: Array<Artefakt>, versand: Array<Versandereignis>,
/**
 * Vom Core aus Originalereignissen und Berichtigungen abgeleitet.
 */
versandstatus: Versandstatus | null, };
export type Abgabewunsch = { id: string, titel: string, dokumente: Array<Dokumentbezug>, xbau: boolean, druck: boolean, anschreiben: string, };
export type Textquelle = "pdf" | "email" | "ocr" | "manuell";
export type Suchtext = { dokument: Dokumentbezug, sha256: string, quelle: Textquelle, text: string, indexiert_am: string, };
export type Suchtreffer = { titel: string, bereich: string, auszug: string, verfahren: string | null, dokument: Dokumentbezug | null, };
export type ObjektArt = "verfahren" | "projektorganisation" | "gebaeude" | "aufgaben" | "unterlagen" | "dokumentdaten" | "raumbuch";
export type Projektphase = "vorbereitung" | "vorplanung" | "entwurf" | "genehmigung" | "ausfuehrung" | "abgeschlossen";
export type Projektorganisation = { kennzeichen: string, phase: Projektphase, verantwortlich: BenutzerId | null, beschreibung: string, };
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
export type Druckquelle = { "art": "dokument", dokument_id: DocumentId, version: number, seiten: Array<number>, } | { "art": "nachweis", id: string, } | { "art": "baugrundstueck" } | { "art": "baugrenzen" } | { "art": "raumbuch", id: string, revision: number, };
export type Druckauftrag = { titel: string, absender: string, empfaenger: string, anschreiben: string, mit_anschreiben: boolean, inhaltsverzeichnis: boolean,
/**
 * Schützt aktuelle Fachberichte vor Änderungen während der Zusammenstellung.
 */
projekt_version: number, quellen: Array<Druckquelle>, };
export type Druckeintrag = { titel: string, quelle: Druckquelle, hinweis: string | null, verfuegbar: boolean, };
export type Druckkatalog = { projekt_version: number, absender: string, eintraege: Array<Druckeintrag>, };
export type Druckposition = { titel: string, erste_seite: number, seiten: number, };
export type NachweisAnsicht = { stand: Nachweisstand, aktuell: boolean, freigabe: Freigabe | null, };
