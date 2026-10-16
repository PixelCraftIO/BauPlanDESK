# Hell und Dunkel: Screenshotvergleich

20 Aufnahmen aus dem Produktionsbuild des Branches `codex/darkmode-abstaende`, bei 1600 × 1100 Pixeln. Fiktive Core-Testdaten, nativer Transport für die Browseraufnahme ersetzt. Die PNG-Dateien wurden nicht nachbearbeitet. In beiden Themes wurden dieselben Ansichten und Eingabestände aufgenommen.

[Galerie mit direktem Vergleich](index.html)

| Bereich | Hell | Dunkel | Bewertung |
| --- | --- | --- | --- |
| Projektverzeichnis | [Bild](01-projekte-light.png) | [Bild](01-projekte-dark.png) | **Gut:** Navigation, Tabellen und ausgewählte Bereiche sind in beiden Themes klar erkennbar. |
| Rechtsgrundlagen | [Bild](02-recht-light.png) | [Bild](02-recht-dark.png) | **Gut:** Feldränder, Beschriftungen und Formulargruppen sind deutlich besser voneinander getrennt. |
| Gebäude und Geschosse | [Bild](03-gebaeude-light.png) | [Bild](03-gebaeude-dark.png) | **Verbessert:** Geschosse sind als Gruppen erkennbar; längere Formulare benötigen weiterhin Scrollen. |
| Wohnfläche · Aufmaßmaske | [Bild](04-wohnflaeche-light.png) | [Bild](04-wohnflaeche-dark.png) | **Nacharbeit:** Die Checkboxen oberhalb der Teilflächen sind von ihren Beschriftungen getrennt. Bei direktem Aufmaß werden weiterhin nicht benötigte Geometriefelder angezeigt. |
| Stellplätze · Prüfbericht | [Bild](05-stellplaetze-light.png) | [Bild](05-stellplaetze-dark.png) | **Gut mit Detailbedarf:** Tabellen sind lesbar. Abschnitte ohne Positionen zeigen noch leere Tabellenköpfe. |
| PDF-Druckvorschau | [Bild](06-pdf-light.png) | [Bild](06-pdf-dark.png) | **Nacharbeit:** Das Originalpapier bleibt korrekt weiß. Der Druckdialog ist für umfangreiche Auswahl und Vorschau zu schmal und scrolllastig. |
| Einstellungen · Darstellung und Speicherorte | [Bild](07-einstellungen-light.png) | [Bild](07-einstellungen-dark.png) | **Gut:** Theme-Auswahl und Speicherorte sind deutlich gruppiert; längere Einstellungsseiten bleiben vertikal scrollbar. |
| Abstandsflächen · Ergebnis und Planskizze | [Bild](08-abstand-light.png) | [Bild](08-abstand-dark.png) | **Gut mit Detailbedarf:** Ergebnis und Planskizze sind erkennbar. Die blaue Flächenmarkierung könnte im dunklen Modus kräftiger sein. |
| Projektdateien · Kategorien und Versionen | [Bild](09-dateien-light.png) | [Bild](09-dateien-dark.png) | **Nacharbeit:** Dateien, Kategorien und Versionen sind lesbar. Filterbeschriftungen und Auswahlfelder stehen noch zu eng zusammen. |
| Verfahrensakte · Nachforderungen | [Bild](10-verfahren-light.png) | [Bild](10-verfahren-dark.png) | **Nacharbeit:** Aktive Tabs sind klar erkennbar. In den Nachforderungspunkten fehlen Abstände zwischen aufeinanderfolgenden Eingabezeilen. |

Die Ansichten wurden auf sichtbare Abstände, Lesbarkeit, aktive Zustände, Tabellen und Dialoge geprüft. In den aufgezeichneten Abläufen traten keine JavaScript-Fehler und keine horizontalen Überläufe der Dokumentwurzel auf. Das schließt lokale Scrollbereiche oder abgeschnittene Inhalte innerhalb eines Dialogs nicht aus.

Die Screenshots dokumentieren den aktuellen Stand einschließlich der genannten offenen Darstellungsprobleme. Dieser Arbeitsschritt verändert keine Anwendungslogik oder Gestaltung.
