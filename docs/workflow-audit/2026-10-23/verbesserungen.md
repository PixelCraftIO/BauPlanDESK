# Befunde und umsetzbare Quality-of-Life-Verbesserungen

Prioritäten: **P1** zuerst (blockierter Arbeitsfluss oder hohes Fehlbedienungsrisiko),
**P2** danach (wiederkehrender Aufwand), **P3** Komfort. Es wurde kein P0-Vorfall
mit reproduziertem Datenverlust festgestellt. Aufwand S/M/L ist eine relative
Einordnung, keine belastbare Termin- oder Kostenschätzung.

Die Quellenlinks zeigen auf den untersuchten Repositorystand; bei späteren
Änderungen die dortige Logik erneut prüfen. B/Q/H siehe [Methodik](README.md).

## U01 – §34 ohne BauNVO sperrt vorbereitende Nutzungsmodule

**P1 · M · B/Q · W01.** In `voraussetzung()` werden GRZ, GFZ, Geschossigkeit
und BMZ gemeinsam gesperrt, solange Rechtsgrundlage oder BauNVO fehlt. Der
§34-Browserfall reproduziert dies. Damit ist auch die vorbereitende Arbeit
in diesen Bereichen unzugänglich. Eine willkürliche BauNVO-Auswahl könnte als
vermeintliche Lösung erscheinen; tatsächliches Nutzerverhalten ist H.

**Verbesserung:** Erfassbarkeit, Berechenbarkeit und Nachweisreife unterscheiden.
Ein für das konkrete Vorhaben zulässiger Maßstab muss fachlich geklärt sein;
keine BauNVO aus §34 automatisch ableiten. Ungeklärte Fälle als Entwurf öffnen,
fehlende Voraussetzungen direkt im Modul mit Lösungsweg anzeigen.

**Abnahme:** §34-Projekt ohne BauNVO kann Entwurf speichern; verbindlicher Nachweis
bleibt bei ungeklärtem Maßstab blockiert. Für §30 bleibt die konkrete Fassung
erforderlich. Tastaturbedienung erreicht den Lösungsweg.

Quelle: [module.ts](../../../apps/desktop/src/lib/module.ts),
[Screenshot](belege/01-par34-navigation.png).

## U02 – Voraussetzungen stehen überwiegend am deaktivierten Navigationseintrag

**P1 · S · B/Q · W01/W02.** Die Sidebar verwendet `disabled` und `title`.
Ein deaktivierter Button ist mit Tab nicht regulär erreichbar. Die Voraussetzung
ist damit als alleiniger Tooltip schwer auffindbar. Planfestsetzungen werden
bei nicht erfüllter Voraussetzung vollständig ausgeblendet.

**Verbesserung:** Erklärungsseite oder fokussierbaren Infoweg anbieten:
„Noch nicht berechenbar: Bezugsfläche bestätigen → Baugrundstück öffnen“.
Nicht relevante Bereiche weiterhin reduzieren, deren Verfügbarkeit aber in
einer Bereichsübersicht erklären.

**Abnahme:** Fehlende Voraussetzung ist ohne Hover lesbar und führt gezielt zur
Quelle. Bestehender Entwurf bleibt erhalten. Fachlich irrelevante Bereiche werden
nicht als offene Pflichtaufgabe gezählt.

Quelle: [Sidebar](../../../apps/desktop/src/lib/Sidebar.svelte),
[Projektakte](../../../apps/desktop/src/lib/Projektakte.svelte).

## U03 – Unterschiedliche Aktionsketten bei Berechnungen

**P1 · M · B/Q · W02/W05/W09.** Grundstück/Nutzung besitzen Ermitteln,
Berechnen, Eingaben speichern, Nachweisstand und Speichern/Drucken. Fachrechnungen
besitzen zusätzlich eigene Register und Regelbestätigung. Der Druckbereich
benötigt gespeicherte bzw. archivierte Stände. Der Nutzer muss diese Unterschiede
lernen, obwohl er jeweils einen geprüften Nachweis erzeugen will.

**Verbesserung:** Gemeinsame Aktionsleiste mit „Entwurf speichern“, „Berechnen und
prüfen“ und „Nachweis erstellen …“. Letzteres zeigt notwendige Schritte und
verknüpft bestehende Modulaktionen; Freigabe bleibt eine gesonderte Entscheidung.
Einen Zwischenerfolg anzeigen, falls Archivierung gelingt, die PDF-Ausgabe aber
scheitert. Erneuter Versuch erzeugt keinen zweiten Nachweis.

**Abnahme:** Einheitliche Begriffe in allen drei Maskenfamilien; gespeicherter
Entwurf, aktuelles Ergebnis und archivierter Stand sind unterscheidbar.
Kein stilles Freigeben, Überschreiben oder ungeprüftes Fortschreiben alter Stände.

Quellen: [Baugrundstücke](../../../apps/desktop/src/lib/Baugrundstuecke.svelte),
[FachBerechnung](../../../apps/desktop/src/lib/berechnungen/FachBerechnung.svelte),
[Druckausgabe](../../../apps/desktop/src/lib/Druckausgabe.svelte).

## U04 – Regelbestätigung nach Eingabeänderung als zusätzlicher Umlauf

**P1 · M · Q · W04/W05.** Der Core setzt die Bestätigung bei Änderungen zurück.
Die UI erklärt „zuerst speichern, danach die Anwendbarkeit bestätigen“. Ein
einfaches gleichzeitiges Ändern und Ankreuzen erhält die Bestätigung nicht.
Ohne aufmerksames Lesen erscheint dies wie eine vergessene Eingabe.

**Verbesserung:** Nach Speichern einen gezielten Prüfschritt anbieten, der die
geänderten Grundlagen und die erforderliche Berechtigung zeigt. Regelpaketgültigkeit,
Eingabevollständigkeit und Nachweisfreigabe sprachlich unterscheiden. Eine weitere
Aufteilung der Bestätigungen benötigt zuerst einen fachlichen Vertrag.

**Abnahme:** Nach Datenänderung klare Meldung mit Grund und Aktion; Berechtigte
können die gesicherte Revision prüfen/bestätigen. Paralleländerungen werden
abgewiesen. Keine automatische Bestätigung eines geänderten Berechnungsstands.

Quellen: [FachBerechnung](../../../apps/desktop/src/lib/berechnungen/FachBerechnung.svelte),
[Core-Service](../../../crates/core-lib/src/berechnungen/service.rs).

## U05 – Abstandsflächen benötigen eine räumlich geführte Bearbeitung

**P1 · L · B/Q/H · W06.** Sechs Unterbereiche enthalten Regelprüfung, Umrisse,
Wände, Lageplan, Sicherungen und Prüfumfang. In der neuen Maske sind bereits
viele Bedienelemente gerendert; mehrere Aktionen befinden sich unterhalb des
ersten Bildausschnitts. Die Übertragung zwischen Umriss und Wand ist eine eigene
Aktion. Fachliche Komplexität wird bisher stark als Formularstruktur abgebildet.

**Verbesserung:** Planansicht mit aktiver Wand und daneben deren Eigenschaften.
Geführter Einstieg: Bezug/Regeln → Geometrie → Höhen/Gelände/Dach → Grenzen →
Sonderfälle → Prüfbericht. Fehlende Daten an der Wand markieren, Wandliste mit
Status, wiederkehrende Wandeigenschaften gezielt kopieren.

**Abnahme:** Vierwandhaus ohne Sonderfall bearbeitbar, ohne Sonderfallfelder
auszufüllen. Neigung, Höhenbezug und Außenseite sichtbar. Kopieren überträgt keine
fremden IDs, Freigaben oder Belege. Teilprüfungen bleiben erkennbar.

Quelle: [AbstandsProjekt](../../../apps/desktop/src/lib/abstandsflaechen/AbstandsProjekt.svelte),
[Sichtbeleg](belege/04-abstand-grundlagen.png).

## U06 – Gespeicherte Quelle, Entwurf und Übernahme sind nicht derselbe Stand

**P1 · M/L · B/Q · W02–W04/W12.** Verbraucher arbeiten sinnvollerweise mit
gespeicherten Quellen. Gebäude weist bereits auf ungespeicherte Flurstücke hin.
Plan-, Raum- und Nachweisübernahmen besitzen aber verschiedene Bedienwege.
Die Fachvorschau verwendet außerdem die gesamte Projektversion und geladene
Quellrevisionen in ihrer Signatur; dadurch kann sie konservativ auch bei
Änderungen außerhalb ihrer tatsächlich verwendeten Werte als veraltet erscheinen.
Das ist eine Aussage über die UI-Vorschau, nicht über den Core-Archivfingerprint.

**Verbesserung:** Einheitliches Quellenpanel: gespeicherter Stand, vorhandener
Entwurf, verwendeter Stand, Aktualisierungsbedarf, betroffene Verbraucher.
„Quelle öffnen und anschließend hier fortfahren“. Später Relevanzprüfung über
typisierte Abhängigkeiten verfeinern, statt pauschal alle Warnungen zu entfernen.

**Abnahme:** Verbraucher erklärt, warum neue Werte fehlen. Quelle speichern
verändert nur den ausgewählten Entwurf. Unbekannte Aktualität bleibt „nicht
auswertbar“. Keine stillen Aktualisierungen archivierter Nachweise.

Quellen: [Gebäude](../../../apps/desktop/src/lib/fachmodule/Gebaeude.svelte),
[FachBerechnung](../../../apps/desktop/src/lib/berechnungen/FachBerechnung.svelte),
[Quellenweg](../../../apps/desktop/src/lib/arbeitsplatz/Quellenweg.svelte).

## U07 – Vollgeschoss → Gebäude → GFZ wirkt wie eine Schleife

**P2 · M · Q/H · W03.** Gebäudedaten liefern die Geometrie für die
Vollgeschossprüfung. Die Einstufung wird aus deren Nachweis wieder am Geschoss
zugeordnet. Das ist fachlich unterscheidbar, benötigt aber mehrere Ortswechsel.

**Verbesserung:** Geschossbezogener Auftrag „Vollgeschossstatus prüfen“ mit
Quellrevision und Rücksprungziel. Danach ausdrückliche Übernahme in das Geschoss.

**Abnahme:** Gleiches Gebäude/Geschoss bei Rückkehr aktiv; keine automatische
Übernahme aus fremdem Geschoss oder veraltetem Stand. Die Liste notwendiger
Folgeprüfungen zeigt GFZ/Geschossigkeit. Keine zyklischen Modulaufrufe einführen.

Quelle: [Gebäude](../../../apps/desktop/src/lib/fachmodule/Gebaeude.svelte).

## U08 – Raumimport sollte Änderungen vor Übernahme erklären

**P2 · M · Q · W04.** Der Raumbuchimport gleicht anhand der Raum-ID ab,
erhält bestimmte Zuordnungen, lässt manuelle Positionen stehen und entfernt
entfallene Quellpositionen. Die zusammenfassende Meldung erscheint nach Ausführung.

**Verbesserung:** Vorheriger Vergleich mit Auswahl und Bestätigung; danach
Rückgängig bis zum Speichern. Tabellenmodus für viele Räume, Einfügen aus Excel
als spätere Erweiterung mit Einheiten-/Dezimalprüfung.

**Abnahme:** Neu/geändert/entfallen sichtbar. Gezieltes Aktualisieren erhält
manuelle Ergänzungen. Duplikate und fehlende IDs werden erklärt, niemals nach
Zeilenposition zusammengeführt. Excel-Import ist Vorschlag, keine vorhandene Funktion.

Quelle: [raumimport](../../../apps/desktop/src/lib/berechnungen/FachBerechnung.svelte).

## U09 – Vorschläge und bestätigte Rechtsgrundlagen stärker unterscheiden

**P1 · M · B/Q/H · W05/W06.** Neuer Abstandsplan verwendet bei fehlendem
festgesetztem Baugebiet `WA` als Startwert. Im geprüften §34-Projekt steht
dieser Wert bereits im Feld. Die vorhandene fachliche Bestätigung reduziert das
Risiko, erklärt aber nicht unmittelbar die Herkunft jeder Vorgabe. Kommunale
Satzungsfaktoren sind ebenfalls keine universellen Standards.

**Verbesserung:** Unbekannt als eigener Zustand oder „Vorschlag, noch zu prüfen“
am Feld. Versionierte, büroseitig geprüfte Regelpakete auswählen; projektbezogene
Geltung ausdrücklich bestätigen. Bearbeiten der Regeln in einen Expertenbereich.

**Abnahme:** Herkunft und Status jeder übernommenen Vorgabe sichtbar. Fehlende
Bestätigung erzeugt keinen positiven Nachweis. Keine ungeprüfte Annahme, §34
bedeute WA oder ein Gemeindefaktor gelte in allen Projekten.

Quellen: [AbstandsProjekt](../../../apps/desktop/src/lib/abstandsflaechen/AbstandsProjekt.svelte),
[Regelkontext](../../../apps/desktop/src/lib/berechnungen/FachBerechnung.svelte).

## U10 – Leere Formularbibliothek erzeugt einen Arbeitsunterbruch

**P2 · S/M · B/Q · W07.** Im Projekt erscheint bei leerer Bibliothek nur ein
Hinweis auf den globalen Bereich. Vorlagen ohne Felder haben deaktiviertes
„Ausfüllen“. Die Behebung findet außerhalb des Projekts statt.

**Verbesserung:** „Vorlage hinzufügen/einrichten“ mit Rückkehr zum Projekt und
zur Ausgangsanforderung, unter Berücksichtigung der Rechte. Für Vorlagen ohne
Felder „Eingabefelder einrichten“. Unverständliche PDF-Feldnamen im Vorlageneditor
mit sprechenden Bezeichnungen und Gruppen versehen; Originalname als Detail.

**Abnahme:** Leere Bibliothek und Vorlage ohne Felder bieten jeweils einen
konkreten Weg. Fehlende Rechte werden erklärt. Rückkehr wählt die neue Vorlage,
legt aber ohne ausdrückliches Speichern kein Projektdokument an.

Quellen: [ProjektFormulare](../../../apps/desktop/src/lib/ProjektFormulare.svelte),
[FormularAusfuellen](../../../apps/desktop/src/lib/FormularAusfuellen.svelte).

## U11 – Mehrere Unterlagenkontexte verlangen wiederholte Zuordnung

**P1 · L · B/Q/H · W07–W10.** Kategorie, Dokumentversion, Unterlagenregister,
Verfahrensanforderung, Druckauswahl und Abgabestand sind unterschiedliche Dinge.
Die konkrete Version wird bereits gut sichtbar gemacht; der Zusammenhang muss
aber häufig vom Nutzer hergestellt werden. Ein fertiges Formular ist noch nicht
automatisch die ausdrücklich gewählte Antwort auf eine Verfahrensanforderung.

**Verbesserung:** Verfahrensbezogene Unterlagen-Arbeitsliste mit Anforderung,
Kategorie, Dokument, zugeordnetem/aktuellem Stand, Prüfung, Unterschrift und
Abgabe. „Erstellen/Importieren/Neue Version“ direkt aus einer Anforderung.
Zuordnung nach Speicherung vorschlagen, bewusst bestätigen lassen.

**Abnahme:** Erstellung aus Anforderung kehrt dorthin zurück. Referenz bleibt auf
exakter Version. Neuere Projektversion ersetzt nie automatisch eine freigegebene
oder eingereichte Version. Druck und Paket zeigen Abweichungen ihrer Auswahl.

Quellen: [Unterlagen](../../../apps/desktop/src/lib/fachmodule/Unterlagen.svelte),
[EinreichenStart](../../../apps/desktop/src/lib/EinreichenStart.svelte),
[Druckausgabe](../../../apps/desktop/src/lib/Druckausgabe.svelte).

## U12 – Dokumentauswahl skaliert als einfaches Select schlecht

**P2 · M · Q/H · W08–W10.** Gemeinsame Dokumentfelder listen Dateiname und
Version in Selects. Kategorie-, Such- und Vorschaufunktionen fehlen in diesen
Auswahlkomponenten. Bei vielen Dokumenten/Versionen wächst die Auswahl schnell.
Fehlt Metadatenauflösung, verwendet Dokumentliste sogar die Dokument-ID als Fallback.

**Verbesserung:** Gemeinsamer durchsuchbarer Versionswähler mit Kategorie,
aktuell/archiviert, Version, Vorschau und Verwendungsnachweisen. Fehlende Metadaten
als „Dokument nicht auflösbar“ mit Diagnose statt nackter ID darstellen.

**Abnahme:** Mit 300 Dokumenten und mehreren Versionen gezielt auswählen; aktuelle
Version bevorzugt auffindbar, historische weiterhin gezielt verfügbar. UI-Seitenwechsel
verändert keine Auswahl. Gleicher Dateiname bleibt über eindeutige Identität getrennt.

Quellen: [Dokumentfeld](../../../apps/desktop/src/lib/verfahren/Dokumentfeld.svelte),
[Dokumentliste](../../../apps/desktop/src/lib/verfahren/Dokumentliste.svelte).

## U13 – Verfahrensakte und Einreichung besitzen zu viele konkurrierende Wege

**P1 · M/L · B/Q/H · W09–W11.** Die Verfahrensakte hat 14 Unterbereiche.
Daneben gibt es Einreichen/Nachreichen, Bauantrag/XBau, Portal, XBau-Nachrichten
und Einreichungen. Der geführte Einstieg hilft bereits, öffnet aber die getrennten
Bereiche. Das Handbuch muss ausdrücklich vor unverbundenen Doppelvorgängen warnen.

**Verbesserung:** Führende Verfahrensakte im Projektkontext sichtbar halten.
Unterbereiche nach Vorbereitung, laufendem Verfahren und Entscheidung gruppieren.
Einreichen/Nachreichen als zentralen Weg ausbauen; technische Werkzeuge als
Details erreichbar halten. Importierte Kommunikation kann einen Entwurf für
Nachforderung/Bescheid vorbereiten, niemals automatisch deren Rechtswirkung festlegen.

**Abnahme:** Verfahren, Empfänger und Abgabestand bei jedem Schritt sichtbar.
Rücksprung behält Kontext. PDF/ZIP-Export und Browseröffnung setzen keinen Versand-
oder Eingangsstatus. Altverfahren bleiben gezielt erreichbar.

Quellen: [Navigation](../../../apps/desktop/src/lib/navigation.ts),
[Verfahrensakte](../../../apps/desktop/src/lib/verfahren/Verfahrensakte.svelte),
[EinreichenStart](../../../apps/desktop/src/lib/EinreichenStart.svelte).

## U14 – Dichte Navigation und lange Aktionswege bei kleinen Bildschirmen

**P2 · S/M · B/H · W06/W12.** Projektkopf, Projektregister, Sidebar,
Eintragsaktionen, Speicherleiste und Fachregister stehen vor den eigentlichen
Feldern. Screenshots bei 1366 × 900 zeigen den Aufwand. Die vorhandenen
Zoom-/Layouttests zeigen keine geprüften Seitenüberläufe; das beweist noch keine
gute Tastaturreihenfolge oder komfortable Arbeit ohne Scrollen.

**Verbesserung:** Kompakter optionaler Projektkopf, haftende kontextbezogene
Aktionsleiste, sichtbarer Rückweg. Suche „Bereich oder Aktion öffnen“, Favoriten
und zuletzt bearbeitete Objekte. Tastenkürzel sichtbar erklären und Konflikte
mit nativen Menübefehlen vermeiden.

**Abnahme:** Tastatur erreicht Quelle → Feldfehler → Speichern → Bericht in
logischer Reihenfolge; Sticky-Bereiche verdecken kein fokussiertes Feld bei
150/200 Prozent. Native Zoom- und Screenreaderprüfung zusätzlich einplanen.

Quellen: [Sichtbeleg](belege/04-abstand-grundlagen.png),
[Fachliste](../../../apps/desktop/src/lib/Fachliste.svelte).

## U15 – Nachforderungen und Auflagen als konkrete Arbeitsaufträge bündeln

**P2 · M/L · Q/H · W10/W11.** Kommunikation, Nachforderungspunkte, Schreiben,
Abgabestände, Bescheide und Aufgaben haben jeweils eigene Bearbeitungsorte.
Das ist fachlich sinnvoll, kann aber organisatorisch zu mehrfach gepflegtem
Fortschritt führen. Dieser Mehraufwand wurde nicht mit echten Büroteams gemessen.

**Verbesserung:** Ein objektbezogener Arbeitsplatz zeigt Original, Wortlaut,
Frist, Zuständigkeit, Antwortentwurf, konkrete Unterlagen und Übermittlungsbeleg.
Aufgaben referenzieren den führenden Fachstatus, statt ihn frei zu duplizieren.
„Meine nächsten Schritte“ und Kalenderexport der vorhandenen Fristen integrieren.

**Abnahme:** Teilantwort auf zwei von drei Punkten lässt einen offen. Fertige
PDF bedeutet nicht eingereicht; Eingang bedeutet nicht behördlich erledigt.
Berichtigungen erhalten das Original. Hinweise ohne Pflicht bleiben Hinweise.

Quelle: [Verfahrensakte](../../../apps/desktop/src/lib/verfahren/Verfahrensakte.svelte),
[Handbuch](../../handbuch/Bedienhandbuch.md).

## Sinnvolle Umsetzungsreihenfolge

| Paket | Inhalte | Ergebnis für Planer | Modulgrenzen |
| --- | --- | --- | --- |
| 1. Arbeitsfähigkeit und klare Voraussetzungen | U01/U02/U09 | Frühe Erfassung ohne beliebige rechtliche Platzhalter; Vorgaben erkennbar | Core-Anwendbarkeit bleibt führend; UI-Erfassung separat |
| 2. Einheitlicher Nachweisweg | U03/U04/U06/U07 | Weniger Such-/Rücksprünge, klare Quelle und nächster Schritt | Bestehende Revisionen, Rechte, Idempotenz und Archivverträge verwenden |
| 3. Unterlagen bis zum Eingang | U10/U11/U12/U13 | Ein verfahrensbezogener Arbeitsfluss, genaue Versionen | DMS besitzt Dateien; Verfahren besitzt Anforderungen/Abgaben; Druck konsumiert exakte Auswahl |
| 4. Fachliche Masseneingabe | U05/U08 | Planorientierte Wände, Raumvergleich und Tabellenarbeit | Geometrie-/Importvorschau getrennt von bestätigtem Fachstand |
| 5. Alltag und Zusammenarbeit | U14/U15 | Kürzere Wege, Zuständigkeiten, offene Punkte | Ein führender Status je Fachobjekt, Aufgaben referenzieren ihn |

Kleine Verbesserungen aus späteren Paketen können vorgezogen werden, insbesondere
Bibliotheks-Rücksprung, sprechende Feldnamen und sichtbare Voraussetzungen.
Das bestehende modulare System muss dafür nicht vollständig neu geschrieben werden.

## Validierung mit echten Planern

Mit mindestens drei unterschiedlichen Rollen testen: Entwurfsverfasser,
Projektassistenz und prüfende Projektleitung. Jeweils W01, W04, W09 und W10
ohne vorgegebene Klickfolge bearbeiten lassen. Erheben: erfolgreicher Abschluss,
falsche Version/Verfahrenszuordnung, Rücksprünge, Hilfebedarf und Zeit pro Aufgabe.
Zuerst Ausgangswert messen, dann denselben fachlichen Fall mit alternativen Daten
im neuen Ablauf. Die genannten Maßnahmen sind bis dahin begründete Vorschläge,
keine empirisch bewiesenen Produktivitätssteigerungen.
