# Entwurf 02: Die Projektakte

[Interaktiver Entwurf](../entwurf-02.html) · [Alle Vorschauen](vorschauen.html) · [Prüfprotokoll](pruefung.json)

Der zweite Entwurf orientiert sich an täglich genutzter Verwaltungs- und Fachsoftware. Eine helle Aktennavigation, eine feste Werkzeugleiste und Register strukturieren den Arbeitsplatz. Tabellen und beschriftete Felder bilden den Inhalt; feine Trennlinien gliedern die Abschnitte. Die Schrift ist kompakt, Zahlen stehen rechtsbündig, Farbe kennzeichnet Auswahl und Prüfergebnisse. Der Entwurf verzichtet auf große Kennzahlenkarten, dekorative Verläufe und stark gerundete Container.

## Gestaltungsentscheidungen

- **Projektakte:** Stammblatt, Planungsrecht, Grundstücke, Nachweise und Dokumente bleiben im selben Arbeitsrahmen. Globale Kunden- und Büroverwaltung sind separat erreichbar.
- **Grundstückseinheiten:** Auswahl links, Flächenansatz und Berechnungsnachweise rechts. Eigentumsfläche, Abzüge und maßgebende Bezugsfläche sind nachvollziehbar getrennt.
- **Datenarbeit:** Kompakte Tabellen, ruhige Streifen, klare Spaltenüberschriften und einfache Verzeichnisfilter. Projektlisten lassen sich nach Aktenzeichen oder Namen sortieren.
- **Verlässliche Bearbeitung:** Speichern und Prüfen sind explizite Aktionen. Ungespeicherte Änderungen lösen beim Ansichtswechsel eine Rückfrage aus; Escape erhält die Eingaben. Geänderte Flächen markieren Ergebnisse als veraltet.
- **Fester Arbeitsrahmen:** Akteninhalt und Navigation scrollen unabhängig; Register und Aktionsleiste bleiben erreichbar. Schmalere Fenster ordnen die Detailspalten untereinander an.

Das Konzept eignet sich besonders für längere Bearbeitungssitzungen am Desktop. Die Informationsdichte ist höher als im ersten Entwurf. Deshalb sind lesbare Beschriftungen, tastaturbedienbare Felder und eindeutige Zustände wesentlicher als dekorative Elemente.

## Mehrere Projekte gleichzeitig öffnen

Eine zusätzliche Tabzeile oberhalb der Projektakte zeigt alle geöffneten Projekte mit Aktenzeichen und Namen. Die darunterliegenden Register wechseln die Fachansicht innerhalb der aktiven Akte. Über das Projektverzeichnis können weitere Projekte geöffnet werden; ein bereits geöffnetes Projekt wird aktiviert statt dupliziert.

Jedes Projekt hält seine eigenen Flächeneingaben, Speicherwerte, Prüfergebnisaktualität, Grundstücksauswahl, Fachansicht, Dokumentfilter und Inhalts-Scrollposition. Beim Tabwechsel bleibt der Entwurf erhalten. Ein Punkt kennzeichnet ungespeicherte Änderungen auch in einem Hintergrundtab. Speichern und Prüfen beziehen sich auf die aktive Akte.

Beim Schließen einer geänderten Akte stehen Speichern, Verwerfen und Abbrechen zur Verfügung. Ein Hintergrundtab kann geschlossen werden, ohne den Entwurf der aktiven Akte zu verändern. Nach dem letzten geschlossenen Tab erscheint das Projektverzeichnis. Zuletzt gespeicherte Beispielwerte bleiben beim erneuten Öffnen innerhalb derselben Seitensitzung erhalten. Beim Neuladen der HTML-Seite werden die Beispieldaten zurückgesetzt.

[Tab-Vorschau](13-projekttabs.png) · [Prüfprotokoll für Projekttabs](projekttabs-pruefung.json)

## Umfang und Grenzen

Eigenständiger HTML-Prototyp mit lokalen Beispieldaten, ohne Backendverbindung. Alle zehn Ansichten sind erreichbar. Drei Beispielprojekte können parallel geöffnet werden; die weiteren Projekte verwenden ausdrücklich gekennzeichnete schematische Fachansichten. Grundstücksflächen, Suchfelder, Dokumentfilter, Sortierung und Änderungsdialog sind interaktiv. Andere Formularbereiche, CAD-Werkzeuge, Import und Export zeigen das Gestaltungskonzept; sie sind nicht fachlich implementiert. Beispielhafte Aktenzeichen, Bearbeitungsstände und Bearbeiter sind Vorschläge für spätere Produktfunktionen.

Die produktive Anwendung wurde für diesen Entwurf nicht verändert. BauNVO-Berechnungen im Prototyp verwenden ausschließlich die dargestellten Beispielansätze und ersetzen nicht die konfigurierten Regelwerke der Anwendung.

## Verifikation

Chromium mit Playwright: Navigation, Such- und Filterfunktionen, keine Öffnen-Aktion bei leerer Projektliste, Escape, Verwerfen auf zuletzt gespeicherte Werte, Strg+S, erneute Prüfung und unabhängige Grundstücksergebnisse. Keine JavaScript-Fehler. Bei 1024, 768 und 620 Pixel Fensterbreite kein horizontaler Seitenüberlauf; breite Tabellen scrollen in ihrem eigenen Bereich.
