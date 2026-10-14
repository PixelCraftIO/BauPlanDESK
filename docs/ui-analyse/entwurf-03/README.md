# Entwurf 03: Moderne Projektakte

[Interaktiver Entwurf](../entwurf-03.html) · [Alle Vorschauen](vorschauen.html) · [Klassische Variante zum Vergleich](../entwurf-02.html)

Diese Variante entwickelt die Projektakte aus Entwurf 02 optisch weiter. Struktur und Bedienung bleiben erhalten: mehrere offene Projektakten, Fachregister, Grundstücksliste mit Detailbereich, Tabellen sowie explizites Speichern und Prüfen.

Die Gestaltung verwendet klarere Schriftgrößen, etwas mehr Zeilenabstand, weiße und leicht getönte Arbeitsflächen, feinere Trennlinien und flache Register mit einer Unterstreichung. Werkzeugaktionen sind dezent dargestellt; Formularfelder und einzelne Bedienelemente haben kleine Radien. Auswahl und Prüfergebnisse setzen gezielte Farbakzente. Große Kennzahlenkarten und dekorative Container bleiben entbehrlich.

Die bisherige klassische Variante bleibt separat verfügbar. Beide Prototypen sind eigenständige HTML-Dateien ohne Backendverbindung und verwenden lokale Beispieldaten. Die produktive Anwendung wurde nicht verändert. Andere Projekte verwenden ausdrücklich gekennzeichnete schematische Fachansichten. Beim Neuladen werden die Beispieldaten zurückgesetzt.

Geprüft mit Chromium und Playwright: alle Fachansichten, Suche und Filter, Speichern und Ergebnisaktualität, Projekttabs mit getrennten Eingaben und Filtern, Schließen aktiver und inaktiver Akten, Abbrechen, Verwerfen und Speichern. Keine JavaScript-Fehler; kein horizontaler Seitenüberlauf bei 1024, 768 und 620 Pixel Breite.

[Bedienprüfung](pruefung.json) · [Prüfung der Projekttabs](projekttabs-pruefung.json)
