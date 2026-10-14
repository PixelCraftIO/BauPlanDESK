# Umsetzungsvorschlag

Diese Arbeitspakete sind ein Entwicklungsplan, keine bereits vorgenommenen
Produktänderungen. Größenangaben sind relativ (S/M/L), keine zugesagten Termine.

| Paket | Größe | Inhalt | Abhängigkeiten | Abnahme |
| --- | --- | --- | --- | --- |
| A · Bearbeitungssicherheit | S–M | Projektfilterauswahl, zentrale Schließprüfung einschließlich Bauantrag, veraltete Abstandsflächenergebnisse, sichtbare Labels, Dark-Mode-Kontraste | Keine neue Navigation nötig | Kein Öffnen unsichtbarer Treffer; Esc verliert keinen Entwurf; Kontrastprüfung für beide Themes; Keyboardabläufe reproduzierbar |
| B · UI-Grundsystem | M | Tokens für Farben, Typografie, Abstände; Button, Field, Section, Table, Notice, Dialog, ActionBar, StatusBadge | A | Neue Komponenten zeigen Laden/Leer/Fehler/Schreibschutz/Fokus; globales `form`/`section`-Layout schrittweise begrenzt |
| C · Verwaltungsgerüst | M | Seitenleiste, Projektliste, Projektkopf, lokale Abschnittsnavigation, Verwaltungseinstieg | B | Globale und Projektfunktionen klar getrennt; native Menüs/Shortcuts behalten; Wechsel erhält Entwürfe |
| D · Listen und Tabellen | M | Projekte, Dokumente, Historie, Protokoll, Benutzer; Suche, Sortierung, Aktionen und Zahlendarstellung | B/C | Bedienbar mit Tastatur; Filter/Sortierung erhalten Auswahlkonsistenz; 1024 px ohne Seitenoverflow; jede bisherige Aktion erreichbar |
| E · Grundstücke/Nachweise | L | Grundstücksliste, Detailbereiche, zentrale Speichern-/Prüfen-Leiste, Ergebnisquelle und Aktualität | B/C | Vorhandene GRZ/GFZ/BMZ-Ergebnisse identisch; je Einheit getrennt; alte Eingaben lesbar; Grundbuchänderungen verschieben keine Berechnung unbemerkt |
| F · Bauantrag | L | Abschnittsnavigation, klickbare Pflichtfeldhinweise, Quelle/Abweichung der Grundstücks- und Kundenangaben | B/C/E; fachliche Mappingentscheidung | Definierte Übernahme ohne Doppelpflege; keine stille Überschreibung bewusst abweichender Angaben; XBau-Schema-/Exporttests grün |
| G · Kunden/Verwaltung | M | Dauerhafte Listen-/Detailseiten; einheitliche Speicherung, Büro, Lizenz | B/C | Bestehende Rollen und Berechtigungen erhalten; Kundenzuordnung und Kundenlöschung nicht verwechselbar; sensible Aktionen haben verständliche Rückmeldung |
| H · Dokument-/Planarbeitsplatz | L | PDF-Vorschau/Feldgruppen; CAD-Werkzeuge, Inspector, Versionen, Legende | B/C/D | PDF-Felder und Original abgleichbar; Messwerkzeuge/Shortcuts unverändert; gespeicherte CAD-Geometrie und DMS-Versionen korrekt |

## UI oder Datenmodell?

Direkt im Frontend umsetzbar: Typografie, Abstände, Navigation, Sections,
Aktionenleiste, lokale Sortierung/Filterung kleiner Datenmengen, Zustandsanzeige,
Dialogregeln, bessere Feldbeschriftung und responsives Layout.

Mit fachlicher bzw. Backend-Abstimmung: Projektstatus/Projektnummer,
Dokumentkategorien, detailliertere Protokolländerungen, Übernahme von
Grundstückszuordnung in XBau, verbindliche Prüfung von Datenvollständigkeit.
Projektnummer und Bearbeitungsstand im Entwurf sind Beispiele solcher Erweiterungen.

Mit technischer Untersuchung: serverseitige Pagination großer Listen,
PDF-Vorschau einschließlich Formularpositionen, Speicherung eigener Layoutpräferenzen.
Keine Produktentscheidung für eine zusätzliche UI-Bibliothek ist für die ersten
Pakete nötig. Svelte 5 und gemeinsame CSS-/Komponentenbausteine reichen zunächst.

## Gemeinsame Abnahme

* 1440×900, 1366×768, 1024×768, 200-%-Zoom; beide Themes, aktive Schreib-/Leserrolle.
* Fachtexte, Codes, Dateinamen und lange Projektnamen bleiben lesbar/kopierbar.
* Keine unbemerkte Verwerfung, kein schreibender Request bei Leserrechten,
  klare Reaktion auf Versionskonflikte.
* Eingaben und Ergebnisaktualität beim Bereichs-/Projektwechsel kontrollieren.
* Zahlen und rechtliche Bewertung bleiben unverändert; keine Grünfärbung fehlender
  Festsetzungen oder nicht geprüfter Daten.
* Tastatur und Fokus für Menü, Dialog, Filter, Tabelle und CAD prüfen.
* Mit realistischen Datenmengen (z. B. 50 Projekte, 200 Dokumente, 10 Grundstücke)
  nutzerorientierte Aufgaben durchspielen; Performance erst danach bewerten.
* Relevante vorhandene Rust-/XBau-Tests, Svelte-Check und Build erhalten. UI-Tests
  prüfen konkrete Bedienfehler, nicht bloß das Vorhandensein von CSS-Klassen.


## Ergänzung: mehrere offene Projektakten

Im zweiten Prototyp umgesetzt; für die produktive Anwendung als eigener Arbeitsschritt einplanen. Dafür muss der bisherige Zustand eines einzelnen aktiven Projekts in getrennte Projektsitzungen aufgeteilt werden. Die Tabzeile zeigt offene Projekte; Fachregister arbeiten innerhalb der aktiven Sitzung.

Abnahme für die Anwendung:

* Je Projekt getrennte Eingaben, Dirty-Flags, Berechnungsergebnisse und Ergebnisaktualität; letzter Fachbereich, Filter und Scrollposition bleiben erhalten.
* Projektwechsel erhält Entwürfe ohne Schließdialog. Bereits offene Projekte werden aktiviert, nicht erneut geöffnet.
* Speichern, CAD-Aktionen, DMS- und XBau-Requests verwenden die Projekt-ID der Sitzung, in der die Aktion ausgelöst wurde. Auch später eintreffende Antworten dürfen kein anderes Projekt verändern.
* Schließen bietet bei ungespeicherten Daten Speichern, Verwerfen und Abbrechen. Fehlgeschlagenes Speichern lässt die Akte offen; Hintergrundtabs verändern die aktive Akte nicht.
* Dokument- und Plansperren, laufende Requests, PDF-/CAD-Ressourcen und Ereignislistener werden beim Schließen der jeweiligen Sitzung gezielt freigegeben.
* Nach dem Schließen einer aktiven Akte wird die Nachbarakte aktiviert; nach der letzten erscheint das Verzeichnis. Tabs bleiben per Tastatur erreichbar und bei vielen/langen Titeln innerhalb der Tabzeile scrollbar.

Eine Wiederherstellung ungespeicherter Entwürfe nach einem Neustart ist eine separate Funktion; der HTML-Prototyp hält diese Daten ausschließlich während seiner Seitensitzung.
