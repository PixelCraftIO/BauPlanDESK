// Automatisch erzeugt: cargo run -p core-lib --example vertraege. Nicht von Hand ändern.
import type { BaugrundstuecksPlan, BaugrundstueckBerechnung, FlaechenAbzug, Planungsrecht } from '../api';
export type Plankalibrierung = { "art": "massstab", nenner: number, } | { "art": "strecke", von: [number, number], bis: [number, number], meter: number, };
export type Planebene = { id: string, bezeichnung: string, sichtbar: boolean, gesperrt: boolean, };
export type Plangeometrie = { "art": "polygon", punkte: Array<[number, number]>, } | { "art": "rechteck", von: [number, number], bis: [number, number], } | { "art": "kreis", mitte: [number, number], rand: [number, number], } | { "art": "strecke", von: [number, number], bis: [number, number], } | { "art": "hoehenpunkt", punkt: [number, number], };
export type Planbedeutung = "unbestimmt" | "flurstueck" | "private_gruenflaeche" | "versiegelung" | "geschoss_aussenkante" | "gebaeudegrundflaeche" | "raum" | "aufenthaltsraum" | "nuf" | "tf" | "vf" | "kgf" | "aussenwand" | "gelaende" | "traufe" | "first" | "dach" | "nullpunkt" | "fussboden" | "decke" | "fundamentsohle" | "hoehenmass";
export type Planmarkierung = { id: string, ebene_id: string, bezeichnung: string, bedeutung: Planbedeutung, geometrie: Plangeometrie, grundstueck_id: string | null, gebaeude_id: string | null, geschoss_id: string | null,
/**
 * Ausschnitt, in dem die Markierung gezeichnet wurde; fehlt bei Altdaten.
 */
ausschnitt_id?: string, };
export type Planhoehenbezug = { punkt: [number, number], hoehe_m: number,
/**
 * Zum Beispiel „m ü. NHN“ oder „lokal, OKFF EG = 0“.
 */
system: string, };
export type Ausschnittart = "grundriss" | "lageplan" | "schnitt" | "ansicht" | "detail";
export type Planbezugslinie = { von: [number, number], bis: [number, number], hoehe_m: number, system: string, };
export type Planausschnitt = { id: string, bezeichnung: string, art: Ausschnittart, von: [number, number], bis: [number, number], kalibrierung?: Plankalibrierung, bezugslinie?: Planbezugslinie, };
export type Planaufmass = { bezeichnung: string, dokument: Dokumentbezug,
/**
 * Einsbasiert. Mehrere Aufmaße derselben Seite dürfen verschiedene Maßstäbe haben.
 */
seite: number, kalibrierung: Plankalibrierung | null, hoehenbezug: Planhoehenbezug | null, ebenen: Array<Planebene>, markierungen: Array<Planmarkierung>,
/**
 * Optionale Ausschnitte; ohne sie gelten Maßstab und Höhenbezug der Seite.
 */
ausschnitte?: Array<Planausschnitt>, };
export type Planmesswert = { id: string, laenge_m: number | null, flaeche_m2: number | null, horizontale_m: number | null, hoehe_m: number | null, neigung_grad: number | null, offen: Array<string>, };
export type Planmessgroesse = "flaeche" | "laenge" | "horizontale" | "hoehe" | "neigung";
export type Planbindung = { aufmass_id: string, revision: number, markierung_id: string, groesse: Planmessgroesse,
/**
 * Tabelle.Zeilen-ID.Feld; niemals Arraypositionen.
 */
ziel: string, };
export type Planziel = { ziel: string, titel: string, groesse: Planmessgroesse, };
export type Aufgabengeometrie = "flaeche" | "hoehenpunkt" | "strecke";
export type Planaufgabe = {
/**
 * `gebaeude` oder `abstand`.
 */
verbraucher: string,
/**
 * Gebäude-ID bzw. ID des Abstandsplans.
 */
objekt: string, objekt_name: string,
/**
 * Anzeigegruppe, z. B. „Lagerhalle · OG“ oder „Südwand“.
 */
gruppe: string, ziel: string, titel: string, groesse: Planmessgroesse, bedeutung: Planbedeutung, geometrie: Aufgabengeometrie, gebaeude_id: string | null, geschoss_id: string | null, grundstueck_id: string | null,
/**
 * Zielfeld trägt bereits einen (manuellen) Wert.
 */
wert_vorhanden: boolean,
/**
 * Bereits verknüpfte Markierung (aus einem beliebigen Aufmaß).
 */
gebunden: string | null,
/**
 * Passende, noch nicht verknüpfte Markierung dieses Aufmaßes.
 */
markierung: string | null, hinweis: string | null, };
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
export type FachPlan = { planbindungen?: Array<Planbindung>, gebaeudedaten_uebernehmen?: boolean, modul: BerechnungsArt, bezeichnung: string, grundstueck_id: string | null, gebaeude_id: string | null, regelkontext: Regelkontext, quellen: Array<Dokumentbezug>, fachquellen: Array<Fachquelle>, uebernahmen: Array<Uebernahme>, eingaben: JsonValue, };
export type Kennzahl = { id: string, bezeichnung: string, wert: number | null, einheit: Einheit, groesse: Messgroesse, };
export type Nachweistabelle = { titel: string, spalten: Array<string>, zeilen: Array<Array<string>>, };
export type FachErgebnis = { rechenstand: string, modul: BerechnungsArt, vollstaendig: boolean, offen: Array<string>, hinweise: Array<string>, tabellen: Array<Nachweistabelle>, kennzahlen: Array<Kennzahl>, };
export type Auswahl = { wert: string, titel: string, };
export type Feld = { name: string, titel: string, typ: string, optionen: Array<Auswahl>, readonly: boolean, gruppe: string, referenz: string | null, groesse: Messgroesse | null, };
export type FormTabelle = { pfad: string, titel: string, gruppe: string, felder: Array<Feld>, leer: JsonValue, };
export type BerechnungsVorlage = { id: string, titel: string, hinweis: string, plan: FachPlan, felder: Array<Feld>, tabellen: Array<FormTabelle>, };
export type APunkt = { x: number, y: number, };
export type CadKontur = { linie: number, ebene: string, punkte: Array<APunkt>, quelle: Dokumentbezug, };
export type Tiefenregel = { gebiete: Array<string>, faktor: number, mindesttiefe_m: number, quelle: string,
/**
 * Besonderer Tiefenfall; fehlt bei der Regeltiefe.
 */
fall?: Tiefenfall, };
export type Dachregel = { ueber_grad: number, faktor: number, quelle: string, };
export type Grenzanlagenregel = { typen: Array<string>, hoehe_m: number, laenge_je_grenze_m: number, laenge_gesamt_m: number, quelle: string, };
export type AbstandsRegeln = { id: string, version: string, land: Bundesland, rechtsstand: string, hinweis: string, tiefen: Array<Tiefenregel>, dach: Array<Dachregel>, grenzanlagen: Grenzanlagenregel | null,
/**
 * Fehlt in Paketen vor Version 2: dann [`Wandhoehenansatz::Maximal`].
 */
wandhoehe?: Wandhoehenansatz,
/**
 * Automatische Giebelanrechnung; ohne Regel bleibt der manuelle Ansatz.
 */
giebel?: Giebelregel, gauben?: Gaubenregel,
/**
 * Ausnahme vom Überdeckungsverbot nach Winkel; ohne Regel keine Ausnahme.
 */
ueberdeckung?: Ueberdeckungsregel, };
export type AbstandsRecht = { gemeinde: string, fassung: string, anwendung: string, gebiet: string, satzung: string, planfestsetzung: string, geprueft: boolean, pruefvermerk: string, };
export type AbstandsGrundstueck = { id: string, bezeichnung: string, flurstueck_ids: Array<string>, grenze: Array<APunkt>, innenringe: Array<Array<APunkt>>, grundlage: string, };
export type Flaechenart = "sicherung" | "oeffentliche_flaeche" | "baukoerper";
export type AbstandsFlaeche = { id: string, grundstueck_id: string, art: Flaechenart, bezeichnung: string, polygon: Array<APunkt>, grundlage: string, wirksam: boolean, beleg: Dokumentbezug | null, };
export type Aussenrichtung = "links" | "rechts";
export type Dachart = "flach" | "dachflaeche" | "giebel" | "manuell" | "ungeklaert";
export type Wandfall = "regel" | "garage" | "carport" | "nebengebaeude" | "vorbau" | "grenzbebauung" | "bestand" | "sonderbau" | "sonstiges";
export type Sonderansatz = { tiefe_m: number, grundlage: string, wirksam: boolean, beleg: Dokumentbezug | null, };
export type AbstandsWand = { id: string,
/**
 * Optionaler Bezug zu Gebäude/Geschosse. Mit Bezug übernimmt der Core die
 * Gebäudebezeichnung; fehlt in älteren Plänen.
 */
gebaeude_id?: string, gebaeude: string, bezeichnung: string, grundstueck_id: string | null, von: APunkt, bis: APunkt, aussen: Aussenrichtung, gelaende_a_m: number | null, gelaende_b_m: number | null, abschluss_a_m: number | null, abschluss_b_m: number | null, dachart: Dachart, dachhoehe_m: number | null, dachneigung_grad: number | null, dachfaktor_manuell: number | null, hoehenquelle: string, dachgrundlage: string, fall: Wandfall, grenzkennung: string, nutzung_geprueft: boolean, sonderansatz: Sonderansatz | null, polygon_manuell: Array<APunkt> | null,
/**
 * Geländehöhen entlang der Wand. Ersetzt `gelaende_a_m`/`gelaende_b_m`;
 * fehlt in älteren Plänen.
 */
gelaende?: Array<Hoehenpunkt>,
/**
 * Wandoberkante (Schnittlinie mit der Dachhaut bzw. oberer Abschluss).
 * Zwei Punkte an derselben Station bilden einen Höhensprung und damit eine
 * Wandteilgrenze. Ersetzt `abschluss_a_m`/`abschluss_b_m`.
 */
oberkante?: Array<Hoehenpunkt>,
/**
 * Zusätzliche vertikale Wandteilgrenzen (Station in m ab Wandanfang).
 */
wandteilgrenzen?: Array<number>,
/**
 * Neigung der zweiten Dachseite bei Giebeln.
 */
dachneigung_b_grad?: number,
/**
 * Gesamtlänge der Gauben/Dachaufbauten auf der Dachfläche über dieser Wand.
 */
gauben_laenge_m?: number, tiefenfall?: Tiefenfall,
/**
 * Begründung für einen besonderen Tiefenfall.
 */
tiefenfall_grundlage?: string,
/**
 * Aus einem Gebäudeumriss erzeugte Seite; Lage und Eckhöhen führt der Umriss.
 */
umriss?: UmrissSeite, };
export type AbstandsPlan = { planbindungen?: Array<Planbindung>, bezeichnung: string, regeln: AbstandsRegeln, recht: AbstandsRecht, hoehenbezug: string, koordinatenbezug: string, grundstuecke: Array<AbstandsGrundstueck>, waende: Array<AbstandsWand>, flaechen: Array<AbstandsFlaeche>, quellen: Array<Dokumentbezug>, umfang_geprueft: boolean, umfang_vermerk: string, umrisse?: Array<AbstandsUmriss>, };
export type AbstandsPolygon = { aussen: Array<APunkt>, innen: Array<Array<APunkt>>, };
export type AbstandsStatus = "offen" | "tiefenvorpruefung" | "eingehalten" | "konflikt" | "grenzfall";
export type WandErgebnis = { id: string, bezeichnung: string, laenge_m: number, wandhoehe_m: number | null, dachanteil_m: number | null, h_m: number | null, faktor: number | null, mindesttiefe_m: number | null, tiefe_m: number | null, herleitung: Array<string>, offen: Array<string>, status: AbstandsStatus, flaeche_m2: number | null, ausserhalb_m2: number | null, polygon: Array<APunkt>, konflikte: Array<AbstandsPolygon>,
/**
 * Angewandter Wandhöhenansatz; fehlt in Ergebnissen vor Version 2.
 */
ansatz?: Wandhoehenansatz, wandteile?: Array<Wandteilergebnis>,
/**
 * Höhenverlauf an allen Profil- und Teilungsstellen.
 */
profil?: Array<Profilzeile>, };
export type Ueberdeckung = { wand_a: string, wand_b: string, flaeche_m2: number, polygone: Array<AbstandsPolygon>,
/**
 * Winkel, in dem die Wände zueinander stehen (0° = parallel gegenüber).
 */
winkel_grad?: number,
/**
 * Rechtsgrundlage, nach der die Überdeckung zulässig ist.
 */
zulaessig?: string, };
export type AbstandsErgebnis = { rechenstand: string, waende: Array<WandErgebnis>, ueberdeckungen: Array<Ueberdeckung>, offen: Array<string>, vollstaendig: boolean, konfliktfrei: boolean, };
export type Tiefenfall = "regel" | "oeffentliche_flaeche" | "wohngebaeude_gk12";
export type Wandhoehenansatz = "maximal" | "mittel_kanten" | "mittel_profil" | "punktweise";
export type Giebelregel = {
/**
 * Haben beide Dachseiten eine größere Neigung, gilt `faktor_steil`.
 */
steil_ueber_grad: number, faktor_steil: number, faktor_sonst: number, quelle: string, };
export type Gaubenregel = {
/**
 * Anteil der Gaubenlänge an der darunterliegenden Wand, ab dem `faktor` gilt.
 */
anteil_ueber: number, faktor: number, quelle: string, };
export type Ueberdeckungsregel = {
/**
 * Überdeckung zulässig, wenn die Wände in einem größeren Winkel zueinander stehen.
 */
winkel_ueber_grad: number, quelle: string, };
export type Hoehenpunkt = { station_m: number, hoehe_m: number, quelle?: string, };
export type UmrissSeite = { umriss_id: string,
/**
 * Seite von Ecke `seite` zu Ecke `seite + 1`.
 */
seite: number, };
export type UmrissEcke = { x: number, y: number,
/**
 * Maßgebliche Geländehöhe an der Gebäudeecke.
 */
gelaende_m?: number, };
export type AbstandsUmriss = { id: string, bezeichnung: string, gebaeude_id?: string, grundstueck_id: string | null, ecken: Array<UmrissEcke>,
/**
 * Vorgabe der Wandoberkante (z. B. Traufe) für neue Seiten.
 */
oberkante_m: number | null, quelle: string, };
export type Wandteilergebnis = { von_m: number, bis_m: number,
/**
 * Wandhöhe an den Kanten des Wandteils.
 */
h_anfang_m: number, h_ende_m: number,
/**
 * Maßgebliche Wandhöhe nach dem Ansatz, ohne Dachanteil.
 */
wandhoehe_m: number, h_m: number | null,
/**
 * Größte Tiefe im Wandteil.
 */
tiefe_m: number | null, };
export type Profilzeile = { station_m: number, gelaende_m: number, oberkante_m: number, wandhoehe_m: number,
/**
 * Tiefe an dieser Stelle (bei gemittelten Ansätzen die des Wandteils).
 */
tiefe_m: number | null,
/**
 * Laufende Nummer des Wandteils ab 1.
 */
wandteil: number, };
export type UmrissAbgleich = { plan: AbstandsPlan, hinweise: Array<string>, };
export type CadHoehe = {
/**
 * Lage in Metern.
 */
x: number, y: number,
/**
 * Höhe wie in der Zeichnung angegeben (m).
 */
hoehe_m: number, ebene: string,
/**
 * „Höhenkote“ (Text) oder „Höhenpunkt“ (Punkt mit z).
 */
art: string, quelle: Dokumentbezug, };
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
export type BauNvo = "1962" | "1968" | "1977" | "1990" | "2017" | "2021";
export type Bauteiltyp = "hauptanlage" | "terrasse_hauptanlage" | "garage" | "garage_abstandsflaeche" | "stellplatz" | "zufahrt" | "nebenanlage14" | "unterirdisch" | "balkon_loggia" | "terrasse_nebenanlage";
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
export type Verfahrensart = "bauantrag" | "vorbescheid" | "aenderungsantrag" | "teilbaugenehmigung" | "freistellung" | "nutzungsaenderung" | "beseitigung" | "widerspruch" | "sonstiges";
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
export type Verlaengerung = {
/**
 * Bescheid, dessen Geltungsdauer verlängert wird (Genehmigung oder
 * frühere Verlängerung).
 */
bescheid: string,
/**
 * Neues Ende der Geltungsdauer laut Verlängerungsbescheid.
 */
bis: string, };
export type Bauausfuehrung = { baubeginn: string, fertigstellung?: string, };
export type Bescheid = { id: string, entscheidung: Entscheidung, datum: string, zugestellt: string | null, original: Dokumentbezug, genehmigte_unterlagen: Array<Dokumentbezug>, auflagen: Array<Auflage>, ersetzt: string | null,
/**
 * Verlängerungsbescheid: verlängert die Geltungsdauer eines früheren
 * Bescheids dieser Akte. Ältere Bescheide lesen die Angabe leer.
 */
verlaengerung?: Verlaengerung,
/**
 * Teilbaugenehmigung im laufenden Bauantrag: genehmigt einen Teil der
 * Bauarbeiten vorab und beendet das Verfahren nicht. Ältere Bescheide
 * lesen die Angabe als `false`.
 */
teilbaugenehmigung?: boolean, };
export type Schrittart = "anhoerung" | "zurueckstellung" | "einvernehmen" | "fachbeteiligung" | "nachbarbeteiligung";
export type Verfahrensschritt = { id: string, art: Schrittart, kommunikation: string,
/**
 * Anhörung: Frist zur Stellungnahme. Zurückstellung: Ende der Zurückstellung.
 */
frist: Frist | null,
/**
 * Ausgehende Stellungnahme dieser Akte; beendet die Wiedervorlage einer Anhörung.
 */
stellungnahme: string | null, verantwortlich: string | null,
/**
 * Zurückstellung: eingegangenes Folgeschreiben der Behörde, das sie vor
 * ihrem Fristende beendet. Ältere Akten lesen die Angabe leer.
 */
folgeschreiben?: string, };
export type Pruefverfahren = "regulaer" | "vereinfacht";
export type Erforderlichkeit = "ungeklaert" | "erforderlich" | "nicht_erforderlich";
export type Anforderung = { id: string, titel: string, erforderlich: Erforderlichkeit, grund: string, quelle: string, dokumente: Array<Dokumentbezug>, freigegeben: boolean, verantwortlich: string | null, };
export type Dokumentbeziehung = { id: string, vorher: Dokumentbezug, nachher: Dokumentbezug, grund: string, bezug: Bezug | null, };
export type Ereignisart = "angelegt" | "status" | "vollstaendigkeit" | "ruecknahme" | "kommunikation" | "nachforderung" | "schritt" | "abgabe" | "versand" | "bescheid" | "zustellung" | "nebenbestimmung" | "ausfuehrung" | "berichtigung" | "notiz" | "angaben";
export type Berichtigungsgegenstand = { "art": "zustellung", bescheid: string, } | { "art": "vollstaendigkeit" } | { "art": "ruecknahme" } | { "art": "baubeginn" } | { "art": "fertigstellung" };
export type Wertberichtigung = { gegenstand: Berichtigungsgegenstand, grund: string, vorher: string, nachher: string, };
export type Aktenereignis = { id: string, datum: string, erfasst_am: string, erfasst_von: string, text: string, beleg: Dokumentbezug | null, berichtigt: string | null,
/**
 * Vom Core abgeleitete Art der Verfahrensänderung (Zeitleiste). Manuell
 * erfasste Ereignisse und ältere Einträge haben keine.
 */
arten?: Array<Ereignisart>,
/**
 * Begründete Berichtigung eines erfassten Werts; alter und neuer Wert
 * werden vom Core eingetragen.
 */
berichtigung?: Wertberichtigung, };
export type Einreichungsstatus = "entwurf" | "eingereicht" | "nachgereicht" | "genehmigt" | "ersetzt";
export type Planstand = { version: number, planindex: string | null, status: Einreichungsstatus,
/**
 * Datum der ersten Einreichung dieser Version.
 */
eingereicht_am: string | null,
/**
 * Versendete Abgabestände, die diese Version enthielten, in Einreichungsfolge.
 */
abgaben: Array<string>,
/**
 * Wirksamer Bescheid, der diese Version genehmigt.
 */
bescheid: string | null, ersetzt_durch: Dokumentbezug | null, };
export type Planunterlage = { dokument_id: DocumentId,
/**
 * Dateiname aus dem jüngsten Abgabestand.
 */
name: string,
/**
 * Zuletzt vergebener Planindex; `None`, solange nichts eingereicht ist.
 */
planindex: string | null,
/**
 * Status der zuletzt eingereichten Version, sonst Entwurf.
 */
status: Einreichungsstatus,
/**
 * Neuere, noch nicht eingereichte Version (DMS oder vorbereiteter Abgabestand).
 */
offene_version: number | null,
/**
 * Nach Version aufsteigend.
 */
staende: Array<Planstand>,
/**
 * Abweichungen, z. B. ein im Anlagenverzeichnis anders ausgewiesener Index.
 */
hinweise: Array<string>, };
export type Terminart = "forderungspunkt" | "nebenbestimmung" | "ablauf" | "anhoerung" | "zurueckstellung" | "rechtsbehelfsfrist" | "geltungsdauer" | "fertigstellung" | "beteiligung" | "bearbeitungsfrist";
export type Termin = { verfahren: string, verfahrensbezeichnung: string,
/**
 * Akteneintrag, zu dem der Termin gehört (Punkt, Nebenbestimmung, Schritt
 * oder Bescheid).
 */
objekt: string, art: Terminart, titel: string, datum: string,
/**
 * Erinnerung so viele Tage vor dem Datum.
 */
vorlauf_tage: number, verantwortlich: string | null,
/**
 * Der Termin endet mit seinem Datum und wird danach nicht überfällig.
 */
endet_mit_ablauf: boolean,
/**
 * Kurze Erläuterung, z. B. die Rechtsgrundlage einer berechneten Frist.
 */
hinweis: string, };
export type Erinnerung = { verfahren: string, objekt: string, titel: string, datum: string, ueberfaellig: boolean, verantwortlich: string | null,
/**
 * Art der Wiedervorlage; bestimmt, wann sie endet.
 */
art: Terminart, };
export type Verfahrensakte = { portal?: Portalakte, bezeichnung: string, art: Verfahrensart,
/**
 * Ältere Akten ohne diese Angaben lesen sie leer.
 */
antragsangaben: Antragsangaben, status: Verfahrensstatus,
/**
 * Ältere Akten ohne diese Angabe lesen sie leer.
 */
vollstaendigkeit?: Vollstaendigkeit, ruecknahme?: Ruecknahme,
/**
 * Baubeginn und Fertigstellung; ältere Akten lesen die Angabe leer.
 */
ausfuehrung?: Bauausfuehrung, behoerde: Behoerde, aktenzeichen: string, verantwortlich: string | null, vorgaenger: string | null,
/**
 * Regulär oder vereinfacht; ältere Akten lesen die Angabe leer.
 */
pruefverfahren?: Pruefverfahren,
/**
 * Bearbeitungsfrist oder Eintritt einer Genehmigungsfiktion, wie sie die
 * Behörde, etwa in der Eingangs- oder Vollständigkeitsbestätigung, nennt.
 * Ältere Akten lesen die Angabe leer.
 */
bearbeitungsfrist?: Frist, profil: Profilstand | null, kommunikation: Array<Kommunikation>, nachforderungen: Array<Nachforderung>,
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
export type Abgabedokument = { kategorie?: KategorieStand,
/**
 * Beim Festhalten im Anlagenverzeichnis ausgewiesener Planindex (Index bei
 * Einreichung als nächster Stand). Maßgebend ist die Ableitung in
 * [`super::planindex::ableiten`]; Abweichungen meldet sie. Fehlt in älteren
 * Abgabeständen.
 */
planindex?: string, dokument: Dokumentbezug, name: string, sha256: string, bytes: number, };
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
export type Bescheidfristen = { bescheid: string, rechtsbehelfsfrist: Fristberechnung | null,
/**
 * Frist für den Baubeginn (Erlöschen der Genehmigung).
 */
geltungsdauer: Fristberechnung | null,
/**
 * Frist für die Fertigstellung, wo die Landesbauordnung eine eigene
 * Frist kennt (Berlin).
 */
fertigstellung: Fristberechnung | null,
/**
 * Warum eine Frist nicht berechnet wurde, und Hinweise nach Verfahrensart.
 */
hinweise: Array<string>,
/**
 * Stand der Fristregeln, nach dem gerechnet wurde.
 */
regelstand: string, };
export type Textquelle = "pdf" | "email" | "ocr" | "manuell";
export type Suchtext = { dokument: Dokumentbezug, sha256: string, quelle: Textquelle, text: string, indexiert_am: string, };
export type Suchtreffer = { titel: string, bereich: string, auszug: string, verfahren: string | null, dokument: Dokumentbezug | null, };
export type ObjektArt = "stellplaetze" | "wohnflaechen" | "din277" | "vollgeschosse" | "hoehen" | "freiflaechen" | "entwaesserung" | "ueberbaubarkeit" | "abstandsflaechen" | "verfahren" | "projektorganisation" | "gebaeude" | "aufgaben" | "unterlagen" | "dokumentdaten" | "raumbuch" | "planaufmass";
export type Projektphase = "vorbereitung" | "vorplanung" | "entwurf" | "genehmigung" | "ausfuehrung" | "abgeschlossen";
export type Projektorganisation = { kennzeichen: string, phase: Projektphase, verantwortlich: BenutzerId | null, beschreibung: string, };
export type Dokumentbezug = { dokument_id: DocumentId, version: number, };
export type Gebaeude = { planbindungen?: Array<Planbindung>, geometrie?: GebaeudeGeometrie, bezeichnung: string,
/**
 * Flurstücke (IDs der Grundstücksinformationen), auf denen das Gebäude
 * steht, bei Vereinigungsbaulast auch mehrere. Das Baugrundstück leitet der
 * Core je Berechnung daraus ab. Fehlt in älteren Ständen.
 */
flurstueck_ids?: Array<string>,
/**
 * `true`: Die Flurstücke stammen aus einer früheren Baugrundstückszuordnung
 * und sind noch auf die tatsächlich bebauten Flurstücke zu prüfen.
 */
flurstuecke_zu_pruefen?: boolean,
/**
 * Veraltet: frühere Zuordnung zu Baugrundstücken. Wird nur noch gelesen,
 * solange keine Flurstücke zugeordnet sind; neu wird sie nicht gesetzt.
 */
grundstueck_ids: Array<string>, geschosse: Array<GebaeudeGeschoss>, quelle: Dokumentbezug | null, };
export type GebaeudeGeschoss = { geometrie?: GeschossGeometrie, id: string, einstufungsquelle?: Einstufungsquelle, bezeichnung: string, vollgeschoss: boolean | null, grundlage: string, };
export type GebaeudeGeometrie = { typ?: Bauteiltyp, grundflaeche_m2?: number, nullpunkt_m_nhn?: number, gelaende_m?: number, fundamentsohle_m?: number, traufe_m?: number, first_m?: number, einzelgeschoss?: GeschossGeometrie,
/**
 * Geprüfte Vollgeschosseinstufung des Einzelgeschosses (nur ohne einzeln
 * erfasste Geschosse); `None` = noch zu prüfen.
 */
einzelgeschoss_vollgeschoss?: boolean,
/**
 * Grundlage der Einstufung des Einzelgeschosses.
 */
einzelgeschoss_grundlage?: string, herkunft?: string, };
export type GeschossGeometrie = { lichte_hoehe_einheitlich?: boolean, flaeche_m2?: number, fussboden_m?: number, lichte_hoehe_m?: number, geschosshoehe_m?: number, deckendicke_m?: number, decke_oberkante_m?: number, unterliegend_geschoss_id?: string, };
export type GebaeudeGrundlagen = { rechenstand: string, gebaeude_id: string, bezeichnung: string, typ: Bauteiltyp | null, grundflaeche_m2: number | null, geschossflaechensumme_m2: number | null, geschosse: Array<GeschossGrundlagen>, offen: Array<string>,
/**
 * Absolute Höhen (m ü. NHN) aus Gebäudenullpunkt und relativer Angabe;
 * `None`, solange eine der beiden Angaben fehlt. Fehlt in älteren Ständen.
 */
hoehen_nhn: HoehenNhn, };
export type GeschossGrundlagen = { id: string, bezeichnung: string, implizit: boolean, flaeche_m2: number | null, vollgeschoss: boolean | null, lichte_hoehe_m: number | null, decke_oberkante_m: number | null, decke_ueber_gelaende_m: number | null, unterliegend_m2: number | null, herleitung: string, };
export type HoehenNhn = { gelaende_m: number | null, traufe_m: number | null, first_m: number | null, };
export type Flurstuecksverweis = { flurstueck_id: string,
/**
 * Fachbereich des verweisenden Objekts: `gebaeude` oder `abstandsflaechen`.
 */
bereich: string, objekt: string, bezeichnung: string, };
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
export type Raumbuch = { planbindungen?: Array<Planbindung>, bezeichnung: string, gebaeude_id: string, geschoss_id: string | null, raeume: Array<Raum>, };
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
 * Deckblatt mit Angaben zum Bauvorhaben und Unterschriftsfeldern.
 */
deckblatt: boolean,
/**
 * Trennblatt mit Anlagennummer vor jeder unverändert beigefügten
 * Originalunterlage (Formulare, Pläne, Fotos).
 */
trennblaetter: boolean,
/**
 * Aufstellende Person für Aufstellungsvermerk und Deckblatt. Fehlend:
 * Vorschlag aus Bauantrag und Büroangaben (siehe [`Druckkatalog::aufgestellt_von`]).
 */
aufgestellt_von: string | null,
/**
 * Schützt aktuelle Fachberichte vor Änderungen während der Zusammenstellung.
 */
projekt_version: number, quellen: Array<Druckquelle>, };
export type Druckgestaltung = { kopfzeile: string, fusszeile: string, anschreiben: string, };
export type Druckeintrag = { titel: string, quelle: Druckquelle, hinweis: string | null, verfuegbar: boolean,
/**
 * Gehört zum empfohlenen Berechnungsheft: aktueller, jüngster Stand je
 * Berechnungsgegenstand in fachlicher Reihenfolge.
 */
empfohlen: boolean, };
export type Druckkatalog = { gestaltung: Druckgestaltung, projekt_version: number, absender: string, eintraege: Array<Druckeintrag>,
/**
 * Vorschlag für die aufstellende Person; leer, wenn weder Bauantrag noch
 * Büroangaben eine/n Entwurfsverfasser/in eindeutig benennen.
 */
aufgestellt_von: string, };
export type Druckposition = { titel: string, erste_seite: number, seiten: number, };
export type NachweisAnsicht = { stand: Nachweisstand, aktuell: boolean, freigabe: Freigabe | null, };
