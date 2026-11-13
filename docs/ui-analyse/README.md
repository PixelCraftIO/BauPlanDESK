# UI-Analyse und Zielbild für BauPlanDESK

**Empfehlung:** Die Anwendung sollte ein einheitliches Verwaltungsgerüst mit
Projektliste, dauerhafter Navigation, übersichtlichen Datentabellen und
abschnittsweiser Bearbeitung erhalten. Die größte Verbesserung entsteht durch
klarere Arbeitsabläufe und eine konsistente Informationsstruktur. Farben und
Abstände unterstützen diese Struktur.

[Interaktiven Gestaltungsentwurf öffnen](prototyp.html) ·
[Ist-/Entwurfsvergleich öffnen](vergleich.html) ·
[Umsetzungsplan](umsetzung.md)

**Alternative Gestaltung:** [Entwurf 02 – klassische Projektakte](entwurf-02.html) ·
[Vorschauen aller Bereiche](entwurf-02/vorschauen.html) ·
[Gestaltungsentscheidungen und Prüfumfang](entwurf-02/README.md)

**Modernere Variante:** [Entwurf 03 – moderne Projektakte](entwurf-03.html) ·
[Vorschauen](entwurf-03/vorschauen.html) ·
[Gestaltung und Prüfumfang](entwurf-03/README.md)

## Umfang und Stand

Basis: `main`, Commit `8b39a60` (PR #11), Analyse vom 14.10.2026.
Alle 25 Svelte-Bereichskomponenten sowie App-Navigation, Dialoge und globale
Styles wurden gelesen. 22 neue Aufnahmen der nativen Anwendung dokumentieren
Projektbereiche, Verwaltung, Dialoge, Bauherren, PDF-Formulare, Dokumenthistorie
und 1024-Pixel-Fensterbreite.

Die Analyse arbeitet mit vorhandenen Testprojekten, drei Flurstücken, zwei
Grundstückseinheiten, vier Dokumenten und einem Kunden. Verhalten mit hunderten
Datensätzen ist eine offene Belastungsprüfung, keine hier durchgeführte Messung.
Dunkelmodus-Kontraste wurden aus den CSS-Farbwerten berechnet. Ein vollständiger
Screenreader-/WCAG-Test der produktiven Anwendung wurde nicht durchgeführt.

Der HTML-Entwurf demonstriert Navigation, Filter, Datendichte und eine
Grundstücks-Detailbearbeitung. Seine Daten und Statuswerte sind Beispiele. Er
ersetzt keine produktive Oberfläche und ruft keine Anwendungs-API auf.
Projektnummern, Workflowstatus, Dokumentkategorien und Dokumentvorschau sind
Gestaltungsvorschläge; die heutige Anwendung besitzt diese Funktionen nicht
alle in der gezeigten Form. Die 820 m² auf der Beispielübersicht sind eine
Informationssumme; alle Kennzahlprüfungen bleiben je Grundstück getrennt.

## Was bereits als Grundlage trägt

Die Anwendung hat verständliche Fachbereiche, zentrale Kundenstammdaten,
Projekt-/Benutzerrechte und Dokumentversionen. Die neuen Rechtsgrundlagen,
BauNVO-Ansätze und Einzelgrundstücksnachweise liefern eine gute fachliche Basis.
Geänderte Nutzungseingaben blenden bereits alte Ergebnisse aus; diese Funktion
ist nach PR #11 vorhanden und wird hier nicht erneut als offener Fehler geführt.
JSON-Konfigurationen und der gemeinsame Rechenkern sollten bei der
UI-Weiterentwicklung erhalten bleiben.

## Priorisierte Befunde

P1 = beeinträchtigt sichere oder effiziente Bearbeitung.
P2 = erschwert den Arbeitsfluss und die professionelle Darstellung.
P3 = Ausbau für größere Datenbestände bzw. zusätzliche Arbeitsabläufe.
„Bestätigt“ bezeichnet sichtbares Verhalten oder einen eindeutigen Codepfad.
„Risiko“ bezeichnet noch ausstehende Prüfung mit realistischen Datenmengen.

| ID | Priorität | Befund und Beleg | Konsequenz | Zieländerung |
| --- | --- | --- | --- | --- |
| U01 | P1 | **Bestätigt:** Projektfilter zeigt keine Treffer, „Öffnen“ bleibt aktiv. `ProjektOeffnen.svelte:19–30,58`; [Screenshot](ist/08-projektfilter-fehler.png). | Ausgewählte ID kann außerhalb der gefilterten Liste liegen; falsches Projekt kann geöffnet werden. | Auswahl nach jedem Filterwechsel abgleichen; Öffnen nur für sichtbaren Treffer zulassen. |
| U02 | P1 | **Bestätigter Codepfad, im vorherigen Review reproduziert:** Escape schließt Kunden-Dialog über `Dialog.svelte:20` und `App.svelte:412`; die Verwerfprüfung in `Kunden.svelte:64,141` wird dabei umgangen. | Unbeabsichtigter Verlust nicht gespeicherter Kundeneingaben. | Gemeinsame asynchrone Schließprüfung für Escape, Abbrechen, Wechsel und Schließen. Vor Verlassen konkreten Entwurf benennen. |
| U03 | P1 | **Bestätigt:** Grundstücksrechnung hat drei Flurstücke; Bauantrag verlangt trotzdem Gemarkung und Flurstücksnummer. [Bauantrag](ist/05-bauantrag.png); getrennte Modelle in `Flurstuecke.svelte` und `Bauantrag.svelte:293–310`. | Doppelpflege und widersprüchliche Antragsdaten möglich. | Übernahme bzw. Abgleich aus Projektgrundstücken mit eindeutiger Herkunft. Explizite Abweichungen im Antrag zulassen und markieren. Fachliche Zuordnung vorher festlegen. |
| U04 | P1 | **Bestätigt am Code:** Abstandsflächen bieten unlabeled BY-/WA-Auswahllisten und „H“. `Abstandsflaeche.svelte:35–45`; [Screenshot](ist/04-abstandsflaechen.png). | Fachliche Bedeutung und Zugang per Assistenztechnik bleiben unklar. | Sichtbare Feldtitel: Bundesland, Baugebiet, maßgebende Wandhöhe; Einheiten und Quelle am Ergebnis. |
| U05 | P1 | **Aus CSS berechnet:** Weiß auf dunklem Modus-Akzent `#7aa7f5` hat 2,42:1; Fehlerrot `#c62828` auf dunklem Panel 2,87:1. `app.css:17–39,75`. | Unzureichender Textkontrast; kleiner normaler Text braucht typischerweise 4,5:1. | Eigene Button-/Text-/Statusfarben pro Theme; Palette gegen WCAG-Kontraste prüfen. Kein AA-Zertifizierungsanspruch dieser Analyse. |
| U06 | P2 | **Bestätigt:** Startbildschirm besteht aus zwei kleinen Karten und viel freier Fläche; Projekte werden überwiegend über einen Öffnen-Dialog gefunden. [Start](ist/18-start.png). | Anwendung wirkt wie ein Werkzeugsatz statt Projektverwaltung. | Projektliste als regulärer Arbeitsbereich, zuletzt verwendete Projekte ergänzend. Such-/Sortier-/Filterleiste und eindeutige Projektinformationen. |
| U07 | P2 | **Bestätigt:** Projektbereiche als lange Tabzeile, globale Kunden/Büro/Benutzer/Lizenz über Menü bzw. Statusleiste. `App.svelte:279–283,386–421`. | Globale und projektbezogene Funktionen sind unterschiedlich auffindbar. | Globale Seitenleiste für Projekte, Kunden, Verwaltung; projektbezogene Bereiche separat. Native Menübefehle und Shortcuts behalten. |
| U08 | P2 | **Bestätigt:** Flurstücke, Abzüge, Grundstücke, Bauteile, Geschosse und Nachweise werden in einer langen verschachtelten Seite bearbeitet. [Formular](ist/01-flurstuecke.png). | Kontext und Speichern liegen weit auseinander; viele Rahmen konkurrieren miteinander. | Grundstücksliste mit Detailbereich, klaren Unterbereichen und beständiger Aktionsleiste. Eingaben beim Bereichswechsel erhalten. |
| U09 | P2 | **Bestätigt:** Dokumentzeilen haben bis zu sechs Linkaktionen; bei 1024 px entstehen starke Umbrüche. [Desktop](ist/09-dokumente.png), [1024 px](ist/16-dokumente-1024.png). | Schwieriges Scannen, wechselnde Zeilenhöhen und wenig Raum für Metadaten. | Hauptaktion, Versionszugang und Aktionenmenü; Suche/Filter, Zahlenspalten, klarer Dateityp. Weitere Aktionen bleiben vollständig erreichbar. |
| U10 | P2 | **Bestätigt am Code:** Generische globale `form`, `section`, `li`-Styles; wiederholte Überschreibungen von `main`/`.hinweisbox`; einzelner fester Feldrahmen `#cbd5e1`. `app.css`. | Neue Bereiche übernehmen unpassende Layouts; Theme und Hierarchie driften auseinander. | Design-Tokens und abgegrenzte Komponenten statt globaler Layoutvorgaben. Frames nur für eigenständige Inhalte. |
| U11 | P2 | **Bestätigt:** Zahlreiche Erklärungen im Arbeitsbereich und technische Angaben im Footer: Modus, Ablage, Regelpakete. `App.svelte:386–393`. | Fachliche Aktionen konkurrieren mit Implementierungsdetails. | Fachlich relevanter Kontext sichtbar; technische Informationen unter Verwaltung/Diagnose. Hilfen pro Feld bzw. Abschnitt. |
| U12 | P2 | **Bestätigt:** Änderungen in Benutzerzeilen werden per `onchange` sofort gespeichert, andere Bereiche haben explizites Speichern. `Benutzerverwaltung.svelte:36–48,68–83`. | Unterschiedliche Erwartung an Speicherung, besonders bei Rollen/Aktivierung. | Einheitliches Muster oder ausdrücklich beschriftete Sofortspeicherung; sensible Rollenänderung mit klarer Rückmeldung. |
| U13 | P2 | **Bestätigt am Code:** `user-select:none` auf Root. `app.css:14`. | Namen, Kennungen und Ergebnisse lassen sich außerhalb von Inputs schlecht kopieren. | Textauswahl für Sachinformationen zulassen; Auswahl nur in Toolbar-/Navigationssteuerung beschränken. |
| U14 | P2 | **Bestätigt:** Gemeinsames Dialogelement bietet Titel und Inhalt, aber keinen einheitlichen Kopf-/Fußbereich oder Fensterhöhe. `Dialog.svelte`; Spezialbreiten über `:has`. | Unterschiedliche Dialogwirkung, mögliche Probleme bei langen Inhalten. | Kleine Dialoge für kurze Aufgaben; längere Verwaltung als Seite. Maximalhöhe, interner Scrollbereich, fixe Aktionen, konsistenter Fokus. |
| U15 | P3 | **Risiko:** Keine durchgängigen Sortier-/Filter-/Seitengrößenmuster in Projekt-, Dokument-, Protokoll- und Benutzerlisten. | Bei vielen Datensätzen fehlen Auffindbarkeit und skalierbare Übersicht. | Sortierung und Filter aus gemeinsamen Tabellenkomponenten; serverseitige Pagination/Virtualisierung erst nach Datenmengenprüfung festlegen. |
| U16 | P2 | **Bestätigt:** PDF-Formular hat Felderliste, aber keine räumliche Dokumentvorschau; Planbearbeitung besitzt eigene Canvas-Werkzeuge. | Übertragung von Formulardaten ist schwer mit dem Original abzugleichen. | PDF-Vorschau neben Feldern; CAD separat als fokussierter Arbeitsbereich mit Inspector. PDF-Vorschau ist funktionaler Ausbau, kein reiner CSS-Wechsel. |
| U17 | P1 | **Bestätigter Codepfad:** Projektwechsel/-schließen fragt Plan-, Nutzungs- und Rechtseditor, aber nicht den Bauantragsentwurf ab. `App.svelte:87–109`; `Bauantrag.svelte` hat eigenen Änderungszustand. | Nicht gespeicherte Antragsänderungen können beim Projektwechsel verloren gehen. | Bauantrag in eine gemeinsame Entwurfs-/Schließkoordination aufnehmen; mit geändertem Antrag und Ctrl+W/Projektwechsel prüfen. |
| U18 | P1 | **Nativ reproduziert:** H nach Berechnen von 6,5 auf 10 m geändert; der alte Abstandsflächenwert bleibt sichtbar. [Screenshot](ist/22-abstand-veraltetes-ergebnis.png); `Abstandsflaeche.svelte:20–29,48–54`. | Ergebnis kann zur falschen Eingabe gelesen werden. | Ergebnis an Eingabestand und Regelpaket binden; nach Änderung ausblenden oder ausdrücklich als veraltet markieren. |

## Bereichsweise Entwicklung

| Bereich | Empfohlene Entwicklung |
| --- | --- |
| Start und Projekt öffnen | Reguläre Projektliste; Suchzustand, gültige Auswahl, klare Leerzustände. Keine künstlichen Kennzahlen ohne Datenquelle. |
| Projektübersicht | Identität, Bauherr, Rechtsgrundlage, Grundstückseinheiten, Bearbeitungs-/Prüfstand und letzte Aktivitäten. Grundstückssummen niemals als gemeinsamen Zulässigkeitsnachweis ausgeben. |
| Rechtsgrundlagen | Grunddaten → Plan → Festsetzungen → Besonderheiten. Gesetzes-/Planbezug direkt beim Feld. Öffentliche Freitexte nicht als maschinengeprüfte Festsetzung darstellen. |
| Grundstücke und Maß der Nutzung | Liste der Einheiten links, Details rechts. Unterbereiche Flurstücke/Abzüge, Bauteile/Geschosse, Nachweise. Alle Grundstücke berechnen weiterhin als zusätzliche Aktion. |
| Abstandsflächen | Ein vollständiges Eingabe-/Ergebnisformular mit Feldtiteln, Einheiten, Quelle und Varianten. Verfügbare Landesregelpakete sichtbar, fehlende Daten nicht verstecken. |
| Dokumente | Fachliche Liste, Suche, Filter, aktuelle Version, Bearbeiter, Sperrstatus. Eine Hauptaktion pro Zeile; weitere Aktionen im Menü bzw. Inspector. |
| Dokumenthistorie | Versionsliste mit Zeitpunkt/Autor/Grund, aktuelle Version deutlich. Wiederherstellen als neue Version weiterhin bestätigen. |
| PDF-Formulare | Quelle/erkanntes Formular, gruppierte Felder, Vorschau und Herkunft übernommener Werte. Leere, unveränderte, vorgeschlagene und schreibgeschützte Felder unterscheiden. |
| Pläne/CAD | Canvas bekommt den größten Platz; Werkzeuge links/oben, Objekte und Prüfergebnisse im Inspector. Maße/Layer als einklappbare Bereiche, Legende und Versionsbezug sichtbar. |
| Bauantrag/XBau | Abschnittsnavigation für Vorhaben, Beteiligte, Grundstücke, Anlagen, Prüfen/Export. Fehlende Angaben führen zum betreffenden Feld; Quelle der übernommenen Projektdaten sichtbar. |
| Kunden | Regulärer Listen-/Detailbereich statt umfangreicher Modalverwaltung. Name, Kategorie, Ort, Projekte und Kontakt. Kundenstamm und Bauherrnzuordnung bleiben getrennte Aufgaben. |
| Bauherren | Kleine Zuordnungsaufgabe innerhalb des Projekts; Datenpflege über Kundenbereich. Zuordnung darf nicht wie Löschen des Kunden aussehen. |
| Projektprotokoll | Zeit, Benutzer, Aktion, Objekt; Filter. Fachlich spezifische Änderungsdetails erfordern ggf. Ausbau der Protokolldaten. |
| Benutzer und Rollen | Verwaltungsseite mit klarer Rolle, Aktivstatus und Speichermuster. Rollenrechte nicht nur über Tooltip erklären. |
| Büro | Verwaltungsformular mit Absenderdaten, XBau-Kennung und verständlichen Validierungen. |
| Lizenz | Übersichtsstatus zuerst, Arbeitsplätze und Aktionen danach. Keine Aktivierungs-/Abmeldeaktion als alltägliche Projektaktion. |
| Projekt anlegen/bearbeiten | Kurzer Dialog für Kerndaten; nach Anlegen in Projektbearbeitung führen. Fehlende fachliche Daten dürfen als Entwurf sichtbar sein. |
| Dialoge und Statusmeldungen | Einheitliche Fokus-/Escape-Regeln, Fehlermeldung am Feld, Ladezustand, Leerzustand, Erfolg, Schreibschutz und Versionskonflikt. |

## Gestaltungsregeln

Eine ruhige Desktop-Verwaltungsoberfläche: heller neutraler Arbeitsbereich,
weiße Inhaltsflächen, dunkelblaue Navigation, eine primäre Aktionsfarbe.
Keine dekorativen Verläufe, großflächigen Hero-Bereiche oder unnötigen Animationen.
Statusfarben werden mit Text kombiniert. „Vergleichswert eingehalten“ bedeutet
nicht „Vorhaben genehmigungsfähig“.

| Element | Zielregel |
| --- | --- |
| Typografie | Systemschrift; 14 px Grundtext, 12–13 px Labels/Metadaten, 18–24 px Seitentitel. Mindestens 12 px für dauerhafte Sachinformationen. |
| Abstände | 4-/8-px-System; 16–24 px zwischen Bereichen, 8 px zwischen Aktionen. |
| Eingaben | Einheitliche 36-px-Höhe im Standardmodus. Sichtbare Titel statt bloßer Platzhalter; Einheit immer erkennbar. |
| Tabellen | Standardzeile ca. 44–48 px, kompakte Variante ca. 36–40 px. Zahlen rechtsbündig, tabellarische Ziffern; stabile Kopfzeile und begrenztes Tabellen-Scrolling. |
| Buttons | Eine klare Hauptaktion je Bereich, sekundäre Aktion als Outline, destruktive Aktion eindeutig. Nicht alle Aktionen gleich blau darstellen. |
| Zustände | Nicht geprüft / veraltet / eingehalten / Abweichung / nicht festgelegt ausdrücklich unterscheiden. Änderung macht gespeichertes Ergebnis ungültig. |
| Responsivität | Primär 1440×900 und 1366×768; zusätzlich 1024×768. Kleinere Fenster: Inspector unter Inhalt, Navigation verdichten, Tabellen lokal scrollen. |
| Tastatur | Sichtbarer Fokus, logische Reihenfolge, Esc nur über Schließprüfung. Listen vollständig mit Tastatur bedienen; keine verdeckte Auswahl. |
| Barrierefreiheit | Labels, verständliche Namen, Statusmeldungen, Textkontraste, Fokus und 200-%-Zoom prüfen. Native Desktopumgebung und Webview separat betrachten. |
| Sprache | Fachlich klar: „Grundstücksfläche“, „maßgebende Bezugsfläche“, „Flächenansatz“, „Prüfergebnis“. Kürzel durch Namen und Hilfen ergänzen. |

## Umsetzung mit überschaubaren Risiken

1. Bearbeitungssicherheit herstellen: Filterauswahl, Verwerfprüfung und Kontraste.
2. Gemeinsame UI-Bausteine schaffen, zunächst Projektliste und Dokumente umstellen.
3. Projektgerüst mit globaler Navigation und übersichtlicher Projektseite einführen.
4. Grundstücke und Berechnungen in Liste/Detail aufteilen, Entwürfe erhalten.
5. Bauantrag mit Abschnittsnavigation und Datenübernahme überarbeiten.
6. Kunden/Verwaltung konsistent umstellen, PDF-/CAD-Details optimieren.

Ein vollständiges Rewriting der Oberfläche ist dafür nicht erforderlich.
Die funktionierenden Fachkomponenten und Berechnungen können schrittweise
in das neue Gerüst übernommen werden. Die Abnahmebedingungen und Abhängigkeiten
stehen im [Umsetzungsplan](umsetzung.md).

## Prüfung des Gestaltungsentwurfs

Chromium: Projektfilter einschließlich leerer Trefferliste, Bereichsnavigation,
Grundstückswechsel, lokale Flächenänderung, Escape/Verwerfen, Beibehaltung des
zuletzt lokal gespeicherten Beispielstands, getrennte Grundstücks-Überschreitungen
und Dokumentfilter wurden geprüft. Keine JavaScriptfehler. Bei 1024 und 768 px
keine horizontale Überbreite der gesamten Seite; breite Tabellen scrollen lokal.
Ergebnis: [prototyp-pruefung.json](prototyp-pruefung.json).

Diese Prüfung betrifft den eigenständigen Entwurf. Sie ist keine Aussage darüber,
dass die heutige Anwendung diese Funktionen bereits besitzt. Für diesen
Analyseauftrag wurden Anwendungscode und Fachlogik nicht verändert.
