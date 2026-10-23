# Praxisworkflows und Prüfszenarien

Die Schritte beschreiben den aktuellen empfohlenen Weg. Die Zielkriterien
beschreiben die nächste Verbesserung, noch keine implementierte Funktion.
Prüfmethoden und Grenzen stehen im [Audit](README.md).

## W01 – Einfamilienhaus nach §34, Grundbuch noch nicht vorhanden

**Ziel:** Früh arbeiten können, obwohl noch nicht alle rechtlichen Unterlagen
vorliegen. Daten: ein Flurstück 500 m², kein B-Plan, Rechtsgrundlage §34,
Grundbuch leer, BauNVO noch ungeklärt.

**Ablauf:** Projekt anlegen → Rechtsgrundlagen erfassen → Flurstück speichern →
Baugrundstück ermitteln/prüfen und speichern → Gebäude anlegen → Nutzungsnachweise.

**Prüfung:** B: §34 ohne BauNVO sperrt GRZ, GFZ, Geschossigkeit und BMZ in der
Navigation. Der Button-Tooltip nennt die Voraussetzung. Die Baugrundstücksmaske
zeigt bei fehlendem Grundbuch „Optional – Flurstück wird einzeln angesetzt“.
Q: Der Core gruppiert Flurstücke ohne vollständige Grundbuchstelle einzeln;
der frühere allgemeine Grundbuch-Pflichtfehler entspricht nicht diesem Stand.
Die tatsächliche Gruppierung wurde im neuen Browserlauf nicht nativ ausgeführt.

**Reibung:** Der Nutzer kann die vorbereitende Erfassung der vier gesperrten
Module nicht erreichen und könnte eine BauNVO auswählen, nur um weiterzukommen.
§34 ist keine pauschale Aufforderung, eine beliebige BauNVO auszuwählen.

**Ziel:** Eingaben als Entwurf zugänglich halten; Berechnungsmaßstab separat
klären. Sichtbar „Maßstab noch offen“, Aktion zur Rechtsgrundlage, keine
automatische positive Nachweisbewertung. Befunde U01, U02.

## W02 – B-Plan, drei Flurstücke und zwei Baugrundstücke

**Daten:** F1 400 m² und F2 200 m² mit gleicher vollständiger Grundbuchstelle,
F3 350 m² mit anderer Stelle. Auf F2 40 m² begründeter Abzug; auf einer Einheit
gegebenenfalls ein planungsrechtlich begründeter Gemeinschaftsanteil.

**Ablauf:** §30, Planart und maßgebende Fassungen erfassen → Planfestsetzungen →
drei Flurstücke speichern → Baugrundstückszuordnungen/Abzüge → Ermittlung prüfen →
Bezugsflächen berechnen und Eingaben speichern → Gebäude auf Flurstücke legen →
GRZ/GFZ je Einheit berechnen → Nachweis archivieren → Druck.

**Sollkontrolle:** Zwei getrennte Einheiten; erste Fläche vor Zuschlägen 560 m²,
zweite 350 m². Keine Vermischung durch ein gemeinsames Projekt. Gemeinschaftsanteil
nur nach bestätigter Zuordnung und nicht doppelt. Dies ist ein entworfener
fachlicher Kontrollfall, kein in diesem Audit neu ausgeführtes Rechenergebnis.

**Prüfung:** Q: getrennte Core-Einheiten und Baugrundstücksmaske. B: bestehende
Suite prüft projektgetrennte Flurstücksentwürfe; keine native Mehrflurstückrechnung.

**Reibung:** „Ermitteln“, „Bezugsflächen berechnen“, „Eingaben speichern“ und
Nachweisstand sind aufeinanderfolgende Entscheidungen. Bei Neuzuordnung kommen
Altdatenübernahme und Gebäudezuordnung hinzu.

**Ziel:** Eine geführte Ermittlung mit Vorschau Alt → Neu und getrennten
Bestätigungen für Flächenänderung und Zusammenführung. Keine automatische
Zusammenlegung nach bloßer räumlicher Nähe. U02, U03, U06.

## W03 – Geschosse, Dachgeschoss und GFZ

**Daten:** Wohnhaus EG/OG/DG; Einstufung DG noch offen. BauNVO-Fassung ist
projektbezogen geklärt. Mindestens eine Geschossfläche stammt aus einem Plan.

**Ablauf:** Gebäude/Geschosse speichern → Vollgeschossprüfung mit Gebäudequelle →
Prüfung und Nachweisstand → Einstufung am Geschoss aus Nachweis übernehmen →
Gebäude speichern → GFZ/Geschossigkeit neu rechnen und archivieren.

**Prüfung:** B: Gebäudeentwurf bleibt über mehrere Bereichswechsel erhalten;
Fachsuite öffnet und prüft Vollgeschossmaske anhand einer Backend-Fixture.
Q: Gebäude besitzt die Auswahl „Aus Vollgeschossnachweis übernehmen“.

**Reibung:** Hin-und-zurück-Weg zwischen lieferndem Gebäudemodul und abgeleitetem
Nachweis. Ohne Erklärung wirkt er kreisförmig; offene Einstufungen können mit
bewussten Nicht-Vollgeschossen verwechselt werden.

**Ziel:** Ein Aktionsweg „Einstufung ermitteln“ aus dem Geschoss, Rückkehr an
dasselbe Geschoss und Vorschau der betroffenen GFZ-Stände. Offene Einstufung
bleibt ausdrücklich offen. Keine rekursive Vollgeschoss-/Gebäudeberechnung. U06, U07.

## W04 – Plan ändern, Raumbuch und Wohnflächen aktualisieren

**Daten:** Plan v1 mit Raum R01; Plan v2 ändert R01, ergänzt R02 und entfernt R03.
Wohnungszuordnungen und manuelle Wohnflächenpositionen existieren bereits.

**Ablauf:** Neue Planversion desselben Dokuments → PDF-Aufmaß kalibrieren und
Räume zuordnen → Raumbuch aus Plan aktualisieren → Wohnflächen/DIN277 öffnen →
Raumfakten übernehmen/aktualisieren → Zuordnungen/Höhenzonen prüfen → speichern,
Regelgrundlagen bestätigen, rechnen und neuen Nachweis archivieren.

**Prüfung:** Q: Planquellen und Raumbuchimport; Import erhält bestimmte
Zuordnungen und manuelle Positionen, entfernt aber entfallene Raumbuchpositionen.
B: Wohnflächenwert ändern, Bereich wechseln, Konflikt auslösen, Eingabe erhalten,
anschließend speichern und erneute Prüfung mit offenen Punkten.

**Reibung:** Mehrere bewusste Übernahmen sind erforderlich. Der Wohnflächenimport
meldet die Auswirkungen nach dem Import; der Nutzer braucht eine Vorschau vor
dem Entfernen alter Quellpositionen.

**Ziel:** Vergleich Neu/Geändert/Entfallen vor Übernahme, Rückgängig für den
unverbindlichen Entwurf, Quellenrevision sichtbar. Keine stille Aktualisierung
archivierter Stände. U06, U08.

## W05 – Stellplätze nach kommunaler Satzung

**Daten:** Mehrfamilienhaus, verschiedene Nutzungen; Satzung mit kommunalem
Faktor und Rundung. Die konkrete Satzung und ihre Geltung müssen geklärt sein.

**Ablauf:** Stellplatznachweis anlegen → Grundstück/Gebäude zuordnen →
Regel-/Satzungsgrundlagen prüfen oder Paket übernehmen → Nutzungen und
Bezugsgrößen erfassen → speichern → Anwendbarkeit bestätigen → berechnen →
Nachweis archivieren → PDF.

**Prüfung:** B: Maske, Prüfbericht, wiederholtes idempotentes Archivieren und
PDF-Vorschau anhand vorhandener Core-Fixture. Q: Regelkontext und bewusste
Bestätigung nach Änderungen. Keine neue Prüfung einer realen Gemeindesatzung.

**Reibung:** Satzungsadministration und normale Projekteingabe befinden sich
nah beieinander. Jede Änderung kann eine erneute Bestätigung erforderlich machen.

**Ziel:** Versionierte Büro-/Gemeindepakete auswählen, Geltung bestätigen;
Regelbearbeitung als bewusster Expertenweg. Abweichungen gesondert darstellen,
keine ungeprüften Standards als gültige Satzung ausgeben. U04, U09.

## W06 – Abstandsflächen eines Hauses mit geneigtem Gelände

**Daten:** NRW-Haus, vier Außenwände, geneigtes Gelände, Dachanteil, angrenzendes
Fremdgrundstück; ein Sonderfall benötigt einen Beleg.

**Ablauf:** Nachweis anlegen → Rechts-/Höhen-/Koordinatenbezug → Gebäudeumriss →
Wände erzeugen → Höhen/Dach/Gelände je Abschnitt prüfen → Grundstücksgrenzen und
Flächen → Sicherungen/Sonderfälle → Prüfumfang bestätigen → rechnen → archivieren
→ PDF/gegebenenfalls DXF.

**Prüfung:** B: neuer Nachweis und Unterbereiche Grundlagen/Wände geöffnet;
Q: sechs Fachunterbereiche, Höhenübernahme, Wand-/Umrissabgleich und Exporte.
Kein neuer nativer Grenz-/Geländenachweis in diesem Audit.

**Reibung:** Viele Eingaben erfordern räumliches Verständnis und sind über
Unterbereiche verteilt. In einem §34-Testprojekt erscheint ohne festgesetztes
Baugebiet bereits WA als Startwert; das ist eine Vorgabe, keine bestätigte
projektbezogene Einordnung. Der einfache Tiefenrechner ist ein anderer Prüfumfang.

**Ziel:** Geometrieorientierter Assistent mit Plan und markierter aktiver Wand;
Rechtsgrundlagen/Bezug zuerst klären, besondere Fälle erst bedarfsgerecht öffnen.
Vorgaben als unbestätigt markieren; Teilprüfung und vollständigen Nachweis
sichtbar unterscheiden. U05, U09, U14.

## W07 – Amtliches Formular im Projekt ausfüllen

**Daten:** Baubeschreibungs-PDF mit Feldern und mehrzeiliger Anschrift; alternativ
neue Vorlage ohne Eingabefelder.

**Ablauf:** Projekt → Unterlagen → Formulare → Bibliotheksvorlage auswählen →
Projekt-/Bauherren-/Verfahrensbezug kontrollieren → Vorschläge prüfen und ausfüllen
→ Livevorschau → im Projekt speichern → Dokumentversion prüfen → Anforderung
zuordnen → Original-PDF einzeln oder im Stapel ausgeben.

**Prüfung:** B: leere Projektbibliothek und ihr Hinweis; Q: Formularausfüller mit
Livevorschau, Mehrzeilenfeldern, Vorschlägen und Speicherung. Die speziellen
Live-PDF-Tests wurden in diesem Audit nicht erneut ausgeführt.

**Reibung:** Leere/ungepflegte Bibliothek verweist textlich auf einen anderen
Arbeitsplatz. Erstellen und anschließende Zuordnung zur passenden Anforderung
sind unterschiedliche Schritte. Technische Feldnamen können sichtbar werden,
wenn eine Vorlage keine Beschriftung besitzt.

**Ziel:** „Formular für diese Anforderung erstellen“ übernimmt Kontext; nach
Speicherung konkrete Version zur Zuordnung anbieten. Direkter Bibliotheksweg mit
Rückkehr. Formulare selbst bleiben unverändert ohne künstlichen Briefbogen. U10, U11.

## W08 – Externe Plan-/Office-Datei korrigieren

**Daten:** Lageplan v1, DWG-Arbeitsdatei und Baubeschreibung.docx; Lageplan v1
ist bereits einem Abgabestand zugeordnet.

**Ablauf:** Projektdateien importieren/kategorisieren → Arbeitskopie mit passender
Anwendung bearbeiten → korrigierte Datei als neue Version desselben Dokuments
übernehmen → Version prüfen → Anforderungen/Nachweise prüfen → neue Abgabe.

**Prüfung:** Q: DMS/Arbeitskopien und Handbuch. B: bestehende Browserprüfung zeigt
zugeordnetes v1 neben aktuellem v2 und öffnet genau v1. Kein realer CAD-/Office-Start.

**Reibung:** Neue Datei und neue Version werden leicht verwechselt. Kategorie
allein genügt nicht, um eine identische Dokumentidentität festzulegen.

**Ziel:** Versionsvergleich mit Vorschlag „bestehendes Dokument fortschreiben“;
Nutzer bestätigt ausdrücklich. Keine Zuordnung nach Dateiname allein. U11, U12.

## W09 – Erstabgabe mit Anschreiben und Inhaltsverzeichnis

**Ablauf:** Verfahrensakte → Vorhabenprofil/Anforderungen → konkrete Dateien und
Nachweise zuordnen → Freigaben/Unterschriften → Einreichen/Nachreichen → Druck
prüfen → Abgabestand einfrieren → Portalbereitstellung → im externen Portal
absenden → Beleg importieren und tatsächliche Dateiversionen abgleichen.

**Prüfung:** B: Einstieg ohne Akte, gesperrte Portalübergabe und zugängliche
Formularvorbereitung; Druck-Artefaktprüfung prüft Vorschau/Export/DMS/Abbruch.
Q: Verfahrensakte, Portalbereich und Abgabestand. Keine echte Portalabgabe.

**Reibung:** Mehrere Einreichungseinstiege; eigenständige Druckauswahl ist nicht
automatisch identisch mit einem eingefrorenen Abgabestand. Ein gelungenes
Druck-PDF ist noch kein eingereichtes Paket.

**Ziel:** Ein verfahrensbezogener Abgabeworkflow mit einer expliziten Versionsliste
für Druck und Paket; Abweichungen vor Erstellung sichtbar. Portal und belegter
Eingang bleiben eigene Ereignisse. U03, U11, U13.

## W10 – Drei Nachforderungspunkte, zunächst zwei beantworten

**Ablauf:** Originalschreiben importieren → eingehende Kommunikation erfassen →
drei Punkte mit Zuständigkeiten/Fristen → korrigierte Berechnung und Plan v2 →
Antwortunterlagen zwei Punkten zuordnen → Antwortschreiben → Teilnachreichung
mit Vorgänger-/Ersatzbezug → tatsächlichen Eingang dokumentieren; dritter Punkt offen.

**Prüfung:** Q: Nachforderungen, Dokumentlisten, Verfahrensakte und Handbuch;
kein vollständiger neuer Nachreichungsdurchlauf im Browser.

**Reibung:** Dateiimport, Kommunikation, Forderung, Antwort, Dokumentversion und
Abgabe sind getrennte Bearbeitungsorte. „Antwort fertig“ darf nicht als
„behördlich erledigt“ verstanden werden.

**Ziel:** Arbeitsmappe pro Nachforderung mit Original, Einzelpunkten, Frist,
Antwortversion und Abgabebezug; Status des dritten Punktes bleibt offen. U11, U13, U15.

## W11 – Genehmigung mit Auflagen und Fristen

**Ablauf:** Originalbescheid/zugestellte Fassung importieren → Kommunikation →
Bescheid und Zustellung → genehmigte Dokumentversionen zuordnen → Bedingungen,
Auflagen und Hinweise unterscheiden → Verantwortliche/Fristen → Bauausführung
und Abschluss dokumentieren. Fehler über Berichtigung mit Begründung behandeln.

**Prüfung:** Q: Bescheide-/Fristenbereiche und Handbuch. Kein neuer Browserlauf
mit einem echten Bescheid; keine Prüfung der Fristberechnung im Audit.

**Reibung:** Rechtliche Wirkung, zuständiges Team und operative Aufgabe sind
verschiedene Informationen. Mehrere Stellen dürfen widersprechende
Erledigungsstände nicht suggerieren.

**Ziel:** Eine zuständige Aufgabenansicht verlinkt die unveränderte Quelle und
den Erfüllungsbeleg; Hinweise werden nicht als verpflichtende Aufgabe behandelt.
Zustellung ist von Dateidatum/Importdatum klar zu unterscheiden. U13, U15.

## W12 – Zwei Projekte parallel, Speicherfehler und schmale Fenster

**Ablauf:** A/B öffnen → getrennte Rechtsgrundlagen bearbeiten → zwischen Akten
und Modulen wechseln → Hintergrundakte schließen → Speichern schlägt fehl →
weiterbearbeiten → verzögert speichern → andere Akte bleibt aktiv → Anwendung
schließen/abbrechen → Lesemodus prüfen.

**Prüfung:** B: alle genannten Fälle durch `projekttabs.py`; Arbeitsplatzsuite
zusätzlich Hell/Dunkel und 100/125/150 Prozent CSS-Zoom. Eingaben bleiben erhalten,
genaue Dokumentversion wird geöffnet, Fehler des Arbeitschecks wird als nicht
auswertbar angezeigt. Kein Ersatz für native Windows-/macOS- oder Screenreaderprüfung.

**Reibung:** Entwurfsschutz ist vorhanden. Die verbleibende Frage ist vor allem,
welche Quelle gespeichert sein muss, bevor ein anderer Bereich sie verwenden kann.

**Ziel:** Im Verbraucher „Quelle hat Entwurf“ mit gezieltem Weg „Quelle speichern
und zurückkehren“; keine unbemerkte Gesamtspeicherung aller Projektentwürfe. U06, U14.
