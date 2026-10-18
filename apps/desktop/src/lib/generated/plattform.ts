// Automatisch erzeugt: cargo run -p core-lib --example vertraege. Nicht von Hand ändern.
import type { BaugrundstuecksPlan, BaugrundstueckBerechnung, FlaechenAbzug, Planungsrecht } from '../api';
export type Profilfreigabe = "allgemein" | "entwurf" | "fachlich_geprueft";
export type PortalUnterlagenart = { code: string, name: string, erforderlich: boolean, kategorien: Array<string>, };
export type Portalprofil = { id: string, version: string, titel: string, bundesland: Bundesland | null, verfahren: Array<Verfahrensart>, freigabe: Profilfreigabe, quelle: string, stand: string, portal_url: string | null, max_datei_bytes: number | null, max_gesamt_bytes: number | null, erlaubte_endungen: Array<string>, unterlagenarten: Array<PortalUnterlagenart>, hinweise: Array<string>, };
export type Portalziel = { kommune: string, stelle: string, behoerdenkennung: string, url: string, zuordnung_bestaetigt: boolean, };
export type Portalvorgang = { id: string, profil: Portalprofil, ziel: Portalziel, erstellt_am: string, erstellt_von: string, abgaben: Array<Portalabgabe>, rueckmeldungen: Array<Portalrueckmeldung>, };
export type Portalakte = { vorgaenge: Array<Portalvorgang>, };
export type Portalabgabeart = "erstantrag" | "nachreichung";
export type PortalForderungsbezug = { nachforderung: string, punkt: string, };
export type DateiZuordnung = { dokument: Dokumentbezug, unterlagenart: string | null, uploadname: string, ersetzt: Dokumentbezug | null, };
export type Dateivergleich = "neu" | "identisch" | "geaendert" | "aelter" | "anderes_dokument";
export type Portaldatei = { original: Abgabedokument, uploadname: string, unterlagenart: string | null, vergleich: Dateivergleich, ersetzt: Dokumentbezug | null, };
export type Portalpruefbericht = { fehler: Array<string>, hinweise: Array<string>, profil_fachlich_geprueft: boolean, };
export type Portalabgabe = { id: string, abgabe: string, auftrag_hash: string, art: Portalabgabeart, vorgaenger: string | null, forderungen: Array<PortalForderungsbezug>, dateien: Array<Portaldatei>, pruefung: Portalpruefbericht, erstellt_am: string, bereitstellung: Artefakt, ereignisse: Array<Portalereignis>, };
export type Portalereignisart = "abgabe_belegt" | "eingang_belegt" | "zurueckgewiesen" | "ausgang_unklar";
export type Dateiabgleich = "ungeklaert" | "vollstaendig" | "abweichend";
export type Portalereignis = { id: string, art: Portalereignisart, datum: string, referenz: string, notiz: string, beleg: Dokumentbezug | null, dateiabgleich: Dateiabgleich, bestaetigte_dateien: Array<Dokumentbezug>, berichtigt: string | null, erfasst_am: string, erfasst_von: string, };
export type Portalereigniswunsch = { id: string, art: Portalereignisart, datum: string, referenz: string, notiz: string, beleg: Dokumentbezug | null, dateiabgleich: Dateiabgleich, bestaetigte_dateien: Array<Dokumentbezug>, berichtigt: string | null, };
export type Portalrueckmeldung = { id: string, betreff: string, datum: string, referenz: string, original: Dokumentbezug, inhalt: string, abgabe: string | null, xbau_nachricht: string | null, erfasst_am: string, erfasst_von: string, };
export type Portalrueckmeldungswunsch = { id: string, betreff: string, datum: string, referenz: string, original: Dokumentbezug, inhalt: string, abgabe: string | null, xbau_nachricht: string | null, };
export type Portalvorgangswunsch = { id: string, profil_id: string, profil_version: string, ziel: Portalziel, };
export type Portalabgabewunsch = { abgabe: string, art: Portalabgabeart, vorgaenger: string | null, dateien: Array<DateiZuordnung>, forderungen: Array<PortalForderungsbezug>, };
export type Portalvorbereitung = { referenzdateien: Array<Portaldatei>, dateien: Array<Portaldatei>, pruefung: Portalpruefbericht, };
export type Portalbedarf = { vorgang: string, abgabe: string | null, titel: string, hinweis: string, };
export type Nachrichtentyp = { element: string, namensraum: string, titel: string, schema: string, };
export type Nachrichtenkopf = { id: string, element: string, namensraum: string, titel: string, erstellt: string, absender: string, empfaenger: string, antragsreferenz: string | null, aktenzeichen: string | null, bezugsnachricht: string | null, };
export type AnlagePruefung = { dateiname: string, dokumentid: string, bytes: number, sha256: string, hash_in_nachricht: boolean, };
export type Pruefbericht = { kopf: Nachrichtenkopf, sha256: string, anlagen: Array<AnlagePruefung>, schemapruefer: string, geprueft_am: string, hinweise: Array<string>, };
export type Richtung = "eingang" | "ausgang";
export type Belegart = "versand" | "transportquittung" | "fachliche_annahme" | "rueckweisung";
export type Nachrichtenbeleg = { id: string, art: Belegart, datum: string, kanal: string, referenz: string, dokument: Dokumentbezug, notiz: string, berichtigt: string | null, };
export type XbauNachricht = { original: Dokumentbezug, richtung: Richtung, pruefung: Pruefbericht, erfasst_am: string, erfasst_von: string, belege: Array<Nachrichtenbeleg>, };
export type Nachrichtenstand = { id: string, revision: number, nachricht: XbauNachricht, };
export type Nachrichtenaufnahme = { original: Dokumentbezug, richtung: Richtung, zuordnung_bestaetigt: boolean, };
export type BerechnungsArt = "stellplaetze" | "wohnflaechen" | "din277" | "vollgeschosse" | "hoehen" | "freiflaechen" | "entwaesserung" | "ueberbaubarkeit";
export type Regelkontext = { bezeichnung: string, version: string, quelle: string, geltungsbereich: string, gueltig_ab: string, gueltig_bis: string, anwendbarkeit_bestaetigt: boolean, pruefvermerk: string, };
export type Einheit = "m" | "m2" | "m3" | "anzahl" | "grad" | "prozent" | "l_pro_s";
export type Messgroesse = "wohnflaeche" | "versiegelungsansatz" | "gruenansatz" | "nutzungsflaeche" | "verkaufsflaeche" | "grundflaeche" | "wohneinheiten" | "sitzplaetze" | "betten" | "beschaeftigte" | "bgf" | "nrf" | "kgf" | "tf" | "vf" | "bri" | "hoehe" | "volumen" | "stellplaetze" | "sonstiges";
export type Uebernahme = { nachweis_id: string, kennzahl: string, ziel: string, };
export type Einstufungsquelle = { nachweis_id: string, kennzahl: string, };
export type Fachquelle = { art: ObjektArt, objekt: string, revision: number, };
export type FachPlan = { gebaeudedaten_uebernehmen?: boolean, modul: BerechnungsArt, bezeichnung: string, grundstueck_id: string | null, gebaeude_id: string | null, regelkontext: Regelkontext, quellen: Array<Dokumentbezug>, fachquellen: Array<Fachquelle>, uebernahmen: Array<Uebernahme>, eingaben: JsonValue, };
export type Kennzahl = { id: string, bezeichnung: string, wert: number | null, einheit: Einheit, groesse: Messgroesse, };
export type Nachweistabelle = { titel: string, spalten: Array<string>, zeilen: Array<Array<string>>, };
export type FachErgebnis = { rechenstand: string, modul: BerechnungsArt, vollstaendig: boolean, offen: Array<string>, hinweise: Array<string>, tabellen: Array<Nachweistabelle>, kennzahlen: Array<Kennzahl>, };
export type Auswahl = { wert: string, titel: string, };
export type Feld = { name: string, titel: string, typ: string, optionen: Array<Auswahl>, readonly: boolean, gruppe: string, referenz: string | null, groesse: Messgroesse | null, };
export type FormTabelle = { pfad: string, titel: string, gruppe: string, felder: Array<Feld>, leer: JsonValue, };
export type BerechnungsVorlage = { id: string, titel: string, hinweis: string, plan: FachPlan, felder: Array<Feld>, tabellen: Array<FormTabelle>, };
export type APunkt = { x: number, y: number, };
export type CadKontur = { linie: number, ebene: string, punkte: Array<APunkt>, quelle: Dokumentbezug, };
export type Tiefenregel = { gebiete: Array<string>, faktor: number, mindesttiefe_m: number, quelle: string, };
export type Dachregel = { ueber_grad: number, faktor: number, quelle: string, };
export type Grenzanlagenregel = { typen: Array<string>, hoehe_m: number, laenge_je_grenze_m: number, laenge_gesamt_m: number, quelle: string, };
export type AbstandsRegeln = { id: string, version: string, land: Bundesland, rechtsstand: string, hinweis: string, tiefen: Array<Tiefenregel>, dach: Array<Dachregel>, grenzanlagen: Grenzanlagenregel | null, };
export type AbstandsRecht = { gemeinde: string, fassung: string, anwendung: string, gebiet: string, satzung: string, planfestsetzung: string, geprueft: boolean, pruefvermerk: string, };
export type AbstandsGrundstueck = { id: string, bezeichnung: string, flurstueck_ids: Array<string>, grenze: Array<APunkt>, innenringe: Array<Array<APunkt>>, grundlage: string, };
export type Flaechenart = "sicherung" | "oeffentliche_flaeche" | "baukoerper";
export type AbstandsFlaeche = { id: string, grundstueck_id: string, art: Flaechenart, bezeichnung: string, polygon: Array<APunkt>, grundlage: string, wirksam: boolean, beleg: Dokumentbezug | null, };
export type Aussenrichtung = "links" | "rechts";
export type Dachart = "flach" | "dachflaeche" | "giebel" | "manuell" | "ungeklaert";
export type Wandfall = "regel" | "garage" | "carport" | "vorbau" | "grenzbebauung" | "bestand" | "sonderbau" | "sonstiges";
export type Sonderansatz = { tiefe_m: number, grundlage: string, wirksam: boolean, beleg: Dokumentbezug | null, };
export type AbstandsWand = { id: string, gebaeude: string, bezeichnung: string, grundstueck_id: string | null, von: APunkt, bis: APunkt, aussen: Aussenrichtung, gelaende_a_m: number | null, gelaende_b_m: number | null, abschluss_a_m: number | null, abschluss_b_m: number | null, dachart: Dachart, dachhoehe_m: number | null, dachneigung_grad: number | null, dachfaktor_manuell: number | null, hoehenquelle: string, dachgrundlage: string, fall: Wandfall, grenzkennung: string, nutzung_geprueft: boolean, sonderansatz: Sonderansatz | null, polygon_manuell: Array<APunkt> | null, };
export type AbstandsPlan = { bezeichnung: string, regeln: AbstandsRegeln, recht: AbstandsRecht, hoehenbezug: string, koordinatenbezug: string, grundstuecke: Array<AbstandsGrundstueck>, waende: Array<AbstandsWand>, flaechen: Array<AbstandsFlaeche>, quellen: Array<Dokumentbezug>, umfang_geprueft: boolean, umfang_vermerk: string, };
export type AbstandsPolygon = { aussen: Array<APunkt>, innen: Array<Array<APunkt>>, };
export type AbstandsStatus = "offen" | "tiefenvorpruefung" | "eingehalten" | "konflikt" | "grenzfall";
export type WandErgebnis = { id: string, bezeichnung: string, laenge_m: number, wandhoehe_m: number | null, dachanteil_m: number | null, h_m: number | null, faktor: number | null, mindesttiefe_m: number | null, tiefe_m: number | null, herleitung: Array<string>, offen: Array<string>, status: AbstandsStatus, flaeche_m2: number | null, ausserhalb_m2: number | null, polygon: Array<APunkt>, konflikte: Array<AbstandsPolygon>, };
export type Ueberdeckung = { wand_a: string, wand_b: string, flaeche_m2: number, polygone: Array<AbstandsPolygon>, };
export type AbstandsErgebnis = { rechenstand: string, waende: Array<WandErgebnis>, ueberdeckungen: Array<Ueberdeckung>, offen: Array<string>, vollstaendig: boolean, konfliktfrei: boolean, };
export type Grundstuecksinformationen = { flurstuecke: Array<FlurstuecksInfo>, };
export type FlurstuecksInfo = { id: string,
/**
 * Name der Gemarkung.
 */
gemarkung: string,
/**
 * Gemarkungsnummer laut Liegenschaftskataster (höchstens vier Ziffern, ohne
 * Landesschlüssel). XBau übermittelt sie statt des Namens.
 */
gemarkungsnummer?: string, flur: string,
/**
 * Flurstücksnummer „Zähler/Nenner“ (Zähler bis fünf, Nenner bis vier Ziffern),
 * siehe [`Self::zaehler_nenner`]. Bestandsangaben in anderer Form bleiben
 * erhalten, neue und geänderte Nummern prüft [`kennzeichen_pruefen`].
 */
nummer: string, flaeche_m2: number, grundbuchbezirk: string, grundbuchblatt: string, laufende_nummer: number | null, grundbuchauszug: Dokumentbezug | null,
/**
 * Frühere Freitextangabe. Sie bleibt lesbar, kann aber nur noch entfernt
 * werden; Eigentümer werden über [`Self::eigentuemer_kunden`] zugeordnet.
 */
eigentuemer: string,
/**
 * Eigentümer aus der Kundendatenbank. Bei Vorhabenflurstücken Bauherr oder
 * Projektbeteiligter mit Verfügungsrecht, siehe [`eigentuemer_pruefen`].
 */
eigentuemer_kunden?: Array<string>, bemerkung: string,
/**
 * Bezug zum Vorhaben; fehlt bei Vorhabenflurstücken.
 */
rolle?: FlurstuecksRolle, };
export type FlurstuecksRolle = "vorhaben" | "nachbar" | "sonstiges";
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
export type Antragsangaben = {
/**
 * Fragen des Vorbescheidsantrags; jede Frage wird ein `beantragterVorbescheid`.
 */
vorbescheidfragen: Array<Vorbescheidfrage>,
/**
 * Genehmigungsfreistellung: bei abschlägiger Entscheidung als Bauantrag weiterführen.
 */
als_bauantrag_weiterfuehren: boolean, };
export type Vorbescheidfrage = { id: string, frage: string, };
export type Verfahrensstatus = "vorbereitung" | "eingereicht" | "unvollstaendig" | "vollstaendig" | "in_pruefung" | "ruhend" | "entschieden" | "zurueckgenommen";
export type Vollstaendigkeit = { datum: string,
/**
 * Eingegangenes Schreiben der Behörde in dieser Akte.
 */
kommunikation: string, };
export type Ruecknahme = { datum: string,
/**
 * Rücknahmeschreiben oder Bestätigung der Behörde in dieser Akte.
 */
kommunikation: string, };
export type Behoerde = { id: string, name: string, kennung: string, kontakt: string, };
export type Bezug = { "art": "dokument", dokument: Dokumentbezug, } | { "art": "nachweis", id: string, } | { "art": "kommunikation", id: string, } | { "art": "nachforderung", id: string, punkt: string, } | { "art": "anforderung", id: string, } | { "art": "abgabe", id: string, } | { "art": "bescheid", id: string, } | { "art": "auflage", bescheid: string, id: string, };
export type Kommunikationsart = "email" | "brief" | "portal" | "telefon" | "besprechung";
export type Kommunikation = { id: string, art: Kommunikationsart, eingehend: boolean, datum: string, betreff: string, beteiligte: string, inhalt: string, original: Dokumentbezug | null, bezuege: Array<Bezug>, };
export type Punktstatus = "offen" | "in_arbeit" | "vorbereitet" | "eingereicht" | "bestaetigt" | "entfallen";
export type Frist = { datum: string, ausloeser: string, beleg: Dokumentbezug | null, erinnerung_tage: number, grund: string, };
export type Forderungspunkt = { id: string, titel: string, grundlage: string, verantwortlich: string | null, frist: Frist | null, status: Punktstatus, antwort: string, dokumente: Array<Dokumentbezug>, abgaben: Array<string>, bestaetigung: string | null, };
export type Nachforderung = { id: string, kommunikation: string, punkte: Array<Forderungspunkt>, };
export type Entscheidung = "genehmigt" | "teilweise_genehmigt" | "abgelehnt" | "sonstiges";
export type Nebenbestimmungsart = "auflage" | "bedingung" | "befristung" | "widerrufsvorbehalt" | "auflagenvorbehalt" | "hinweis";
export type Auflage = { id: string,
/**
 * Ältere Akten ohne diese Angabe lesen sie als Auflage.
 */
art: Nebenbestimmungsart, titel: string, ausloeser: string, verantwortlich: string | null, frist: Frist | null, erledigt: boolean, nachweise: Array<Dokumentbezug>, };
export type Bescheid = { id: string, entscheidung: Entscheidung, datum: string, zugestellt: string | null, original: Dokumentbezug, genehmigte_unterlagen: Array<Dokumentbezug>, auflagen: Array<Auflage>, ersetzt: string | null, };
export type Schrittart = "anhoerung" | "zurueckstellung";
export type Verfahrensschritt = { id: string, art: Schrittart, kommunikation: string,
/**
 * Anhörung: Frist zur Stellungnahme. Zurückstellung: Ende der Zurückstellung.
 */
frist: Frist | null,
/**
 * Ausgehende Stellungnahme dieser Akte; beendet die Wiedervorlage einer Anhörung.
 */
stellungnahme: string | null, verantwortlich: string | null, };
export type Erforderlichkeit = "ungeklaert" | "erforderlich" | "nicht_erforderlich";
export type Anforderung = { id: string, titel: string, erforderlich: Erforderlichkeit, grund: string, quelle: string, dokumente: Array<Dokumentbezug>, freigegeben: boolean, verantwortlich: string | null, };
export type Dokumentbeziehung = { id: string, vorher: Dokumentbezug, nachher: Dokumentbezug, grund: string, bezug: Bezug | null, };
export type Aktenereignis = { id: string, datum: string, erfasst_am: string, erfasst_von: string, text: string, beleg: Dokumentbezug | null, berichtigt: string | null, };
export type Erinnerung = { verfahren: string, objekt: string, titel: string, datum: string, ueberfaellig: boolean, verantwortlich: string | null, };
export type Verfahrensakte = { portal?: Portalakte, bezeichnung: string, art: Verfahrensart,
/**
 * Ältere Akten ohne diese Angaben lesen sie leer.
 */
antragsangaben: Antragsangaben, status: Verfahrensstatus,
/**
 * Ältere Akten ohne diese Angabe lesen sie leer.
 */
vollstaendigkeit?: Vollstaendigkeit, ruecknahme?: Ruecknahme, behoerde: Behoerde, aktenzeichen: string, verantwortlich: string | null, vorgaenger: string | null, profil: Profilstand | null, kommunikation: Array<Kommunikation>, nachforderungen: Array<Nachforderung>,
/**
 * Ältere Akten ohne diese Angabe lesen sie leer.
 */
schritte: Array<Verfahrensschritt>, anforderungen: Array<Anforderung>, dokumentbeziehungen: Array<Dokumentbeziehung>, abgaben: Array<Abgabestand>,
/**
 * Außerhalb der App erfolgte Einreichungen. Ältere Akten ohne diese
 * Angabe lesen sie leer; leer wird sie nicht geschrieben.
 */
externe_einreichungen?: Array<ExterneEinreichung>, bescheide: Array<Bescheid>, ereignisse: Array<Aktenereignis>, };
export type Vorhabenprofil = { bundesland: Bundesland, art: Verfahrensart, gebaeudeklasse: number | null, sonderbau: boolean | null, nutzung: string, };
export type Profilregel = { id: string, titel: string, quelle: string, sonderbau: boolean | null, ab_gebaeudeklasse: number | null, nutzung: string | null, };
export type Anforderungskatalog = { id: string, version: string, titel: string, hinweis: string, bundesland: Bundesland | null, art: Verfahrensart | null, regeln: Array<Profilregel>, };
export type Profilstand = { vorhaben: Vorhabenprofil, katalog: Anforderungskatalog, bestaetigt_von: string | null, bestaetigt_am: string | null, };
export type Versandstatus = "versendet" | "eingegangen" | "zurueckgewiesen" | "ungeklaert";
export type Versandereignis = { id: string, status: Versandstatus, datum: string, weg: string, referenz: string, notiz: string, beleg: Dokumentbezug | null, berichtigt: string | null, };
export type Abgabedokument = { kategorie?: KategorieStand, dokument: Dokumentbezug, name: string, sha256: string, bytes: number, };
export type Artefakt = { format: string, dateiname: string, storage_key: string, sha256: string, bytes: number, };
export type Abgabestand = { id: string, auftrag_hash: string, titel: string, erstellt_am: string, erstellt_von: string, grundlagen: JsonValue, dokumente: Array<Abgabedokument>, anforderungen: Array<Anforderung>, hinweise: Array<string>, artefakte: Array<Artefakt>, versand: Array<Versandereignis>,
/**
 * Vom Core aus Originalereignissen und Berichtigungen abgeleitet.
 */
versandstatus: Versandstatus | null, };
export type Abgabewunsch = { xbau_bezug?: string, id: string, titel: string, dokumente: Array<Dokumentbezug>, xbau: boolean, druck: boolean, anschreiben: string, };
export type Einreichungsweg = "post" | "portal" | "persoenlich" | "email" | "fax" | "sonstiges";
export type ExterneEinreichung = { id: string,
/**
 * Tag der Einreichung bei der Behörde (Ortsdatum); nie nach dem Erfassungstag.
 */
datum: string, weg: Einreichungsweg,
/**
 * Einlieferungs-, Eingangs- oder Portalbeleg, falls vorhanden.
 */
beleg: Dokumentbezug | null, bemerkung: string,
/**
 * Dokumentversionen der Projektablage, die nachweislich eingereicht wurden.
 * Leer, wenn die eingereichten Unterlagen nicht in der Ablage liegen.
 */
dokumente: Array<Dokumentbezug>,
/**
 * Frühere externe Einreichung dieser Akte, die dieser Eintrag berichtigt.
 */
berichtigt: string | null,
/**
 * Vom Core gesetzt.
 */
erfasst_am: string,
/**
 * Vom Core gesetzt.
 */
erfasst_von: string, };
export type ExterneEinreichungswunsch = { id: string, datum: string, weg: Einreichungsweg, beleg: Dokumentbezug | null, bemerkung: string, dokumente: Array<Dokumentbezug>, berichtigt: string | null, };
export type Fristauftrag = { art: Verfahrensart, bescheide: Array<Bescheid>, };
export type Fristberechnung = {
/**
 * Tag des fristauslösenden Ereignisses; er wird nicht mitgezählt (§ 187 Abs. 1 BGB).
 */
beginn: string, ende: string, grundlage: string, hinweise: Array<string>, };
export type Bescheidfristen = { bescheid: string, rechtsbehelfsfrist: Fristberechnung | null, geltungsdauer: Fristberechnung | null,
/**
 * Warum eine Frist nicht berechnet wurde.
 */
hinweise: Array<string>, };
export type Textquelle = "pdf" | "email" | "ocr" | "manuell";
export type Suchtext = { dokument: Dokumentbezug, sha256: string, quelle: Textquelle, text: string, indexiert_am: string, };
export type Suchtreffer = { titel: string, bereich: string, auszug: string, verfahren: string | null, dokument: Dokumentbezug | null, };
export type ObjektArt = "stellplaetze" | "wohnflaechen" | "din277" | "vollgeschosse" | "hoehen" | "freiflaechen" | "entwaesserung" | "ueberbaubarkeit" | "abstandsflaechen" | "verfahren" | "projektorganisation" | "gebaeude" | "aufgaben" | "unterlagen" | "dokumentdaten" | "raumbuch";
export type Projektphase = "vorbereitung" | "vorplanung" | "entwurf" | "genehmigung" | "ausfuehrung" | "abgeschlossen";
export type Projektorganisation = { kennzeichen: string, phase: Projektphase, verantwortlich: BenutzerId | null, beschreibung: string, };
export type Dokumentbezug = { dokument_id: DocumentId, version: number, };
export type Gebaeude = { geometrie?: GebaeudeGeometrie, bezeichnung: string, grundstueck_ids: Array<string>, geschosse: Array<GebaeudeGeschoss>, quelle: Dokumentbezug | null, };
export type GebaeudeGeschoss = { geometrie?: GeschossGeometrie, id: string, einstufungsquelle?: Einstufungsquelle, bezeichnung: string, vollgeschoss: boolean | null, grundlage: string, };
export type GebaeudeGeometrie = { typ?: Bauteiltyp, grundflaeche_m2?: number, nullpunkt_m_nhn?: number, gelaende_m?: number, fundamentsohle_m?: number, traufe_m?: number, first_m?: number, einzelgeschoss?: GeschossGeometrie, herkunft?: string, };
export type GeschossGeometrie = { lichte_hoehe_einheitlich?: boolean, flaeche_m2?: number, fussboden_m?: number, lichte_hoehe_m?: number, geschosshoehe_m?: number, deckendicke_m?: number, decke_oberkante_m?: number, unterliegend_geschoss_id?: string, };
export type GebaeudeGrundlagen = { rechenstand: string, gebaeude_id: string, bezeichnung: string, typ: Bauteiltyp | null, grundflaeche_m2: number | null, geschossflaechensumme_m2: number | null, geschosse: Array<GeschossGrundlagen>, offen: Array<string>, };
export type GeschossGrundlagen = { id: string, bezeichnung: string, implizit: boolean, flaeche_m2: number | null, vollgeschoss: boolean | null, lichte_hoehe_m: number | null, decke_oberkante_m: number | null, decke_ueber_gelaende_m: number | null, unterliegend_m2: number | null, herleitung: string, };
export type AufgabenStatus = "offen" | "in_arbeit" | "erledigt";
export type Aufgabe = { titel: string, beschreibung: string, faellig: string | null, verantwortlich: BenutzerId | null, status: AufgabenStatus, bezug: string | null, };
export type UnterlagenStatus = "entwurf" | "pruefung" | "freigegeben";
export type Unterlage = { vorlage: ChecklistenQuelle | null, bezeichnung: string, erforderlich: boolean, dokument: Dokumentbezug | null, status: UnterlagenStatus, bemerkung: string, };
export type ChecklistenQuelle = { id: string, version: string, punkt: string, };
export type Checklistenpunkt = { id: string, bezeichnung: string, erforderlich: boolean, };
export type Checkliste = { id: string, version: string, titel: string, hinweis: string, unterlagen: Array<Checklistenpunkt>, };
export type Statushinweis = { bereich: string, text: string, };
export type Projektstatus = { ermittelt_am: string, offene_aufgaben: number, ueberfaellige_aufgaben: number, erforderliche_unterlagen: number, fehlende_unterlagen: number, ungepruefte_unterlagen: number, nachweise: number, veraltete_nachweise: number, offene_freigaben: number, exportierte_pakete: number, bestaetigte_einreichungen: number, hinweise: Array<Statushinweis>, };
export type Dringlichkeit = "ueberfaellig" | "heute" | "handeln" | "pruefen" | "warten";
export type Themenbereich = "grundlagen" | "aufgaben" | "unterlagen" | "nachweise" | "verfahren" | "einreichungen";
export type UebersichtsZiel = { bereich: string, objekt: string | null, register: string | null, eintrag: string | null, };
export type Arbeitspunkt = { id: string, thema: Themenbereich, dringlichkeit: Dringlichkeit, titel: string, hinweis: string, faellig: string | null, verantwortlich: string | null, ziel: UebersichtsZiel, };
export type UebersichtsZahlen = { offene_aufgaben: number, fehlende_unterlagen: number, nachweise_pruefen: number, offene_nachforderungen: number, offene_auflagen: number, ueberfaellig: number, };
export type VerfahrensUeberblick = { id: string, bezeichnung: string, behoerde: string, aktenzeichen: string, status: string, offene_punkte: number, offene_auflagen: number, abgaben: number, versendet: number, bescheide: number, entscheidung: Entscheidung | null, entscheidungsdatum: string | null, };
export type AbgabeUeberblick = { id: string, titel: string, empfaenger: string, quelle: string, status: string, datum: string | null, dokumente: number, ziel: UebersichtsZiel, };
export type EmpfaengerDokument = { dokument_id: string, dateiname: string, eingereichte_version: bigint, aktuelle_version: bigint | null, eingereicht_am: string, status: string, };
export type EmpfaengerUeberblick = { empfaenger: string, dokumente: Array<EmpfaengerDokument>, ziel: UebersichtsZiel, };
export type Projektuebersicht = { projekt_id: ProjectId, projekt_version: bigint, ermittelt_am: string, stichtag: string, zahlen: UebersichtsZahlen, punkte: Array<Arbeitspunkt>, verfahren: Array<VerfahrensUeberblick>, abgaben: Array<AbgabeUeberblick>, empfaenger: Array<EmpfaengerUeberblick>, hinweise: Array<string>, };
export type Dokumentdaten = { dokument: Dokumentbezug, dokumentart: string, plannummer: string, revisionsindex: string, massstab: string, verfasser: string, datum: string | null, };
export type Raumbuch = { bezeichnung: string, gebaeude_id: string, geschoss_id: string | null, raeume: Array<Raum>, };
export type Raum = { id: string, nummer: string, bezeichnung: string, nutzung: string, flaeche_m2: number, hoehe_m: number | null, };
export type RaumSummen = { anzahl: number, flaeche_m2: number, volumen_m3: number | null, };
export type NutzungsArt = "grz" | "gfz" | "geschossigkeit" | "bmz";
export type NachweisEingabe = { "art": "fach", objekt: string, plan: FachPlan, } | { "art": "abstand_projekt", objekt: string, plan: AbstandsPlan, } | { "art": "nutzung", fach: NutzungsArt, plan: BaugrundstuecksPlan, } | { "art": "abstand", gegenstand: string, eingabe: AbstandsflaecheInput, };
export type RegelArchiv = { id: string, version: string, sha256: string, format: string, inhalt: string, pruefstatus: string, };
export type Nachweisstand = { id: string, bezeichnung: string, erstellt_am: string, erstellt_von: string, land: Bundesland, rechtskontext: Planungsrecht, projekt_version: number, eingabe: NachweisEingabe, grundlagen_hash: string, regeln: Array<RegelArchiv>, gebaeude_hash: string, anfrage_hash: string, rechenstand: string, ergebnis: JsonValue, vollstaendig: boolean, };
export type Freigabe = { nachweis_id: string, grundlagen_hash: string, freigegeben_am: string, freigegeben_von: string, bemerkung: string, };
export type Druckquelle = { "art": "dokument", dokument_id: DocumentId, version: number, seiten: Array<number>, } | { "art": "nachweis", id: string, } | { "art": "baugrundstueck" } | { "art": "baugrenzen" } | { "art": "raumbuch", id: string, revision: number, };
export type Druckauftrag = {
/**
 * Fehlend: gespeicherter Bürostandard. Explizit: Gestaltung dieser Ausgabe.
 */
gestaltung: Druckgestaltung | null, aktenzeichen: string, datum: string | null, titel: string, absender: string, empfaenger: string, anschreiben: string, mit_anschreiben: boolean, inhaltsverzeichnis: boolean,
/**
 * Schützt aktuelle Fachberichte vor Änderungen während der Zusammenstellung.
 */
projekt_version: number, quellen: Array<Druckquelle>, };
export type Druckgestaltung = { kopfzeile: string, fusszeile: string, anschreiben: string, };
export type Druckeintrag = { titel: string, quelle: Druckquelle, hinweis: string | null, verfuegbar: boolean, };
export type Druckkatalog = { gestaltung: Druckgestaltung, projekt_version: number, absender: string, eintraege: Array<Druckeintrag>, };
export type Druckposition = { titel: string, erste_seite: number, seiten: number, };
export type NachweisAnsicht = { stand: Nachweisstand, aktuell: boolean, freigabe: Freigabe | null, };
