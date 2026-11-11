// Automatisch erzeugt: cargo run -p core-lib --example vertraege. Nicht von Hand ändern.
import type { BaugrundstuecksPlan, BaugrundstueckBerechnung, FlaechenAbzug, Planungsrecht } from '../api';
export type Friststufe = "ueberfaellig" | "diese_woche" | "monat" | "spaeter";
export type CockpitTermin = { projekt_id: ProjectId, projekt: string,
/**
 * Verfahrensbezeichnung, „Aufgabe“, „Projekttagebuch“ oder „Fachplaner“.
 */
bezug: string,
/**
 * Art eines Verfahrenstermins; `None` bei Aufgaben und Wiedervorlagen des
 * Projekttagebuchs.
 */
art: Terminart | null, art_bezeichnung: string, titel: string, datum: string,
/**
 * Beginn der Wiedervorlage (Datum abzüglich Vorlauf).
 */
faellig_ab: string, verantwortlich: string | null, hinweis: string, stufe: Friststufe,
/**
 * Sprungziel in der Projektakte.
 */
ziel: UebersichtsZiel, };
export type CockpitVerfahren = { id: string, bezeichnung: string, art: Verfahrensart, status: Verfahrensstatus, status_bezeichnung: string, aktenzeichen: string, behoerde: string, verantwortlich: string | null,
/**
 * Datum des jüngsten Versands oder Eingangs eines Abgabestands bzw. einer
 * externen Einreichung.
 */
letzte_abgabe: string | null,
/**
 * Forderungspunkte, die weder bestätigt noch entfallen sind.
 */
offene_punkte: number,
/**
 * Jüngster nicht ersetzter Bescheid.
 */
letzter_bescheid: CockpitBescheid | null,
/**
 * Abgelehnt und noch kein Folgeverfahren angelegt: Handlungsoptionen im
 * Register „Lage“ der Akte prüfen.
 */
ablehnung_offen: boolean, };
export type CockpitBescheid = { entscheidung: Entscheidung, datum: string, };
export type CockpitProjekt = { projekt_id: ProjectId, name: string, bundesland: Bundesland, verfahren: Array<CockpitVerfahren>, naechste_frist: string | null, ueberfaellig: number, offene_punkte: number, };
export type CockpitFehler = { projekt_id: ProjectId, projekt: string, meldung: string, };
export type Buerocockpit = { stichtag: string,
/**
 * Laufende Termine aller Projekte, überfällige zuerst, dann nach Datum.
 */
termine: Array<CockpitTermin>,
/**
 * Projekte nach Friststufe (überfällig, nächste Frist, Name).
 */
projekte: Array<CockpitProjekt>, fehler: Array<CockpitFehler>, };
export type Projektstufe = "vorbereitung" | "eingereicht" | "abgelehnt" | "genehmigt" | "im_bau" | "fertiggestellt" | "abgeschlossen";
export type Projektstand = { projekt_id: ProjectId, stufe: Projektstufe, bezeichnung: string,
/**
 * Datum, an dem die Stufe erreicht wurde, soweit die Akte es nennt.
 */
seit: string | null,
/**
 * Stufe aus den Verfahrensakten, ohne den Abschluss durch das Büro.
 */
verfahrensstufe: Projektstufe, };
export type Aboumfang = "alle" | "meine";
export type Aboauskunft = { aktiv: boolean, umfang: Aboumfang | null, erstellt_am: string | null, };
export type Aboschluessel = { umfang: Aboumfang,
/**
 * Pfad relativ zur Serveradresse, z. B. `/kalender/<benutzer>/<schluessel>.ics`.
 */
pfad: string, };
export type Planart = "lageplan" | "grundriss" | "schnitt" | "ansicht" | "detail" | "entwaesserung" | "freiflaechen" | "brandschutz" | "sonstiges";
export type Plannummernschema = { muster: string, };
export type Planrevision = {
/**
 * Büroindex, z. B. „a“, „b“ oder „01“.
 */
index: string, datum: string,
/**
 * Was sich gegenüber dem vorigen Index geändert hat.
 */
aenderung: string,
/**
 * Dokumentversion im DMS, die diesen Index trägt.
 */
version: number, };
export type Planversand = { id: string, empfaenger: string, datum: string,
/**
 * Weitergegebener Büroindex.
 */
index: string,
/**
 * E-Mail, Post, Plattform, persönlich …
 */
weg: string, bemerkung: string, };
export type Planeintrag = { dokument_id: DocumentId, planart: Planart,
/**
 * Geschoss oder Bereich, z. B. „EG“, „OG1“, „Nord“; darf leer sein.
 */
geschoss: string,
/**
 * Laufende Nummer, ab 1.
 */
nummer: number, titel: string,
/**
 * Maßstab als Nenner (100 für 1:100); `None` ohne Maßstab.
 */
massstab: number | null,
/**
 * Vom Core beim Speichern gesetzt.
 */
plannummer: string,
/**
 * Aufsteigend nach Datum.
 */
revisionen: Array<Planrevision>, verteiler: Array<Planversand>, };
export type Behoerdenstand = { verfahren: string, bezeichnung: string, planindex: string | null, status: Einreichungsstatus, };
export type Planzeile = { eintrag: Planeintrag, revision: number, dateiname: string,
/**
 * Neueste Version im DMS; `None`, wenn das Dokument entfernt wurde.
 */
aktuelle_version: number | null,
/**
 * Neueste Planrevision des Büros.
 */
index: Planrevision | null,
/**
 * Im DMS liegt eine neuere Version als die des letzten Index.
 */
ohne_index: boolean, behoerde: Array<Behoerdenstand>,
/**
 * Erfasste Versendungen und Abgaben an die Behörde, neueste zuerst.
 */
versand: Array<Planversand>, };
export type PlanOhneEintrag = { dokument_id: DocumentId, dateiname: string, behoerde: Array<Behoerdenstand>, };
export type Planliste = { schema: Plannummernschema, kennzeichen: string, zeilen: Array<Planzeile>,
/**
 * Bei der Behörde eingereichte Pläne, die die Planliste noch nicht führt.
 */
ohne_eintrag: Array<PlanOhneEintrag>, };
export type Blattformat = "a4" | "a3" | "a2" | "a1" | "a0";
export type Layoutwunsch = {
/**
 * Eigene Zeichnung (Planaufmaß ohne Unterlage).
 */
aufmass_id: string, revision: number, format: Blattformat, hochformat: boolean,
/**
 * Maßstabszahl, z. B. 100 für 1 : 100.
 */
massstab: number,
/**
 * Nordpfeil: Richtung Norden in Grad gegen den Uhrzeigersinn; `None` = ohne.
 */
nordwinkel_grad: number | null,
/**
 * Planinhalt im Plankopf; leer = Titel aus der Planliste bzw. Name der Zeichnung.
 */
planinhalt: string,
/**
 * Plan der Planliste, dessen Nummer und Index der Plankopf trägt; mit
 * `ablegen` wird die Ausgabe neue Version dieses Dokuments.
 */
dokument_id: DocumentId | null, legende: boolean, nur_sichtbare_ebenen: boolean,
/**
 * Ausgabe zusätzlich im Projekt ablegen.
 */
ablegen: boolean, };
export type Stempelwunsch = { dokument_id: DocumentId, version: number, seite: number, };
export type Wolkenwunsch = { dokument_id: DocumentId, version: number, seite: number,
/**
 * Bereiche als [x, y, Breite, Höhe] in Seitenpunkten (links oben, y nach unten).
 */
bereiche: Array<[number, number, number, number]>,
/**
 * Index im Dreieck neben jeder Wolke, z. B. „b“.
 */
index: string, };
export type Plankopf = { bauherr: string, vorhaben: string, grundstueck: string, planinhalt: string, planverfasser: string, plannummer: string, index: string, massstab: string, datum: string, format: string, };
export type Laengeneinheit = "millimeter" | "zentimeter" | "dezimeter" | "meter" | "kilometer" | "zoll" | "fuss";
export type CadMarkierung = { id: string, bezeichnung: string, punkte: Array<[number, number]>, geschlossen: boolean, };
export type CadProjektaufmass = { dokument: Dokumentbezug, einheit: Laengeneinheit, markierungen: Array<CadMarkierung>, };
export type Plankalibrierung = { "art": "massstab", nenner: number, } | { "art": "strecke", von: [number, number], bis: [number, number], meter: number, } | { "art": "einheit", einheit: Laengeneinheit, };
export type Planebene = { id: string, bezeichnung: string, sichtbar: boolean, gesperrt: boolean,
/**
 * Stift, Linienart, Farbe und Schraffur der Ebene; Elemente können sie überschreiben.
 */
darstellung?: Darstellung, };
export type Darstellung = {
/**
 * Stiftstärke in mm auf dem Papier (0,1 … 2,5).
 */
stift_mm?: number, linienart?: Linienart,
/**
 * Farbe als `#rrggbb`.
 */
farbe?: string, schraffur?: Schraffur,
/**
 * Bauzustand nach BauVorlV/BauPrüfVO: Bestand grau, Abbruch gelb, Neubau rot
 * (gilt, wenn keine eigene Farbe gesetzt ist).
 */
bauzustand?: Bauzustand, };
export type Linienart = "volllinie" | "strichlinie" | "strichpunktlinie" | "punktlinie";
export type Schraffur = "keine" | "voll" | "diagonal" | "kreuz" | "mauerwerk" | "beton" | "daemmung" | "erdreich";
export type Bauzustand = "bestand" | "abbruch" | "neubau";
export type Oeffnung = { art: Oeffnungsart,
/**
 * Abstand des Öffnungsanfangs vom Wandanfang (m).
 */
lage_m: number, breite_m: number, hoehe_m?: number, bruestung_m?: number,
/**
 * Türanschlag: Drehflügel öffnet zur linken Wandseite (in Richtung von→bis gesehen).
 */
links: boolean, };
export type Oeffnungsart = "fenster" | "tuer";
export type Plansymbol = "baum_bestand" | "baum_neu" | "baum_faellung" | "stellplatz" | "zufahrt" | "hauseingang" | "grenzpunkt";
export type Plangeometrie = { "art": "polygon", punkte: Array<[number, number]>, } | { "art": "rechteck", von: [number, number], bis: [number, number], } | { "art": "kreis", mitte: [number, number], rand: [number, number], } | { "art": "strecke", von: [number, number], bis: [number, number], } | { "art": "hoehenpunkt", punkt: [number, number], } | { "art": "flaeche", polygone: Array<Planpolygon>, };
export type Planpolygon = { aussen: Array<[number, number]>, innen?: Array<Array<[number, number]>>, };
export type Flaechenoperation = "vereinen" | "abziehen" | "schneiden" | "teilen";
export type Markierungspruefung = { id: string, wert: Planmesswert | null, fehler: string | null, };
export type Planpruefung = { markierungen: Array<Markierungspruefung>, };
export type Planexportwunsch = { aufmass_id: string, revision: number, format: Planexportformat, nur_sichtbare_ebenen: boolean, };
export type Planexportformat = "pdf" | "csv";
export type Planbedeutung = "unbestimmt" | "flurstueck" | "baufenster" | "vorhabenumriss" | "private_gruenflaeche" | "oeffentliche_gruenflaeche" | "verkehrsflaeche" | "ausserhalb_bauland" | "sonstiger_abzug" | "zufahrt" | "stellplatz" | "garage" | "nebenanlage" | "terrasse" | "unterirdisch" | "versiegelung" | "geschoss_aussenkante" | "gebaeudegrundflaeche" | "raum" | "aufenthaltsraum" | "nuf" | "tf" | "vf" | "kgf" | "aussenwand" | "gelaende" | "traufe" | "first" | "dach" | "nullpunkt" | "fussboden" | "decke" | "fundamentsohle" | "hoehenmass" | "abstandsflaeche" | "attika" | "wandteilgrenze" | "gaube" | "abgrabung";
export type Planmarkierung = { id: string, ebene_id: string, bezeichnung: string, bedeutung: Planbedeutung, geometrie: Plangeometrie, grundstueck_id: string | null, gebaeude_id: string | null, geschoss_id: string | null,
/**
 * Ausschnitt, in dem die Markierung gezeichnet wurde; fehlt bei Altdaten.
 */
ausschnitt_id?: string,
/**
 * Raummarkierung, deren Höhenzone (Teilfläche mit abweichender lichter
 * Höhe, z. B. unter einer Dachschräge) diese Fläche ist. Das Raumbuch
 * übernimmt sie als Höhenzone des Raums, nicht als eigenen Raum.
 */
hoehenzone_von?: string,
/**
 * Markierung desselben Aufmaßes, deren Kontur diese Markierung teilt:
 * eine Kontur, mehrere Bedeutungen (z. B. Gebäudegrundfläche und
 * Außenkante EG). Die Geometrie folgt beim Speichern der Quelle, siehe
 * [`Planaufmass::konturen_angleichen`]; Kennung, Bedeutung und Zuordnung
 * bleiben eigene, damit Übernahmen und Nachweise unverändert adressieren.
 */
kontur_von?: string,
/**
 * Flurstück (ID aus den Grundstücksinformationen), dessen Grenze diese
 * Markierung mit Bedeutung „Flurstück“ zeigt. Grundlage der Übernahme in
 * die Flurstücksfläche; ohne Zuordnung bleibt die Fläche eine Kontrolle.
 */
flurstueck_id?: string, };
export type Planhoehenbezug = { punkt: [number, number], hoehe_m: number,
/**
 * Zum Beispiel „m ü. NHN“ oder „lokal, OKFF EG = 0“.
 */
system: string, };
export type Ausschnittart = "grundriss" | "lageplan" | "schnitt" | "ansicht" | "detail";
export type Planbezugslinie = { von: [number, number], bis: [number, number], hoehe_m: number, system: string, };
export type Planausschnitt = { id: string, bezeichnung: string, art: Ausschnittart, von: [number, number], bis: [number, number],
/**
 * Stabile Zeichenebene; Altbestände verwenden weiterhin die Namenszuordnung.
 */
ebene_id?: string, kalibrierung?: Plankalibrierung, bezugslinie?: Planbezugslinie, };
export type Planaufmass = { bezeichnung: string,
/**
 * Unterlage (PDF-Seite oder DXF). Fehlt sie, ist das Aufmaß eine eigene
 * Zeichnung: Seitenpunkte sind Meter (y nach unten), Unterlagen liegen
 * als `hintergruende` eingepasst darunter.
 */
dokument?: Dokumentbezug,
/**
 * Einsbasiert. Mehrere Aufmaße derselben Seite dürfen verschiedene Maßstäbe haben.
 */
seite: number, kalibrierung: Plankalibrierung | null, hoehenbezug: Planhoehenbezug | null, ebenen: Array<Planebene>, markierungen: Array<Planmarkierung>,
/**
 * Optionale Ausschnitte; ohne sie gelten Maßstab und Höhenbezug der Seite.
 */
ausschnitte?: Array<Planausschnitt>,
/**
 * Nur eigene Zeichnungen: eingepasste Unterlagen als Hintergrund.
 */
hintergruende?: Array<Planhintergrund>,
/**
 * Zeichenelemente ohne Messwert (Wand, Maßkette, Text).
 */
elemente?: Array<Zeichenelement>, };
export type Planhintergrund = { id: string, dokument: Dokumentbezug,
/**
 * Einsbasiert; DXF immer 1.
 */
seite: number, ursprung: [number, number], lage: [number, number], drehung_grad: number, meter_pro_punkt: number,
/**
 * 0 … 1.
 */
deckkraft: number, sichtbar: boolean,
/**
 * IFC-Hintergrund: Geschoss (GlobalId), dessen Grundriss-Schnitt gezeigt
 * wird; Seitenpunkte sind dann Meter des Modells (wie eine DXF in Metern).
 */
geschoss?: string, };
export type Zeichenform = { "art": "wand", von: [number, number], bis: [number, number], dicke_m: number,
/**
 * Fenster und Türen, entlang der Achse ab `von`.
 */
oeffnungen?: Array<Oeffnung>, } | { "art": "masskette", von: [number, number], bis: [number, number],
/**
 * Abstand der Maßlinie von der Strecke in Seitenpunkten; positiv = rechts
 * der Richtung von→bis (bei y nach unten also im Uhrzeigersinn gedreht).
 */
abstand: number, } | { "art": "text", punkt: [number, number], inhalt: string,
/**
 * Schrifthöhe in Seitenpunkten.
 */
hoehe: number, drehung_grad: number, } | { "art": "symbol", punkt: [number, number], symbol: Plansymbol,
/**
 * Größe in Seitenpunkten (bei eigenen Zeichnungen Meter).
 */
groesse: number, drehung_grad: number, } | { "art": "raumstempel", punkt: [number, number], raumbuch_id: string, raum_id: string,
/**
 * Schrifthöhe in Seitenpunkten.
 */
hoehe: number, };
export type Zeichenelement = { id: string, ebene_id: string,
/**
 * Abweichende Darstellung des Elements; fehlende Felder erben von der Ebene.
 */
darstellung?: Darstellung, } & ({ "art": "wand", von: [number, number], bis: [number, number], dicke_m: number,
/**
 * Fenster und Türen, entlang der Achse ab `von`.
 */
oeffnungen?: Array<Oeffnung>, } | { "art": "masskette", von: [number, number], bis: [number, number],
/**
 * Abstand der Maßlinie von der Strecke in Seitenpunkten; positiv = rechts
 * der Richtung von→bis (bei y nach unten also im Uhrzeigersinn gedreht).
 */
abstand: number, } | { "art": "text", punkt: [number, number], inhalt: string,
/**
 * Schrifthöhe in Seitenpunkten.
 */
hoehe: number, drehung_grad: number, } | { "art": "symbol", punkt: [number, number], symbol: Plansymbol,
/**
 * Größe in Seitenpunkten (bei eigenen Zeichnungen Meter).
 */
groesse: number, drehung_grad: number, } | { "art": "raumstempel", punkt: [number, number], raumbuch_id: string, raum_id: string,
/**
 * Schrifthöhe in Seitenpunkten.
 */
hoehe: number, });
export type Planmesswert = { id: string, laenge_m: number | null, flaeche_m2: number | null, horizontale_m: number | null, hoehe_m: number | null, neigung_grad: number | null, offen: Array<string>, };
export type Planmessgroesse = "flaeche" | "laenge" | "horizontale" | "hoehe" | "neigung";
export type Planbindung = { aufmass_id: string, revision: number, markierung_id: string, groesse: Planmessgroesse,
/**
 * Tabelle.Zeilen-ID.Feld; niemals Arraypositionen.
 */
ziel: string, };
export type Aufmassstand = { aufmass_id: string, revision: number, };
export type Zeilenzuordnung = { markierung_id: string,
/**
 * Schlüssel der Zeile ([`Abgleichskandidat::zeile`]), die die gezeichnete
 * Fläche übernimmt; leer legt eine neue Zeile an.
 */
zeile?: string, };
export type Abgleichskandidat = {
/**
 * Zeilenkennung oder, für Altzeilen ohne Kennung, `#<Position>` in ihrer
 * Liste. Gilt nur für den Entwurf, aus dem die Frage entstand.
 */
zeile: string, bezeichnung: string, flaeche_m2: number, };
export type Abgleichsfrage = { markierung_id: string, markierung: string,
/**
 * Ziel der Zeile, etwa „Flurstück Musterdorf 12 · Verkehrsfläche“.
 */
ziel: string,
/**
 * Gezeichnete Fläche; fehlt, wenn die Markierung noch nicht messbar ist.
 */
flaeche_m2?: number, kandidaten: Array<Abgleichskandidat>,
/**
 * Vorbelegung: die Handzeile mit der geringsten Flächendifferenz.
 */
vorschlag?: string, };
export type Doppelerfassung = {
/**
 * `abzug` oder `bauteil`.
 */
bereich: string,
/**
 * Flurstück (Abzug) bzw. Schlüssel des Baugrundstücks (Bauteil).
 */
bezug: string,
/**
 * Kennung der gezeichneten Zeile.
 */
gebunden: string,
/**
 * Position der Handzeile in ihrer Liste.
 */
hand: number, text: string, };
export type Planquellenhinweis = {
/**
 * Bereich für den Sprung: `flurstuecke`, `grundstueck`, `gebaeude`,
 * `raumbuch`, `abstand` oder das Modul einer Fachberechnung.
 */
bereich: string, objekt?: string, titel: string, text: string, };
export type CadUebernahme = { uebernommen: Array<string>, vorhanden: number, uebersprungen: Array<string>, };
export type Planziel = { ziel: string, titel: string, groesse: Planmessgroesse, };
export type Aufgabengeometrie = "flaeche" | "hoehenpunkt" | "strecke";
export type Planaufgabe = {
/**
 * `flurstueck`, `abzug`, `bauteil`, `gebaeude`, `abstand`, `berechnung` oder `raumbuch`.
 */
verbraucher: string,
/**
 * Flurstücks-ID, Zeilenkennung des Abzugs bzw. Bauteils, Gebäude-ID, ID des
 * Abstandsplans, der Fachberechnung oder des Raumbuchs.
 */
objekt: string,
/**
 * Modul einer Fachberechnung (`din277`, `wohnflaechen`, …), sonst `None`.
 */
modul?: string, objekt_name: string,
/**
 * Anzeigegruppe, z. B. „Lagerhalle · OG“ oder „Südwand“.
 */
gruppe: string, ziel: string, titel: string, groesse: Planmessgroesse, bedeutung: Planbedeutung, geometrie: Aufgabengeometrie, gebaeude_id: string | null, geschoss_id: string | null, grundstueck_id: string | null,
/**
 * Flurstück, dem eine neue Markierung zugeordnet wird (Verbraucher `flurstueck`).
 */
flurstueck_id: string | null,
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
markierung: string | null, hinweis: string | null,
/**
 * Ein Nachweis oder eine Berechnung braucht den Wert. Freiwillige Werte
 * (Höhen eines Gebäudes ohne Verbraucher) zählen nicht als offen.
 */
pflicht: boolean,
/**
 * Planarten, in denen sich der Wert zeichnen lässt; die Aufgabenliste
 * zeigt zuerst, was zum geöffneten Ausschnitt passt.
 */
planarten: Array<Ausschnittart>, };
export type Werkzeugvorlage = { id: string,
/**
 * Anzeigegruppe, z. B. „Grundstück“ oder „DIN 277“.
 */
gruppe: string, titel: string, bedeutung: Planbedeutung, geometrie: Aufgabengeometrie,
/**
 * Zeichenebene, die beim ersten Gebrauch angelegt wird.
 */
ebene: string, hinweis: string, };
export type Modellakte = { bezeichnung: string,
/**
 * IFC-Datei (`.ifc` oder `.ifczip`) im DMS.
 */
dokument: Dokumentbezug,
/**
 * Letzte Übernahme in Gebäude, Zeichnungen und Raumbücher.
 */
uebernahme?: Modelluebernahme,
/**
 * Nacharbeit: Änderungen über die GlobalId, in dieser Reihenfolge angewendet.
 */
nacharbeit?: Array<Modellaenderung>, };
export type Georeferenz = {
/**
 * Koordinatenreferenzsystem, z. B. `EPSG:25832`.
 */
crs: string | null, ostwert: number | null, nordwert: number | null, hoehe: number | null,
/**
 * Drehung der Modell-x-Achse gegen Ost, Grad gegen den Uhrzeigersinn.
 */
drehung_grad: number | null,
/**
 * IFC2X3: geographische Lage des Grundstücks in Dezimalgrad.
 */
breite: number | null, laenge: number | null, };
export type Mengenart = "laenge" | "flaeche" | "volumen" | "anzahl" | "gewicht" | "zeit";
export type Modelleigenschaft = { satz: string, name: string,
/**
 * Anzeigewert (z. B. „Ja“, „2,50 m“, „Wohnen“).
 */
wert: string,
/**
 * Zahlenwert in SI-Einheit (m, m², m³), sofern es eine Zahl ist.
 */
zahl: number | null,
/**
 * Vom Typ (`IfcRelDefinesByType`) geerbt statt am Element gesetzt.
 */
vom_typ: boolean, };
export type Modellmenge = { satz: string, name: string, art: Mengenart, wert: number, };
export type Modellgeometrie = {
/**
 * Senkrechte Projektion (Grundriss) aller Körperteile.
 */
grundriss: Array<Planpolygon>, grundflaeche_m2: number, unten_m: number, oben_m: number, volumen_m3: number | null,
/**
 * Boolesche Operationen oder schräge Körper angenähert.
 */
naeherung: boolean, };
export type Strukturelement = { global_id: string, name: string, langname: string | null,
/**
 * Übergeordnetes Strukturelement (Grundstück → Gebäude → Geschoss).
 */
teil_von: string | null,
/**
 * Geschosse: Höhe der Geschossebene über dem Projektnullpunkt (m).
 */
hoehe_m: number | null, eigenschaften: Array<Modelleigenschaft>, mengen: Array<Modellmenge>, geometrie: Modellgeometrie | null, };
export type Modellelement = { global_id: string,
/**
 * Entitätsname in Großbuchstaben, z. B. `IFCWALL`, `IFCSPACE`.
 */
klasse: string, name: string, langname: string | null, objekttyp: string | null,
/**
 * `PredefinedType` (z. B. `FLOOR`, `ROOF`) oder Innen/Außen bei IFC2X3-Räumen.
 */
vordefiniert: string | null,
/**
 * Geschoss (GlobalId), dem das Element über die räumliche Struktur angehört.
 */
geschoss: string | null,
/**
 * Gebäude (GlobalId) des Elements, auch ohne Geschoss.
 */
gebaeude: string | null,
/**
 * Übergeordnetes Element (z. B. Dach → Dachplatten) oder Raum.
 */
teil_von: string | null, eigenschaften: Array<Modelleigenschaft>, mengen: Array<Modellmenge>, geometrie: Modellgeometrie | null,
/**
 * Nicht ausgewertete Darstellungsarten der Geometrie.
 */
geometrie_offen: Array<string>, };
export type Klassenzahl = { klasse: string, titel: string, anzahl: number, };
export type Modellbericht = { klassen: Array<Klassenzahl>, elemente: number, mit_geometrie: number, ohne_geometrie: number, naeherungen: number, raeume: number, raeume_mit_flaechenmenge: number,
/**
 * Darstellungsarten, die nicht ausgewertet wurden, mit Anzahl Elemente.
 */
offene_darstellungen: Array<Klassenzahl>, hinweise: Array<string>, };
export type Modellauszug = { rechenstand: string,
/**
 * `IFC2X3`, `IFC4`, `IFC4X3_ADD2` …
 */
schema: string,
/**
 * Erzeugende Software laut Dateikopf.
 */
system: string | null, projekt: string | null,
/**
 * Längeneinheit der Datei, z. B. „Millimeter“.
 */
einheit: string, meter_je_einheit: number, georeferenz: Georeferenz | null, grundstuecke: Array<Strukturelement>, gebaeude: Array<Strukturelement>,
/**
 * Geschosse nach Höhe sortiert.
 */
geschosse: Array<Strukturelement>, elemente: Array<Modellelement>, bericht: Modellbericht, };
export type Grundrissteil = { global_id: string, klasse: string, titel: string, name: string,
/**
 * Meter im Weltsystem des Modells (y nach Modell-y, nicht gespiegelt).
 */
polygone: Array<Planpolygon>, raum: boolean,
/**
 * Projektion statt Schnitt (Flächenmodell oder schräger Körper).
 */
projektion: boolean, };
export type Modellgrundriss = { geschoss: string, schnitthoehe_m: number, teile: Array<Grundrissteil>,
/**
 * Umfang aller Elemente des Modells (nicht nur dieses Geschosses), damit
 * die Grundrisse aller Geschosse deckungsgleich liegen.
 */
min: [number, number], max: [number, number], };
export type Modellgeschoss = { global_id: string, name: string, hoehe_m: number | null, };
export type Modelluebernahme = { gebaeude_id: string,
/**
 * Dokumentversion, aus der übernommen wurde.
 */
dokument: Dokumentbezug, geschosse: Array<Geschossuebernahme>, };
export type Geschossuebernahme = {
/**
 * GlobalId des IFC-Geschosses.
 */
global_id: string,
/**
 * Geschoss im Gebäude.
 */
geschoss_id: string,
/**
 * Eigene Zeichnung (Fachobjekt `planaufmass`) des Geschosses.
 */
aufmass_id: string,
/**
 * Raumbuch des Geschosses, sofern mit übernommen.
 */
raumbuch_id?: string,
/**
 * Übernommene Markierungen und ihre Herkunft im Modell.
 */
markierungen: Array<Modellbezug>, };
export type Modellbezug = { markierung_id: string, global_id: string, };
export type Modelluebernahmewunsch = { akte_id: string, revision: number,
/**
 * Vorhandenes Gebäude, in das übernommen wird; ohne Angabe das Gebäude
 * der letzten Übernahme oder ein neues.
 */
gebaeude_id: string | null,
/**
 * Je Geschoss ein Raumbuch aus den Räumen füllen.
 */
raumbuch: boolean, };
export type Modelluebernahmeergebnis = { akte: Fachstand, gebaeude: Fachstand, zeichnungen: Array<Fachstand>, raumbuecher: Array<Fachstand>, hinweise: Array<string>, };
export type Befundstufe = "fehler" | "warnung" | "hinweis";
export type Befundaktion = "klassifizieren" | "geschoss_zuordnen" | "eigenschaft_setzen" | "raum_bilden" | "benennen";
export type Befund = {
/**
 * Regelkennzeichen, z. B. `proxy` oder `IDS:W-AUSSENWAND`.
 */
regel: string, stufe: Befundstufe, titel: string, text: string,
/**
 * Betroffene Elemente (GlobalIds), leer bei Modellbefunden.
 */
elemente: Array<string>, aktion: Befundaktion | null,
/**
 * Bei IDS-Befunden: fehlende Eigenschaft (`Pset.Name`) für die Nacharbeit.
 */
eigenschaft: string | null, };
export type Modellpruefung = {
/**
 * Titel des IDS, gegen das geprüft wurde.
 */
regelwerk: string, befunde: Array<Befund>, fehler: number, warnungen: number, hinweise: number,
/**
 * IDS-Teile, die nicht ausgewertet werden konnten.
 */
nicht_geprueft: Array<string>, };
export type Eigenschaftswert = { "typ": "text", wert: string, } | { "typ": "zahl", wert: number, } | { "typ": "logisch", wert: boolean, };
export type Modellaenderung = { "art": "klassifizieren", global_id: string, klasse: string, vordefiniert: string | null, } | { "art": "geschoss_zuordnen", global_id: string, geschoss: string, } | { "art": "eigenschaft_setzen", global_id: string, satz: string, name: string, wert: Eigenschaftswert, } | { "art": "benennen", global_id: string, name: string, langname: string | null, } | { "art": "raum_bilden", global_id: string, geschoss: string, name: string, langname: string | null, umriss: Array<[number, number]>,
/**
 * Lichte Höhe; ohne Angabe bis zur nächsten Geschossebene.
 */
hoehe_m: number | null, } | { "art": "ausblenden", global_id: string, } | { "art": "hoehe_setzen", global_id: string, unten_m: number | null, oben_m: number | null, } | { "art": "umriss_setzen", global_id: string, umriss: Array<[number, number]>, };
export type Raumvorschlag = {
/**
 * Neue GlobalId für den Raum, falls er gebildet wird.
 */
global_id: string,
/**
 * Umriss in Modellkoordinaten (m).
 */
umriss: Array<[number, number]>, flaeche_m2: number,
/**
 * Vorhandener Raum, der diesen Umriss schon abdeckt.
 */
vorhanden: string | null, };
export type Nacharbeitsstand = "wirksam" | "verwaist" | "ueberholt";
export type Elementaenderung = { global_id: string, titel: string,
/**
 * Was sich geändert hat, z. B. „Klasse Proxy → Wand“, „Fläche +1,20 m²“.
 */
aenderungen: Array<string>, };
export type Elementverweis = { global_id: string, titel: string, };
export type Modellabgleich = { alt_schema: string, neu_schema: string, neu: Array<Elementverweis>, entfernt: Array<Elementverweis>, geaendert: Array<Elementaenderung>, unveraendert: number,
/**
 * Geschosse: hinzugekommen, entfallen, Höhe geändert (Texte).
 */
geschosse: Array<string>,
/**
 * Stand jeder Nacharbeit, in der Reihenfolge der Liste.
 */
nacharbeit: Array<Nacharbeitsstand>, hinweise: Array<string>, };
export type Reimportwunsch = { akte_id: string, revision: number,
/**
 * Neue IFC-Datei (neue Version derselben Datei oder eine andere).
 */
dokument: Dokumentbezug,
/**
 * Überholte Nacharbeit (die neue Version hat den Zustand selbst) streichen.
 */
ueberholte_entfernen: boolean,
/**
 * Verwaiste Nacharbeit (Element fehlt in der neuen Version) streichen.
 */
verwaiste_entfernen: boolean, };
export type Geschossauswertung = { global_id: string, name: string,
/**
 * Geschossebene über dem Projektnullpunkt (m).
 */
ebene_m: number,
/**
 * Rechnerische Höhe für den Rauminhalt: bis zur nächsten Geschossebene,
 * im obersten Geschoss bis zur Oberkante der Bauteile, im untersten ab
 * der Unterkante der Bodenplatte.
 */
hoehe_m: number,
/**
 * Liegt auf oder über der Ebene des Erdgeschosses.
 */
oberirdisch: boolean,
/**
 * Brutto-Grundfläche (Außenkante von Wänden, Stützen und Decken).
 */
bgf_m2: number, bri_m3: number,
/**
 * Netto-Raumfläche (Summe der Raumgeometrie).
 */
nrf_m2: number,
/**
 * Summe `NetFloorArea` der Räume, soweit eingetragen.
 */
nrf_menge_m2: number | null, raeume: number, aussenkante: Array<Planpolygon>, };
export type Nutzungsflaeche = {
/**
 * Langname des Raums (Nutzung) oder „ohne Nutzung“.
 */
nutzung: string, raeume: number, flaeche_m2: number, };
export type Modellhoehen = {
/**
 * Ebene des Erdgeschosses über dem Projektnullpunkt (m).
 */
bezug_m: number,
/**
 * Bezugshöhe in m über NHN, wenn das Modell georeferenziert ist.
 */
bezug_nhn_m: number | null,
/**
 * Höchster Punkt der Dachbauteile.
 */
first_m: number | null,
/**
 * Flachdach: Oberkante Dach; geneigtes Dach: tiefste Unterkante der Dachbauteile.
 */
traufe_m: number | null,
/**
 * Höchster Punkt aller Bauteile (auch Schornsteine, Aufbauten).
 */
oberkante_m: number | null, flachdach: boolean, };
export type Bezugsflaeche = { grundstueck_id: string, bezeichnung: string, flaeche_m2: number, };
export type Modellauswertung = { rechenstand: string,
/**
 * Gebäudegrundfläche: Projektion der Außenkanten aller oberirdischen Geschosse.
 */
grundflaeche_m2: number, grundflaeche: Array<Planpolygon>, bgf_m2: number, bgf_oberirdisch_m2: number, bri_m3: number, nrf_m2: number, geschosse: Array<Geschossauswertung>, nutzungen: Array<Nutzungsflaeche>, hoehen: Modellhoehen, grundstueck: Bezugsflaeche | null,
/**
 * Gebäudegrundfläche / Bezugsfläche (rechnerisch).
 */
grz: number | null,
/**
 * BGF der oberirdischen Geschosse / Bezugsfläche (rechnerisch).
 */
gfz: number | null, hinweise: Array<string>, };
export type Exportbericht = { dateiname: string, schema: string,
/**
 * Elemente der geschriebenen Datei (wie der Auszug sie zählt).
 */
elemente: number, geaendert: number, neu: number, entfernt: number, hinweise: Array<string>,
/**
 * Abweichungen der wieder eingelesenen Datei vom erwarteten Modell.
 */
abweichungen: Array<string>, };
export type Entwurfszuordnung = {
/**
 * Geschoss des Gebäudes (Fachobjekt `gebaeude`).
 */
geschoss_id: string,
/**
 * Eigene Zeichnung (Planaufmaß ohne Unterlage, Meter) dieses Geschosses.
 */
zeichnung_id: string, };
export type Exportwunsch = { "art": "nacharbeit", akte_id: string, } | { "art": "entwurf", gebaeude_id: string, geschosse: Array<Entwurfszuordnung>, };
export type Exportergebnis = {
/**
 * Abgelegte IFC-Datei (neue Version, wenn es den Namen schon gibt).
 */
dokument: Dokumentbezug, bericht: Exportbericht, };
export type Bauantragsanlage = {
/**
 * `false`: Die Datei lag dem Bauantrag schon bei.
 */
neu: boolean,
/**
 * Anlagenart (leer, wenn die Anlagenliste fehlt oder 2010 nicht kennt).
 */
anlageart: string,
/**
 * Zahl der Anlagen des Bauantrags.
 */
anlagen: number, };
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
umriss?: UmrissSeite,
/**
 * Breite aus der verknüpften Ansicht statt Lage im Lageplan: Die Wand
 * liegt in einem eigenen Bezug von (0, 0) nach (Breite, 0); die Lage
 * prüft die Software nicht, maßgebend ist `grenzabstand_m`.
 */
breite_aus_ansicht?: boolean,
/**
 * Wandbreite in der verknüpften Ansicht (Linie „Abstandsfläche“), für
 * den Längenabgleich mit dem Lageplan.
 */
ansicht_laenge_m?: number,
/**
 * Abstand der Wand zur Grundstücksgrenze als Angabe des Planers; nur
 * maßgebend, wenn die Wand keine Grenze im Plan hat.
 */
grenzabstand_m?: number,
/**
 * Felder, die der Höhenabgriff in der verknüpften Ansicht setzt
 * (`gelaende`, `oberkante`, `wandteilgrenzen`, `dachhoehe_m`,
 * `dachneigung_grad`, `dachneigung_b_grad`, `gauben_laenge_m`); sie
 * sind im Plan gesperrt, solange die Ansicht verknüpft ist.
 */
ansicht_felder?: Array<string>,
/**
 * Offene Punkte aus dem Höhenabgriff in der Ansicht.
 */
ansicht_hinweise?: Array<string>, };
export type AbstandsPlan = { planbindungen?: Array<Planbindung>, bezeichnung: string, regeln: AbstandsRegeln, recht: AbstandsRecht, hoehenbezug: string, koordinatenbezug: string, grundstuecke: Array<AbstandsGrundstueck>, waende: Array<AbstandsWand>, flaechen: Array<AbstandsFlaeche>, quellen: Array<Dokumentbezug>, umfang_geprueft: boolean, umfang_vermerk: string, umrisse?: Array<AbstandsUmriss>,
/**
 * Gewählte Regeldatei (Land und Fassung); fehlt in Plänen vor der
 * Fassungswahl, die dann nur mit der Regelkopie `regeln` rechnen.
 */
regelwerk?: Regelwerkbezug, };
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
export type AbstandslinienUebernahme = { plan: AbstandsPlan, hinweise: Array<string>, };
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
export type Fassungsangabe = { gesetz: string, paragraf: string,
/**
 * Tag des Inkrafttretens; fehlt bei Pilotansätzen ohne belegte Fassung.
 */
gueltig_ab: string | null,
/**
 * Letzter Geltungstag; fehlt, solange die Fassung gilt.
 */
gueltig_bis: string | null, fundstelle: string, uebergang: string, };
export type Rechtsquelle = { titel: string, url: string,
/**
 * Abrufdatum; fehlt, wenn die Quelle nicht online geprüft werden konnte.
 */
abgerufen: string | null,
/**
 * Beigelegte Datei relativ zum Länderordner.
 */
datei: string,
/**
 * SHA-256 der beigelegten Datei.
 */
sha256: string, };
export type Regelpruefung = {
/**
 * Nur das Büro setzt `true` (fachliche Abnahme), zusammen mit dem
 * Eintrag in `freigaben.json`.
 */
geprueft: boolean, geprueft_von: string, geprueft_am: string | null, vermerk: string, };
export type Absatztext = { absatz: number, text: string, };
export type Regelpruefpunkt = { absatz: number, frage: string, };
export type Beispielrechnung = { titel: string, gebiet: string, laenge_m: number,
/**
 * Geländehöhe an Wandanfang und Wandende.
 */
gelaende_m: [number, number],
/**
 * Wandoberkante an Wandanfang und Wandende.
 */
oberkante_m: [number, number], dachhoehe_m?: number, dachneigung_grad?: number, tiefenfall?: Tiefenfall, erwartet_h_m: number, erwartet_tiefe_m: number, };
export type Regeldatei = { schema: number, fassung: Fassungsangabe, quellen: Array<Rechtsquelle>, pruefung: Regelpruefung, wortlaut: Array<Absatztext>, pruefpunkte: Array<Regelpruefpunkt>, beispiele: Array<Beispielrechnung>, regeln: AbstandsRegeln, };
export type Regelwerk = {
/**
 * Pfad relativ zu `regelwerke/abstandsflaechen`, z. B. `nw/2024-01-01.json`.
 */
datei: string,
/**
 * SHA-256 des Dateiinhalts.
 */
pruefsumme: string, schema: number, fassung: Fassungsangabe, quellen: Array<Rechtsquelle>, pruefung: Regelpruefung, wortlaut: Array<Absatztext>, pruefpunkte: Array<Regelpruefpunkt>, beispiele: Array<Beispielrechnung>, regeln: AbstandsRegeln, };
export type Regelwerkbezug = {
/**
 * Pfad relativ zu `regelwerke/abstandsflaechen`, z. B. `nw/2024-01-01.json`.
 */
datei: string,
/**
 * SHA-256 der Regeldatei bei der Übernahme.
 */
pruefsumme: string,
/**
 * Tag, nach dem sich die Fassung richtet (in der Regel Antragseingang).
 */
stichtag: string,
/**
 * Woher der Stichtag stammt, z. B. „Eingang Bauantrag“.
 */
stichtag_art: string,
/**
 * Begründung, wenn die Fassung nicht am Stichtag gilt (z. B. Übergangsrecht).
 */
abweichung?: string,
/**
 * Bearbeitungsstand der Prüfpunkte der Regeldatei.
 */
pruefpunkte?: Array<Pruefpunktstand>, };
export type Pruefpunktstand = { absatz: number, erledigt: boolean, vermerk?: string, };
export type Regelwerkpruefung = {
/**
 * Die gewählte Regeldatei, sofern in dieser Programmversion enthalten.
 */
regelwerk: Regelwerk | null,
/**
 * Datei der am Stichtag geltenden Fassung.
 */
geltend: string | null,
/**
 * Rechenregeln des Plans entsprechen der Regeldatei.
 */
regeln_aktuell: boolean,
/**
 * Die Regeldatei hat sich seit der Übernahme geändert.
 */
datei_geaendert: boolean,
/**
 * Absätze, die die Rechenregeln anwenden (aus den Quellenangaben).
 */
angewandte_absaetze: Array<number>, offen: Array<string>, };
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
rolle?: FlurstuecksRolle,
/**
 * Im Aufmaß gezeichnete Flurstücksgrenze (Flurkarte oder Lageplan):
 * Bindung an eine Markierung mit Bedeutung „Flurstück“, die diesem
 * Flurstück zugeordnet ist (Ziel [`Self::planziel`]). Die Katasterfläche
 * [`Self::flaeche_m2`] bleibt der Rechenwert, solange nicht
 * [`Self::flaeche_aus_plan`] gesetzt ist; die Abweichung bleibt sichtbar.
 */
planbindung?: Planbindung,
/**
 * Gemessene Fläche der gebundenen Markierung in m² (auf 0,01 m² gerundet).
 * Setzt ausschließlich der Core bei der Übernahme (`planaufmass_flurstuecke`).
 */
planflaeche_m2?: number,
/**
 * Die gezeichnete Fläche gilt als Flurstücksfläche, weil (noch) kein
 * amtlicher Wert vorliegt; `flaeche_m2` wird dann aus `planflaeche_m2`
 * übernommen und ist nicht von Hand änderbar.
 */
flaeche_aus_plan?: boolean,
/**
 * Die Fläche wird im Aufmaß ermittelt; eine Katasterfläche liegt noch
 * nicht vor. `flaeche_m2` ist dann 0 und gilt als **fehlend**, nicht als
 * 0 m²: Baugrundstücke, GRZ/GFZ und die Projektfläche bleiben offen, bis
 * die gezeichnete Fläche übernommen ist (danach „Fläche aus Plan“).
 */
flaeche_offen?: boolean, };
export type FlurstuecksRolle = "vorhaben" | "nachbar" | "sonstiges";
export type Flaechenabweichung = "unauffaellig" | "warnung" | "fehler";
export type FlurstuecksAnsatz = { flurstueck_id: string, abzuege: Array<FlaechenAbzug>, abweichende_zuordnung: { kennung: string; grundlage: string } | null, };
export type BaugrundstuecksErmittlung = { ansaetze: Array<FlurstuecksAnsatz>, berechnungen: Array<BaugrundstueckBerechnung>,
/**
 * Bei Speicherung durch den Core gesetzt; keine zweite Kopie der Katasterflächen.
 */
quellen_hash: string | null, };
export type Baugebiet = "WS" | "WR" | "WA" | "WB" | "MD" | "MDW" | "MI" | "MU" | "MK" | "GE" | "GI" | "SO" | "SOF" | "SOS";
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
export type Rechtsgrundlage = "par30" | "par34" | "par35";
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
export type Forderungspunkt = { id: string, titel: string, grundlage: string, verantwortlich: string | null, frist: Frist | null, status: Punktstatus, antwort: string, dokumente: Array<Dokumentbezug>, abgaben: Array<string>, bestaetigung: string | null,
/**
 * Anforderung der Akte, die dieser Punkt nachfordert. Solange der Punkt
 * offen ist, steht der Vorgang nur hier; seine Antwortunterlagen
 * belegen auch die Anforderung.
 */
anforderung?: string, };
export type Nachforderung = { id: string, kommunikation: string, punkte: Array<Forderungspunkt>, };
export type Entscheidung = "genehmigt" | "teilweise_genehmigt" | "abgelehnt" | "sonstiges";
export type Nebenbestimmungsart = "auflage" | "bedingung" | "befristung" | "widerrufsvorbehalt" | "auflagenvorbehalt" | "hinweis";
export type Auflage = { id: string,
/**
 * Ältere Akten ohne diese Angabe lesen sie als Auflage.
 */
art: Nebenbestimmungsart, titel: string, ausloeser: string, verantwortlich: string | null, frist: Frist | null, erledigt: boolean, nachweise: Array<Dokumentbezug>,
/**
 * Wann die Pflicht erfüllt sein muss (vor Baubeginn …); ältere Einträge
 * lesen die Angabe leer.
 */
faelligkeit?: Faelligkeit, };
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
teilbaugenehmigung?: boolean,
/**
 * Entscheidungen über beantragte Abweichungen, Ausnahmen und Befreiungen.
 * Ältere Bescheide lesen die Angabe leer.
 */
abweichungsentscheidungen?: Array<Abweichungsentscheidung>, };
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
folgeschreiben?: string,
/**
 * Anhörung: Beanstandungen der Behörde, auf die die Stellungnahme
 * eingeht. Ältere Akten lesen die Angabe leer.
 */
gruende?: Array<Beanstandung>, };
export type Pruefverfahren = "regulaer" | "vereinfacht";
export type Erforderlichkeit = "ungeklaert" | "erforderlich" | "nicht_erforderlich";
export type Nachweisunterlage = { dokument: Dokumentbezug,
/**
 * Nachweisstände, die die Unterlage wiedergibt.
 */
staende: Array<string>, };
export type Anforderung = { id: string, titel: string, erforderlich: Erforderlichkeit, grund: string, quelle: string, dokumente: Array<Dokumentbezug>, freigegeben: boolean, verantwortlich: string | null,
/**
 * Bereiche der Projektakte, deren Nachweis die Anforderung belegt
 * (aus dem Katalog); leer bei Unterlagen ohne eigenen Nachweisbereich.
 */
nachweise?: Array<string>,
/**
 * Dokumentkategorien, deren Dateien die Anforderung üblicherweise
 * belegen (aus dem Katalog); Grundlage für Dateivorschläge.
 */
kategorien?: Array<string>,
/**
 * Aus den Nachweisständen der Projektakte erzeugte Unterlage dieser
 * Anforderung; fehlt, solange kein Nachweis übernommen wurde.
 */
nachweisunterlage?: Nachweisunterlage, };
export type Dokumentbeziehung = { id: string, vorher: Dokumentbezug, nachher: Dokumentbezug, grund: string, bezug: Bezug | null, };
export type Ereignisart = "angelegt" | "status" | "vollstaendigkeit" | "ruecknahme" | "kommunikation" | "nachforderung" | "schritt" | "abgabe" | "versand" | "bescheid" | "zustellung" | "nebenbestimmung" | "ausfuehrung" | "berichtigung" | "notiz" | "angaben" | "abweichung" | "gebuehr";
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
 * Dateiname aus dem jüngsten Abgabestand; bei nur extern eingereichten
 * Unterlagen der Dateiname im DMS (leer, wenn nicht mehr vorhanden).
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
export type Terminart = "forderungspunkt" | "nebenbestimmung" | "ablauf" | "anhoerung" | "zurueckstellung" | "rechtsbehelfsfrist" | "geltungsdauer" | "fertigstellung" | "beteiligung" | "bearbeitungsfrist" | "zahlung";
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
export type Abweichungsart = "abweichung" | "ausnahme" | "befreiung" | "erleichterung";
export type Nachbarzustimmung = { id: string, name: string,
/**
 * Flurstück des Angrenzers, frei formuliert.
 */
flurstueck: string,
/**
 * `None` angefragt, `Some(true)` zugestimmt, `Some(false)` verweigert.
 */
zugestimmt: boolean | null, datum: string | null,
/**
 * Unterschriebene Zustimmung oder Ablehnung.
 */
beleg: Dokumentbezug | null, };
export type Abweichung = { id: string, art: Abweichungsart,
/**
 * Betroffene Vorschrift, z. B. „§ 6 Abs. 5 BauO NRW (Tiefe der Abstandsfläche)“.
 */
vorschrift: string,
/**
 * Rechtsgrundlage der Zulassung, z. B. „§ 31 Abs. 2 BauGB“.
 */
rechtsgrundlage: string,
/**
 * Was weicht ab (Bauteil, Wand, Festsetzung).
 */
gegenstand: string, soll: string, ist: string,
/**
 * Warum die Zulassung gerechtfertigt ist (Schutzziel, nachbarliche und
 * öffentliche Belange).
 */
begruendung: string,
/**
 * Schlüssel des abgeleiteten Bedarfs, den dieser Eintrag abdeckt.
 */
herkunft?: string, nachbarn?: Array<Nachbarzustimmung>,
/**
 * Begründende Unterlagen (Lageplanausschnitt, Gutachten).
 */
dokumente?: Array<Dokumentbezug>,
/**
 * Code der eingespielten XBau-Codeliste (siehe [`Abweichungsart::codeliste`]);
 * leer = Vorschlag nach der Art.
 */
xbau_code?: string,
/**
 * Nicht (mehr) erforderlich, mit Begründung (z. B. Planänderung,
 * Baulast). Der Eintrag bleibt in der Akte.
 */
entfallen?: string, };
export type Abweichungsentscheidung = { abweichung: string, zugelassen: boolean, begruendung: string,
/**
 * Nebenbestimmung zur Zulassung, wörtlich aus dem Bescheid.
 */
nebenbestimmung: string, };
export type Abweichungsbedarf = {
/**
 * Stabiler Schlüssel aus Herkunft und Gegenstand.
 */
schluessel: string, art: Abweichungsart, vorschrift: string, rechtsgrundlage: string, gegenstand: string, soll: string, ist: string,
/**
 * Nachweis, aus dem der Bedarf stammt.
 */
quelle: string, };
export type Abweichungsabgleich = { bedarf: Array<Abweichungsbedarf>,
/**
 * Schlüssel des Bedarfs ohne Eintrag in der Akte.
 */
offen: Array<string>,
/**
 * Einträge, deren Bedarf im aktuellen Planungsstand nicht mehr besteht.
 */
ohne_bedarf: Array<string>,
/**
 * Einträge, deren Soll oder Ist vom aktuellen Bedarf abweicht.
 */
geaendert: Array<string>,
/**
 * Nicht ermittelbare Bereiche und weitere Hinweise.
 */
hinweise: Array<string>, };
export type Beanstandung = { id: string, text: string, grundlage: string,
/**
 * Wie der Antrag reagiert: Stellungnahme, Planänderung, Abweichungsantrag.
 */
reaktion: string,
/**
 * Abweichung dieser Akte, mit der die Beanstandung ausgeräumt werden soll.
 */
abweichung?: string, };
export type FormellerBefund = {
/**
 * Code der Befundart laut Nachricht.
 */
art: string, text: string, };
export type MateriellerBefund = { rechtsbereich: string, regelverstoss: string, begruendung: string, };
export type BescheidNebenbestimmung = { art: Nebenbestimmungsart,
/**
 * Code der XBau-Liste „Nebenbestimmung Art“ (1 Bedingung … 6
 * aufschiebende Bedingung).
 */
xbau_code: string, text: string, };
export type BescheidAbweichung = {
/**
 * Kennung der beantragten Abweichung (aus dem eigenen Antrag).
 */
id: string, zugelassen: boolean, vorschrift: string, begruendung: string, nebenbestimmung: string, };
export type Auswertung = { "art": "eingangsbestaetigung", information: string, } | { "art": "aktenzeichen", aktenzeichen: string, } | { "art": "befundliste", vollstaendig: boolean, befunde: Array<FormellerBefund>, frist: string | null, anschreiben: string, } | { "art": "anhoerung", aufforderung: string, befunde: Array<MateriellerBefund>, frist: string | null, } | { "art": "bescheid", genehmigung: boolean, nebenbestimmungen: Array<BescheidNebenbestimmung>, abweichungen: Array<BescheidAbweichung>, begruendung: string, rechtsbehelfsbelehrung: string, hinweise: string, } | { "art": "gebuehrenbescheid", vorauszahlung: boolean,
/**
 * Summe in Eurocent, wenn die Kostenberechnung sie nennt.
 */
summe_cent: number | null, begruendung: string, verwendungszweck: string, frist: string | null, rechtsbehelfsbelehrung: string, } | { "art": "sonstige" };
export type Gebuehrenbescheid = { id: string,
/**
 * Eingegangenes Schreiben dieser Akte.
 */
kommunikation: string, datum: string,
/**
 * Betrag in Eurocent, wenn bekannt.
 */
betrag_cent: number | null, vorauszahlung: boolean, verwendungszweck: string, frist: Frist | null,
/**
 * Tag der Zahlung; beendet die Wiedervorlage.
 */
bezahlt: string | null, verantwortlich: string | null, };
export type Nachrichtenauswertung = { nachricht: string, element: string, titel: string, auswertung: Auswertung,
/**
 * Was die Übernahme in der gewählten Akte bewirkt; leer ohne Akte.
 */
wirkung: Array<string>,
/**
 * Bescheid: vorgeschlagene genehmigte Unterlagen aus den versendeten
 * Abgabeständen des Antrags.
 */
genehmigte_unterlagen: Array<Dokumentbezug>,
/**
 * Bereits fachlich in die gewählte Akte übernommen.
 */
uebernommen: boolean, };
export type Uebernahmewunsch = { verfahren: string, basis: number,
/**
 * Bestätigtes Zugangsdatum; ohne Angabe der Tag der Erfassung im Journal.
 */
zugang: string | null,
/**
 * Bescheid: ausdrücklich genehmigte Unterlagen.
 */
genehmigte_unterlagen: Array<Dokumentbezug>, };
export type Unterschriftsrolle = "bauherrschaft" | "entwurfsverfasser" | "fachplaner";
export type Unterschriftsart = "handschriftlich" | "qualifiziert" | "portal";
export type Unterschrift = { id: string, dokument: Dokumentbezug, rolle: Unterschriftsrolle, name: string, art: Unterschriftsart, datum: string, };
export type Unterschriftsbedarf = { dokument: Dokumentbezug, name: string, kategorie: string, erforderlich: Array<Unterschriftsrolle>, fehlend: Array<Unterschriftsrolle>, quelle: string, };
export type Merkmal = "wohngebaeude" | "gebaeudeklasse_bis" | "bebauungsplan" | "keine_planungsrechtliche_abweichung" | "erschliessung" | "gemeinde_verzicht" | "kein_grosser_sonderbau" | "grosser_sonderbau";
export type Fiktionsregel = { rechtsgrundlage: string, regulaer_monate: number, vereinfacht_wochen: number, hinweis: string, };
export type Vorhabenmerkmale = { gebaeudeklasse: number | null, sonderbau: boolean | null, grosser_sonderbau: boolean | null, wohngebaeude: boolean | null, bebauungsplan: boolean | null, planungsrechtliche_abweichung: boolean | null, erschliessung: boolean | null, gemeinde_verzicht: boolean | null, };
export type Eignung = "moeglich" | "ausgeschlossen" | "pruefen";
export type Bedingungsergebnis = { text: string, erfuellt: boolean | null, quelle: string, };
export type Verfahrensvorschlag = { art: Verfahrensart, pruefverfahren: Pruefverfahren | null, titel: string, rechtsgrundlage: string, eignung: Eignung, bedingungen: Array<Bedingungsergebnis>, };
export type Verfahrenswahl = { stichtag: string,
/**
 * Angewandte Fassung; leer ohne Regelpaket für das Land.
 */
fassung: string | null, geprueft: boolean, merkmale: Vorhabenmerkmale, vorschlaege: Array<Verfahrensvorschlag>,
/**
 * Genehmigungsfiktion nach der Fassung, falls vorgesehen.
 */
fiktion: Fiktionsregel | null, hinweise: Array<string>, };
export type Verfahrensangaben = { wohngebaeude: boolean | null, grosser_sonderbau: boolean | null, erschliessung: boolean | null, gemeinde_verzicht: boolean | null,
/**
 * Stichtag (Antragseingang); ohne Angabe heute.
 */
stichtag: string | null, };
export type Briefblock = { "art": "absatz", text: string, } | { "art": "abschnitt", titel: string, } | { "art": "hinweis", text: string, } | { "art": "tabelle", koepfe: Array<string>, zeilen: Array<Array<string>>, } | { "art": "unterschrift", felder: Array<[string, string]>, };
export type Brief = { absender: string, empfaenger: string, datum: string, betreff: string,
/**
 * Aktenzeichen der Behörde; leer = ohne.
 */
aktenzeichen: string, bloecke: Array<Briefblock>, };
export type Schreibenart = { "art": "nachforderungsantwort", nachforderung: string, } | { "art": "fristverlaengerung",
/**
 * Forderungspunkt oder Verfahrensschritt mit Frist.
 */
bezug: string, bis: string, grund: string, } | { "art": "stellungnahme", schritt: string, } | { "art": "abweichungsantrag" } | { "art": "sachstandsanfrage" } | { "art": "ruecknahme" };
export type Schreibenwunsch = { art: Schreibenart,
/**
 * Datum des Schreibens; ohne Angabe heute.
 */
datum: string | null,
/**
 * Zusätzlicher Text vor dem Gruß.
 */
zusatz: string,
/**
 * Als Projektunterlage ablegen.
 */
ablegen: boolean, };
export type Schreibenergebnis = { dateiname: string, brief: Brief,
/**
 * Hinweise zur Vollständigkeit (fehlende Antworten, Unterlagen …).
 */
hinweise: Array<string>,
/**
 * Abgelegte Dokumentversion, wenn gewünscht.
 */
dokument: Dokumentbezug | null, };
export type Ablehnungsgrund = { id: string,
/**
 * Ablehnender Bescheid dieser Akte.
 */
bescheid: string, text: string, grundlage: string, reaktion: string, abweichung?: string,
/**
 * Folgeverfahren, in dem der Grund ausgeräumt wird.
 */
folgeverfahren?: string, };
export type Nachbarbekanntgabe = { id: string, bescheid: string, name: string, datum: string, beleg: Dokumentbezug | null,
/**
 * Tag, an dem der Nachbar Widerspruch oder Klage erhoben hat.
 */
angefochten?: string, };
export type Optionsaktion = { "art": "schreiben", schreiben: string, } | { "art": "folgeverfahren", verfahrensart: Verfahrensart, } | { "art": "register", register: string, };
export type Handlungsoption = { titel: string, beschreibung: string, frist: string | null, rechtsgrundlage: string, aktion: Optionsaktion | null, };
export type Verfahrenslage = {
/**
 * Kurze Beschreibung der Lage, z. B. „Anhörung offen“.
 */
lage: string, optionen: Array<Handlungsoption>, };
export type Folgeverfahrenswunsch = { id: string, art: Verfahrensart, bezeichnung: string, };
export type Faelligkeit = "mit_antrag" | "vor_baubeginn" | "waehrend_bauausfuehrung" | "vor_nutzungsaufnahme" | "dauerhaft";
export type Nachweisart = "standsicherheit" | "brandschutz" | "waermeschutz" | "schallschutz" | "sonstiges";
export type Pruefart = "keine" | "bauaufsicht" | "pruefingenieur" | "sachverstaendiger";
export type BautechnischerNachweis = { id: string, art: Nachweisart, bezeichnung: string, pruefung: Pruefart,
/**
 * Prüfende Person oder Stelle.
 */
pruefer: string, faellig: Faelligkeit,
/**
 * Bei der Behörde vorgelegt am.
 */
vorgelegt: string | null,
/**
 * Geprüft bzw. bescheinigt am (Prüfbericht, Bescheinigung).
 */
bescheinigt: string | null, dokumente?: Array<Dokumentbezug>, };
export type Bauvorbereitung = {
/**
 * Bauleiterin oder Bauleiter.
 */
bauleitung: string,
/**
 * Der Behörde benannt am.
 */
bauleitung_benannt: string | null,
/**
 * Grundrissfläche und Höhenlage abgesteckt am.
 */
absteckung: string | null,
/**
 * Baubeginn angezeigt am.
 */
baubeginnanzeige: string | null,
/**
 * Fertigstellung bzw. Nutzungsaufnahme angezeigt am.
 */
nutzungsanzeige: string | null, belege?: Array<Dokumentbezug>, };
export type Phase = "baubeginn" | "nutzungsaufnahme";
export type Voraussetzung = { titel: string, erfuellt: boolean | null, hinweis: string, quelle: string, };
export type Phasencheckliste = { phase: Phase, voraussetzungen: Array<Voraussetzung>,
/**
 * Alle belegt.
 */
bereit: boolean, hinweise: Array<string>, };
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
schritte: Array<Verfahrensschritt>, anforderungen: Array<Anforderung>,
/**
 * Abweichungen, Ausnahmen, Befreiungen und Erleichterungen. Ältere Akten
 * lesen die Angabe leer; leer wird sie nicht geschrieben.
 */
abweichungen?: Array<Abweichung>, dokumentbeziehungen: Array<Dokumentbeziehung>, abgaben: Array<Abgabestand>,
/**
 * Außerhalb der App erfolgte Einreichungen. Ältere Akten ohne diese
 * Angabe lesen sie leer; leer wird sie nicht geschrieben.
 */
externe_einreichungen?: Array<ExterneEinreichung>, bescheide: Array<Bescheid>,
/**
 * Gebührenbescheide mit Zahlungsfrist; ältere Akten lesen die Angabe leer.
 */
gebuehren?: Array<Gebuehrenbescheid>,
/**
 * Erfasste Unterschriften auf Dokumentversionen (§ 70 Abs. 3 BauO NRW
 * u. ä.); ältere Akten lesen die Angabe leer.
 */
unterschriften?: Array<Unterschrift>,
/**
 * Gründe ablehnender Bescheide; ältere Akten lesen die Angabe leer.
 */
ablehnungsgruende?: Array<Ablehnungsgrund>,
/**
 * Bekanntgaben der Genehmigung an Nachbarn.
 */
nachbarbekanntgaben?: Array<Nachbarbekanntgabe>,
/**
 * Bautechnische Nachweise mit Vorlage und Prüfung.
 */
bautechnische_nachweise?: Array<BautechnischerNachweis>,
/**
 * Bauleitung, Absteckung und Anzeigen.
 */
bauvorbereitung?: Bauvorbereitung, ereignisse: Array<Aktenereignis>, };
export type Vorhabenprofil = { bundesland: Bundesland, art: Verfahrensart, gebaeudeklasse: number | null, sonderbau: boolean | null, nutzung: string,
/**
 * Wohngebäude ja/nein; fehlt = noch nicht geklärt.
 */
wohngebaeude?: boolean,
/**
 * Planungsrechtliche Grundlage des Projekts (§ 30, § 34 oder § 35 BauGB).
 * Der Core trägt sie beim Anwenden des Profils aus dem Projekt ein.
 */
rechtsgrundlage?: Rechtsgrundlage, };
export type Profilregel = { id: string, titel: string, quelle: string, sonderbau: boolean | null, ab_gebaeudeklasse: number | null, nutzung: string | null,
/**
 * Verfahrensarten, für die die Anforderung gilt; fehlt = alle.
 */
arten?: Array<Verfahrensart>,
/**
 * Dokumentkategorien, deren Unterlagen die Anforderung belegen können;
 * ihre neuesten Versionen werden beim Anwenden vorgeschlagen.
 */
kategorien?: Array<string>,
/**
 * Gilt nur für Wohngebäude (`true`) bzw. nur für andere Gebäude (`false`).
 */
wohngebaeude?: boolean,
/**
 * Gilt nur bei diesen planungsrechtlichen Grundlagen; fehlt = immer.
 */
rechtsgrundlagen?: Array<Rechtsgrundlage>,
/**
 * Nur im Einzelfall nötig (z. B. Abweichungsanträge): bleibt ungeklärt,
 * auch wenn alle Bedingungen erfüllt sind.
 */
bei_bedarf?: boolean,
/**
 * Bereiche der Projektakte, deren Nachweis die Anforderung belegt
 * (z. B. `stellplaetze`, `abstand`). Der Baum zeigt sie als offen, bis
 * ein Nachweis vorliegt.
 */
nachweise?: Array<string>, };
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
export type Abgabelage = { text: string,
/**
 * Tag des maßgebenden Ereignisses (Versand, Eingang, externe
 * Einreichung); fehlt, solange nichts versendet ist.
 */
datum: string | null,
/**
 * Liegt der Stand der Behörde vor?
 */
eingereicht: boolean,
/**
 * Externe Einreichung, die diesen nachträglich festgehaltenen Stand belegt.
 */
extern_belegt?: string, };
export type Abgabestand = { id: string, auftrag_hash: string, titel: string, erstellt_am: string, erstellt_von: string, grundlagen: JsonValue, dokumente: Array<Abgabedokument>, anforderungen: Array<Anforderung>, hinweise: Array<string>, artefakte: Array<Artefakt>, versand: Array<Versandereignis>,
/**
 * Vom Core aus Originalereignissen und Berichtigungen abgeleitet.
 */
versandstatus: Versandstatus | null,
/**
 * Die eine Aussage zum Stand dieser Einreichung (siehe [`lage`]); vom
 * Core beim Speichern der Akte abgeleitet. Fehlt in älteren Ständen.
 */
lage?: Abgabelage, };
export type Folgewunsch = { "art": "stellungnahme", schritt: string, } | { "art": "abweichungsantrag" } | { "art": "verlaengerung", bis: string | null, } | { "art": "baubeginn", datum: string, pruefung_standsicherheit: boolean, } | { "art": "nutzungsaufnahme", datum: string, standsicherheit_bescheinigt: boolean, brandschutz_bescheinigt: boolean, wiederkehrende_pruefungen: boolean, } | { "art": "ruecknahme" };
export type Abgabewunsch = { xbau_bezug?: string,
/**
 * XBau-Folgenachricht statt des Antrags; fehlt bei Anträgen und
 * Nachreichungen.
 */
xbau_folge?: Folgewunsch, id: string, titel: string, dokumente: Array<Dokumentbezug>, xbau: boolean, druck: boolean, anschreiben: string, };
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
export type ObjektArt = "stellplaetze" | "wohnflaechen" | "din277" | "vollgeschosse" | "hoehen" | "freiflaechen" | "entwaesserung" | "ueberbaubarkeit" | "abstandsflaechen" | "verfahren" | "projektorganisation" | "gebaeude" | "aufgaben" | "unterlagen" | "dokumentdaten" | "raumbuch" | "planaufmass" | "cadaufmass" | "ifcmodell" | "projekttagebuch" | "planliste" | "fachplanerlieferung" | "baukennwerte";
export type Projektphase = "vorbereitung" | "vorplanung" | "entwurf" | "genehmigung" | "ausfuehrung" | "abgeschlossen";
export type Projektorganisation = { kennzeichen: string, phase: Projektphase, verantwortlich: BenutzerId | null, beschreibung: string,
/**
 * Navigationsbereiche, die das Büro für dieses Projekt ausdrücklich als
 * nicht benötigt gekennzeichnet hat (siehe [`crate::bereichsstatus`]).
 * Fehlt in älteren Ständen.
 */
nicht_benoetigt?: Array<string>, };
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
export type GebaeudeGeometrie = { typ?: Bauteiltyp, grundflaeche_m2?: number, nullpunkt_m_nhn?: number, gelaende_m?: number, fundamentsohle_m?: number, traufe_m?: number, first_m?: number,
/**
 * Dachform; `None` = nicht erfasst. Fehlt in älteren Ständen.
 */
dachform?: Dachform,
/**
 * Dachneigung in Grad (0 bis unter 90); bei mehreren Dachflächen die
 * maßgebende Neigung. `None` = nicht erfasst.
 */
dachneigung_grad?: number, einzelgeschoss?: GeschossGeometrie,
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
hoehen_nhn: HoehenNhn,
/**
 * Dach, sofern eine Dachform erfasst ist. Fehlt in älteren Ständen.
 */
dach: Dachgrundlagen | null, };
export type GeschossGrundlagen = { id: string, bezeichnung: string, implizit: boolean, flaeche_m2: number | null, vollgeschoss: boolean | null, lichte_hoehe_m: number | null, decke_oberkante_m: number | null, decke_ueber_gelaende_m: number | null, unterliegend_m2: number | null, herleitung: string, };
export type HoehenNhn = { gelaende_m: number | null, traufe_m: number | null, first_m: number | null, };
export type Dachform = "flachdach" | "satteldach" | "pultdach" | "walmdach" | "sonstige";
export type Dachgrundlagen = { form: Dachform, neigung_grad: number | null,
/**
 * Höhe des Dachs über der Traufe in m; `None`, solange Traufe oder First fehlt.
 */
dachhoehe_m: number | null, herleitung: string, };
export type Flurstuecksverweis = { flurstueck_id: string,
/**
 * Fachbereich des verweisenden Objekts: `gebaeude`, `abstandsflaechen`
 * oder `planaufmass`.
 */
bereich: string, objekt: string, bezeichnung: string, };
export type AufgabenStatus = "offen" | "in_arbeit" | "erledigt";
export type Aufgabe = { titel: string, beschreibung: string, faellig: string | null, verantwortlich: BenutzerId | null, status: AufgabenStatus, bezug: string | null, };
export type Tagebuchart = "notiz" | "telefonat" | "besprechung" | "ortstermin" | "vorfall" | "entscheidung_bauherr" | "sonstiges";
export type Wiedervorlage = { datum: string, verantwortlich: BenutzerId | null, erledigt: boolean, };
export type Tagebucheintrag = {
/**
 * Tag des Ereignisses (Ortsdatum), nicht nach heute.
 */
datum: string, art: Tagebuchart, titel: string, text: string,
/**
 * Beteiligte Personen oder Firmen als freie Angabe.
 */
beteiligte: string,
/**
 * Fotos, Protokolle oder Schreiben aus den Projektdateien.
 */
anhaenge: Array<Dokumentbezug>,
/**
 * Optionaler Bezug auf eine Verfahrensakte des Projekts.
 */
verfahren: string | null, wiedervorlage: Wiedervorlage | null, };
export type Fachdisziplin = "tragwerksplanung" | "brandschutz" | "waermeschutz" | "schallschutz" | "vermessung" | "entwaesserung" | "baugrund" | "haustechnik" | "freianlagen" | "sonstiges";
export type Lieferstatus = "offen" | "angefragt" | "teilweise" | "geliefert" | "geprueft" | "entfallen";
export type Lieferposition = { bezeichnung: string, geliefert: boolean, };
export type Fachplanerlieferung = { disziplin: Fachdisziplin,
/**
 * Büro oder Person des Fachplaners.
 */
fachplaner: string, kontakt: string, positionen: Array<Lieferposition>, angefragt_am: string | null,
/**
 * Zugesagter Liefertermin.
 */
faellig: string | null, status: Lieferstatus,
/**
 * Wer im Büro die Lieferung nachhält.
 */
verantwortlich: BenutzerId | null,
/**
 * Gelieferte Unterlagen aus den Projektdateien.
 */
dokumente: Array<Dokumentbezug>,
/**
 * Verfahren, für das geliefert wird; leer = alle Verfahren des Projekts.
 */
verfahren: string | null, bemerkung: string, };
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
export type Ampel = "rot" | "gelb" | "gruen";
export type Pruefbereich = "grundlagen" | "unterlagen" | "nachweise" | "plaene" | "formulare" | "antrag" | "fachplaner" | "verfahren";
export type Pruefpunkt = { id: string, bereich: Pruefbereich, ampel: Ampel, titel: string, hinweis: string, faellig: string | null, verantwortlich: string | null, ziel: UebersichtsZiel, };
export type Bereichsstand = { bereich: Pruefbereich, bezeichnung: string, ampel: Ampel, rot: number, gelb: number, };
export type Einreichungspruefung = { stichtag: string, verfahren: string | null, gesamt: Ampel, bereiche: Array<Bereichsstand>,
/**
 * Rot vor Gelb, innerhalb nach Bereich.
 */
punkte: Array<Pruefpunkt>, };
export type Regelquelle = { titel: string, url: string | null, };
export type Gebaeudeartwert = { id: string, bezeichnung: string, euro_je_m3: number | null, };
export type Bautaetigkeit = "neubau" | "bestand";
export type Bauherrenart = "privater_haushalt" | "unternehmen" | "oeffentliche_hand" | "organisation_ohne_erwerbszweck";
export type Baukennwerte = {
/**
 * Kennung einer Gebäudeart aus dem Regelpaket.
 */
gebaeudeart: string,
/**
 * Brutto-Rauminhalt; leer = aus der DIN-277-Berechnung.
 */
bri_m3: number | null,
/**
 * Rohbauwert je m³; leer = Wert des Regelpakets.
 */
euro_je_m3: number | null,
/**
 * Gebührensatz in Promille des Rohbauwerts; leer = Regelpaket.
 */
promille: number | null, bautaetigkeit: Bautaetigkeit,
/**
 * Leer = aus der Kundenkategorie des ersten Bauherrn.
 */
bauherrenart: Bauherrenart | null,
/**
 * Leer = Wohneinheiten der Wohnflächenberechnung.
 */
wohnungen: number | null, heizung: string, energie_primaer: string, energie_sekundaer: string, baustoff: string, fertigteilbau: boolean,
/**
 * Veranschlagte Kosten des Bauwerks (Kostengruppen 300 und 400).
 */
kosten_bauwerk_euro: number | null, };
export type Kennwert = { wert: number | null,
/**
 * „Eingabe“, Berechnungsname oder Regelpaket.
 */
quelle: string, };
export type Gebuehrenschaetzung = { land: string, gebaeudeart: string, bri_m3: Kennwert, euro_je_m3: Kennwert, rohbauwert_euro: number | null, promille: Kennwert, mindestgebuehr_euro: number, gebuehr_euro: number | null, rechtsgrundlage: string,
/**
 * `false` = Regelwerte ohne fachliche Abnahme.
 */
geprueft: boolean, stand: string, offen: Array<string>, hinweise: Array<string>, quellen: Array<Regelquelle>, gebaeudearten: Array<Gebaeudeartwert>, };
export type Statistikmerkmal = { gruppe: string, bezeichnung: string,
/**
 * Leer = noch keine Angabe.
 */
wert: string, quelle: string, };
export type Baustatistikbogen = { merkmale: Array<Statistikmerkmal>,
/**
 * Bezeichnungen ohne Angabe.
 */
offen: Array<string>, hinweis: string, };
export type Tagesmaileinstellung = { aktiv: boolean, adresse: string, umfang: Aboumfang,
/**
 * Abwesend bis einschließlich; während dessen gehen eigene Termine an die Vertretung.
 */
abwesend_bis: string | null, vertretung: BenutzerId | null, };
export type Projektuebersicht = { projekt_id: ProjectId, projekt_version: bigint, ermittelt_am: string, stichtag: string, zahlen: UebersichtsZahlen, punkte: Array<Arbeitspunkt>, verfahren: Array<VerfahrensUeberblick>, abgaben: Array<AbgabeUeberblick>, empfaenger: Array<EmpfaengerUeberblick>, hinweise: Array<string>, };
export type Bereichszustand = "problem" | "veraltet" | "in_arbeit" | "zu_erledigen" | "geprueft" | "leer" | "nicht_benoetigt";
export type Bereichsstatus = {
/**
 * Kennung des Bereichs wie in der Navigation (z. B. `flurstuecke`, `grz`).
 */
bereich: string, zustand: Bereichszustand,
/**
 * Kurze Gründe für den Zustand: Probleme, dann Veraltetes, dann Offenes.
 */
gruende: Array<string>,
/**
 * Ob sich der Bereich als „nicht benötigt“ kennzeichnen lässt: nur
 * Datenbereiche, in denen noch nichts erfasst ist.
 */
kennzeichenbar: boolean, };
export type Bereichsstaende = { projekt_version: bigint, stichtag: string, bereiche: Array<Bereichsstatus>, };
export type Dokumentdaten = { dokument: Dokumentbezug, dokumentart: string, plannummer: string, revisionsindex: string, massstab: string, verfasser: string, datum: string | null, };
export type Raumbuch = { planbindungen?: Array<Planbindung>, bezeichnung: string, gebaeude_id: string, geschoss_id: string | null, raeume: Array<Raum>, };
export type Raum = { id: string, nummer: string, bezeichnung: string, nutzung: string,
/**
 * Grundfläche des Raums einschließlich seiner Höhenzonen.
 */
flaeche_m2: number,
/**
 * Lichte Höhe der Restfläche außerhalb der Höhenzonen.
 */
hoehe_m: number | null,
/**
 * Teilflächen mit abweichender lichter Höhe, z. B. unter Dachschrägen.
 * Leer = einheitliche Raumhöhe. Fehlt in älteren Ständen.
 */
hoehenzonen?: Array<Hoehenzone>, };
export type Hoehenzone = { id: string, bezeichnung: string, flaeche_m2: number,
/**
 * Kleinste lichte Höhe in der Zone (m); `None` = noch nicht erfasst
 * (z. B. nach Übernahme aus dem Aufmaß).
 */
lichte_hoehe_min_m: number | null,
/**
 * Größte lichte Höhe in der Zone (m); für den Rauminhalt bei geneigter
 * Decke. `None` = nicht erfasst.
 */
lichte_hoehe_max_m?: number, };
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
aufgestellt_von: string,
/**
 * Vorschlag für das Aktenzeichen: das der Verfahrensakte, wenn das Projekt
 * genau ein Aktenzeichen führt; sonst leer.
 */
aktenzeichen: string, };
export type Druckposition = { titel: string, erste_seite: number, seiten: number, };
export type NachweisAnsicht = { stand: Nachweisstand, aktuell: boolean, freigabe: Freigabe | null,
/**
 * Technischer Fehler: kein Beleg für eine fachliche Quellenänderung.
 */
prueffehler?: string,
/**
 * Neuerer Stand desselben Nachweises (gleiche Berechnung, gleicher
 * Bezugsumfang). Ein ersetzter Stand ist Archiv und kein Handlungsbedarf.
 */
ersetzt_durch?: string, };
export type Erneuerung = { bezeichnung: string,
/**
 * Der veraltete Stand.
 */
alt: string,
/**
 * Der neu archivierte Stand; `None`, wenn eine Prüfung von Hand nötig ist.
 */
neu: string | null,
/**
 * Warum der Nachweis nicht automatisch erneuert wurde.
 */
grund: string | null, };
