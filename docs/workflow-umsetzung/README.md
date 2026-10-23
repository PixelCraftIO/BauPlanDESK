# Umsetzung des Workflow-Audits

Basis: `6d7ae35` mit dem [Audit vom 23.10.2026](../workflow-audit/2026-10-23/README.md).
Branch: `codex/workflow-verbesserungen`. Main `9d2aee9` enthält die bereits gemergten Stabilitätsänderungen und wird in den Branch aufgenommen.

## Was sich für Planer ändert

| Befund | Umsetzung |
| --- | --- |
| U01 – Frühzeitige Sperren | Gesperrte Nutzungsmodule bieten einen getrennten Entwurfsweg. Flächen bleiben bei ungeklärtem Maßstab ohne behauptete Anrechnung; Berechnung und Nachweis bleiben gesperrt. Vorbereitende Entwürfe können auf diesem Gerät benutzer-/projektbezogen gesichert und wiederhergestellt werden. |
| U02 – Versteckte Voraussetzungen | Jeder gesperrte Eintrag besitzt einen Mouseover-Hinweis mit konkreten Arbeitsschritten. Das Informationssymbol ist per Tastatur erreichbar; Escape schließt die Erklärung. Ein direkter Weg führt zu Grundlagen bzw. Entwurf. Die Erklärung wird außerhalb des Sidebar-Scrollcontainers angezeigt. |
| U03 – Verschiedene Nachweiswege | Die PDF-/Archivaktion heißt in den Berechnungsfamilien einheitlich „Nachweis erstellen …“. Fachrechnungen erklären die Folge Entwurf → Prüfung → Nachweis → Freigabe. Der Archivierungserfolg bleibt sichtbar, auch wenn eine anschließende Listenaktualisierung scheitert; bestehende Vorgangskennungen verhindern doppelte Archivierung bei Wiederholung. |
| U04 – Bestätigungsschleife | „Gespeicherte Grundlagen bestätigen …“ speichert zunächst die geänderten Eingaben und bestätigt danach mit vorhandener Berechtigung die gespeicherte Revision. Parallelkonflikte unterbrechen den Ablauf; es gibt keine automatische fachliche Nachweisfreigabe. |
| U05 – Räumliche Abstandsflächenbearbeitung | Geführte Schritte, Planskizze neben der Wandbearbeitung, Wandwahl mit Maus und Tastatur. Dachparameter lassen sich gezielt von einer anderen Wand übernehmen. Geometrie, Gelände, Sicherungen, Belege und Freigaben werden dabei nicht kopiert. Bei kleinen Fenstern stehen Plan und Felder untereinander. |
| U06 – Quellen und Entwürfe | Quellenwege zeigen ungespeicherte Quellentwürfe und erlauben das gezielte Speichern nur dieses Bereichs. Ein Rückweg erhält den aufrufenden Berechnungskontext. Vorschau-Signaturen betrachten ausgewählte Gebäude/Raumbücher statt aller Objekte und ignorieren reine Titel-/Projektversionsänderungen. Rechtliche Grundlagen und Fehlerzustände bleiben relevant; die verbindliche Aktualität archivierter Stände kommt weiterhin aus dem Core. |
| U07 – Vollgeschossstatus | Vom konkreten Geschoss aus einen Prüfauftrag für das Gebäude öffnen. Der Benutzer legt die Prüfung ausdrücklich an. Nach dem Archivieren zurückkehren und den Nachweis am Geschoss bewusst übernehmen; vorhandene Entwürfe und Einstufungen werden nicht automatisch ersetzt. |
| U08 – Raumänderungen | Vergleich vor der Übernahme mit Neu/Aktualisiert/Entfällt/Manuell beibehalten. Anwenden verändert zunächst nur den Entwurf. Rückgängig ist möglich, solange keine weiteren Änderungen oder Speicherungen erfolgt sind. Zusätzlich Tabellenmodus für Positionen mit den gleichen Feldprüfungen und Quellensperren. |
| U09 – Regelvorgaben | Neues Abstandsprojekt startet bei unbekanntem Baugebiet mit „Noch zu klären“ statt WA. Persönliche versionierte Regelpakete können wiederverwendet und als JSON für Büroarbeitsplätze exportiert werden. Bestätigungen werden nie auf das neue Projekt übertragen. Geänderte Regeln benötigen weiterhin eine neue Regelversion. |
| U10 – Formularbibliothek | Aus Projektformularen direkt zu Vorlagen-/Feldeinrichtung und zurück. Die Bibliothek wird bei Rückkehr neu geladen; verspätete Antworten ersetzen keine neuere Bibliotheksansicht. Aus einer Anforderung heraus wird der Ausgangskontext bis zur Zuordnung mitgeführt. |
| U11 – Unterlagenkontexte | Eine verfahrensbezogene Arbeitsliste bündelt Anforderungen, referenzierte Dokumentversionen und neuere Projektstände. Formularerstellung und Dateiimport/-auswahl können aus einer Anforderung beginnen. Die konkrete Version wird anschließend ausdrücklich zugeordnet; die Freigabe wird zurückgesetzt. Druck kann die exakten Versionen eines eingefrorenen Abgabestands übernehmen und meldet nicht verfügbare/nicht druckbare Stände. |
| U12 – Große Dokumentauswahl | Gemeinsame Suche nach Dateiname, Kategorie und Dateityp. Aktuelle Versionen zuerst, historische ausdrücklich einblendbar. Zugeordnete historische Stände bleiben bei Filterwechsel erhalten und können exakt geöffnet werden. Fehlende Metadaten werden verständlich angezeigt. Einzelimport gleichnamiger Dateien bietet Neue Version / Separate Datei / Abbrechen. |
| U13 – Verfahren | Verfahrensbereiche sind in Vorbereitung, laufendes Verfahren und Entscheidung/Ausführung gegliedert. Aktive Akte, Behörde und Aktenzeichen bleiben im Verfahrenskopf sichtbar. Einreichen und Portalbereich verwenden den gewählten Verfahrenskontext weiter. |
| U14 – Arbeitskomfort | Bereichs-/Aktionssuche mit Strg+K, lokale Favoriten und zuletzt geöffnete Bereiche; einklappbare Projektdetails. Beschriftungen großer Felder stehen auf eigenen Zeilen. Quellwerte bleiben gesperrt, die vorhandene haftende Speicherleiste und Feldfehlernavigation bleiben wirksam. |
| U15 – Arbeitsaufträge | Gemeinsame Liste für Nachforderungspunkte und Pflichten aus Bescheiden mit Zuständigkeit, Frist, führendem Status und Beleganzahl. Originalschreiben sind direkt erreichbar. Hinweise ohne Pflicht sind keine Aufgabe. Links führen zu Originaleintrag und vorhandenen Wiedervorlagen/Kalenderexporten; keine duplizierten Erledigungsstände. |

## Architektur und erhaltene Verträge

- `ProjektNavigation` besitzt nur den flüchtigen Arbeitskontext einer Projektakte:
  Rückweg, ausgewähltes Verfahren, Prüfauftrag und Zuordnungsvorschlag. Dort
  werden weder fachliche Vollständigkeit noch Versandstatus berechnet.
- Flurstücke, Baugrundstücksbildung, Berechnungen, DMS, Verfahren und PDF-Ausgabe
  behalten ihre vorhandenen Backend-Verträge. Keine Tabellenmigrationen und keine
  neuen Fachregeln. Behördenübermittlung erfolgt weiterhin im externen Portal.
- `EditorRegister.speichernIn` speichert nur die registrierten Editoren eines
  ausdrücklich gewählten Quellbereichs. Andere Projektentwürfe bleiben bestehen.
- `Dokumentwahl` wird von Einzel- und Mehrfachzuordnungen gemeinsam verwendet.
  Der gespeicherte Bezug bleibt immer Dokument-ID plus konkrete Version.
- Raumvergleich arbeitet auf einer Kopie. Vor Anwendung müssen Eingabe- und
  Quellenstand noch dem Ausgangsstand entsprechen. Rückgängig überschreibt keine
  zwischenzeitlichen Bearbeitungen. Die verbindliche Import-/Berechnungslogik
  bleibt im Core; der Vergleich ergänzt die bestehende UI-Zusammenführung.
- `quellenstand.ts` bestimmt nur die Aktualität der angezeigten Fachvorschau.
  Rechtliche Grundlagen werden konservativ vollständig berücksichtigt. Die
  Archiv-/Freigabefingerprints wurden nicht durch UI-Signaturen ersetzt.
- Neue Regelpaketsammlung und vorbereitende Nutzungsentwürfe verwenden den
  bestehenden IndexedDB-Speicher und sind benutzerbezogen. Sie sind lokale
  Arbeitsplatzdaten, keine serverseitig geteilte Büroablage. Exportierte
  Regelpakete können über den vorhandenen Importweg auf andere Geräte gelangen.

## Bewusste Grenzen

Die 15 Befunde sind in den vorhandenen Modulen adressiert. Die weiterführenden
Ausbaurichtungen des Audits – beispielsweise Excel-Massenimport, vollständig
zentral administrierte Gemeindepakete, eine
komplette Neuzeichnung der Geometrieoberfläche – sind keine neu zugesagten
Funktionen dieses Schritts. Die neuen Tabellen, Paketdateien und die vorhandenen
Geometrieeditoren bilden dafür die Grundlage.

Lokale Entwürfe werden nicht als bestätigte Grundstücks-/Berechnungsstände an
andere Arbeitsplätze weitergegeben. Bei Wiederherstellung müssen geänderte
Flurstücksquellen erneut geprüft werden. Ein vollständiger Planernutzertest,
native Screenreaderprüfung und echte Behördenannahme können nicht durch
Browsertests mit simuliertem IPC ersetzt werden.

## Prüfen

```sh
npm run check --prefix apps/desktop
npm run build --prefix apps/desktop
npm run handbuch:check --prefix apps/desktop
node --experimental-strip-types --test apps/desktop/tests/workflow-quellen.test.mjs
BAUPROJEKT_UI_URL=http://127.0.0.1:1430 python apps/desktop/tests/workflow-ux.py
```

Der neue Browserlauf verwendet die Fach-/Verfahrensfixtures aus den vorhandenen
Storage-Integrationstests. Die CI erzeugt diese bereits und führt den neuen Lauf
zusätzlich aus. Er prüft gezielt die Übergänge, nicht nur das Rendern von Masken:
Sperrgrund, lokalen Entwurf nach Reload, zweistufige Grundlagenbestätigung,
Tabellenmodus, Raumvergleich/Rückgängig, Kontext-Rückkehr, bewusste Versionierung
und Importabbruch. Weitere Regressionen stehen in `verfahren.py`,
`abstandsflaechen.py`, `fachberechnungen.py`, `arbeitsplatz-ui.py`,
`ui-felder-navigation.py`, `projekttabs.py` und `druckartefakt.py`.

Die integrierte Hilfe und das exportierte [Bedienhandbuch](../handbuch/Bedienhandbuch.md)
beschreiben die neuen Schritte einschließlich lokaler Speichergrenzen.

## Ergebnis der Prüfungen

- Svelte: 0 Fehler, 0 Warnungen; Produktionsbuild erfolgreich. Der bereits bekannte
  Hinweis auf einen Hauptchunk über 500 kB bleibt bestehen.
- 17 Node-Tests unter Node 22 bestanden, darunter fünf neue Quellen-/Aktualitätsfälle.
- 12 Browser-Suites bestanden; der neue Workflowlauf prüft auch Mouseover,
  Tastatur, Importabbruch sowie Hell/Dunkel und 150-Prozent-CSS-Zoom.
- Handbuch Markdown/HTML/PDF synchron; 55 npm-Pakete ohne Lizenzverstoß.
- [Maschinenlesbares Prüfprotokoll](belege/pruefung.json) mit Links zu den
  einzelnen Browserausgaben. Die Browser verwenden Test-IPC; echte native
  Portalabgaben wurden nicht ausgeführt.

### Sichtbelege aus fiktiven Projekten

- [Mouseover / Tastaturhinweis im hellen Modus](belege/00-sperrgrund-light.png)
- [Hinweis im dunklen Modus bei 150 Prozent](belege/00-sperrgrund-dark.png)
- [Vorbereitende GRZ-Erfassung](belege/01-grz-entwurf.png)
- [Dokumentzuordnung zur Anforderung](belege/03-anforderung.png)
- [Planorientierte Wandbearbeitung im dunklen Modus](belege/06-abstand-dark-1.png)
