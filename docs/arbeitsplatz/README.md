# Arbeitsplätze und Abgabeführung

Basis: Main `c1e1e80`. Die acht UX-Schwerpunkte werden innerhalb der bestehenden
Fachmodule umgesetzt. Es gibt keine neuen Fachregeln, Tabellenmigrationen,
automatischen Freigaben oder Änderungen an archivierten Abgaben.

| Schwerpunkt | Umsetzung |
| --- | --- |
| 1. Täglicher Projekteinstieg | Nächster Arbeitsschritt und bestehende Fristen bleiben zentral. Zuständigkeitsfilter „Mir zugewiesen“ / „Ohne Zuweisung“, vollständiges Zurücksetzen der Filter und ereignisgesteuerte Aktualisierung ergänzen die Übersicht. Ein einklappbarer Arbeitscheck bündelt die Abgabevorbereitung. |
| 2. Verständliche Zustände | Wiederverwendbare Statusanzeige mit Text, Symbol und Erklärung für Entwurf, Speicherung, Berechnung, Archivierung, geänderte Grundlagen und Freigabe. Die Statushilfe erklärt außerdem Export, Einreichung und belegten Eingang. Fachliche Zustände kommen weiterhin aus dem Core. |
| 3. Datenherkunft und Folgen | Quellenwege an Berechnungs- und Unterlagenbereichen führen zu den liefernden Modulen. Übernommene Fachwerte verlinken den konkreten Quellnachweis und zeigen dessen geladene Aktualität. Archivierte Nachweise führen zurück zu ihrem Produzenten. Veraltete Vorschauen bieten eine direkte Neuberechnung. |
| 4. Übersichtlichere Formulare | Eintragslisten sind beim Bearbeiten einklappbar. Gemeinsame Speicherleiste mit Status, Feldfehler öffnen auch verschachtelte Abschnitte. Bestehende bedingte Fachfelder und Entwurfssicherung bleiben wirksam. Fehlende optionale Bearbeitungsfristen öffnen die Verfahrensakte jetzt ohne Svelte-Bindungsfehler. |
| 5. Lesbare Berechnungen | Nutzungstabellen unterscheiden Ermittelt, Zulässig/Vorgabe, Abweichung und textliche Bewertung. Vorschau, vollständiger tabellarischer Rechenweg, Quellen, Archiv und Bürofreigabe sind direkt erreichbar. Fehlende Vorgaben ergeben keine positive Bewertung. |
| 6. Zusammenhängende Unterlagen | Unterlagenregister meldet neuere Projektversionen und öffnet den exakt zugeordneten Stand. Der Behördenstand führt auf die tatsächlich eingereichte Version. Die Historie hebt den referenzierten Stand hervor; Versionen werden nie automatisch ersetzt. |
| 7. Geführte Abgabe | Der gleiche Arbeitscheck steht vor der Einreichung zur Verfügung. Verfahrensbezogene Punkte werden auf die gewählte Akte eingegrenzt. Die Schrittauswahl führt zu Anforderungen, Antragsdaten, Unterlagen, Abgabe und Eingangsbeleg. Ladefehler bleiben ausdrücklich nicht auswertbar. |
| 8. Darstellung und Bedienbarkeit | Größere Felder, Schaltflächen und Navigationszeilen, mehr Tabellenabstand, klare Tastaturfokusse und lesbare Mehrzeilenfelder. Themevariablen gelten auch für neue Komponenten. Bildschirmregeln verändern keine Druckvorlagen. |

## Verträge und Grenzen

- `arbeitsplatz/arbeitscheck.ts` gruppiert vorhandene `Arbeitspunkt`-Einträge aus
  `projektuebersicht`; es berechnet keine fachliche oder rechtliche Abgabereife.
  Keine Meldungen bedeutet nicht, dass alle Anforderungen erfasst sind.
- Keine maschinelle Ampel bestätigt die Vollständigkeit des Bauantrags. Die
  vorhandenen Paket-/Portalprüfungen und die Prüfung des Sachbearbeiters bleiben
  für den konkreten Abgabestand maßgebend.
- Projektbezogene Ereignisse aktualisieren Lesemodelle. Request-Generationen
  verhindern, dass verspätete Antworten neuere Prüfdaten überschreiben.
- Die UI-Navigation verwendet für Dokumenthistorien `bereich=dokumente`,
  `objekt=Dokument-ID`, `register=historie`, `eintrag=Versionsnummer als Text`.
  Dies ist eine lokale Navigation; sie ändert keinen Core-Transportvertrag.
  Vor der Anzeige werden Projektzugehörigkeit und Version geprüft. Offene
  Dokumentbearbeitungen werden dabei nicht überschrieben.
- Fachnachweise navigieren mit ihrem gespeicherten Produzentenobjekt. Der
  bestehende Editor schützt einen dort eventuell vorhandenen anderen Entwurf.
- Zugewiesene Personen werden im jeweiligen Eintrag angezeigt. Die Übersicht
  unterscheidet eigene, fremde und fehlende Zuweisung ohne technische IDs.

## Prüfen

```sh
npm run check --prefix apps/desktop
npm run build --prefix apps/desktop
node --experimental-strip-types --test apps/desktop/tests/arbeitsplatz.test.mjs
BAUPROJEKT_UI_URL=http://127.0.0.1:1430 python apps/desktop/tests/arbeitsplatz-ui.py
```

Der Browserlauf braucht den Produktionsbuild auf dem angegebenen lokalen Server
und den vorhandenen Testtransport. Er prüft Navigation, Entwurfserhalt, genaue
Dateiversionen, Fehlerzustände und Light/Dark bei 100/125/150 Prozent. Die
Screenshots stammen aus fiktiven Testdaten; sie sind keine fachliche Prüfung.
Weitere Regressionen: `ui-felder-navigation.py`, `projekttabs.py`,
`projektuebersicht.py` (Core-Fixture), `handbuch-ui.py`.

Das aktualisierte [Bedienhandbuch](../handbuch/Bedienhandbuch.md) beschreibt die
neuen Abläufe; HTML und PDF werden weiterhin aus derselben Quelle erzeugt.

## Sichtprüfung

- [Projektübersicht hell](screenshots/uebersicht-hell.png)
- [Projektübersicht dunkel](screenshots/uebersicht-dunkel.png)
- [Einreichung bei 150 % dunkel](screenshots/einreichen-150-dunkel.png)
- [Wohnflächenformular dunkel](screenshots/wohnflaeche-dunkel.png)

Validiert: Svelte-Prüfung ohne Fehler/Warnungen, Produktionsbuild, 12 Node-Tests
(einschließlich Handbuch), neue Navigationstests zusätzlich unter Node 22,
Core-Integration `storage --test projektuebersicht` (1 Test) sowie die Browserläufe
Arbeitsplatz, Projektübersicht, Eingabefelder/Navigation, Projekttabs und Handbuch.
Native Windows-/macOS-Desktop-Builds wurden hier nicht ausgeführt.
