# Datenfluss Gebäude → Baugrundstücke → Nachweise, Formulare und XBau

## Verhalten und Verträge

- **Gebäude stehen auf Flurstücken (neu):** In Gebäude/Geschosse werden die
  Flurstücke zugeordnet, auf denen ein Gebäude steht (`Gebaeude.flurstueck_ids`,
  stabile IDs der Grundstücksinformationen), bei Vereinigungsbaulast auch mehrere.
  Das Baugrundstück ist ein Rechenergebnis: Der Core leitet es je Berechnung aus
  der aktuellen Baugrundstücksermittlung ab (`Lagebezug`). Liegen alle
  Vorhabenflurstücke eines Gebäudes in einer Einheit, geht es dort ein; eine
  Vereinigungsbaulast wird über die gemeinsame Zuordnung der Flurstücke in der
  Ermittlung abgebildet. Umgruppierungen erfordern keine Übertragung mehr.
  Die frühere Zuordnung `grundstueck_ids` wird nur noch gelesen (Altbestand),
  neue Baugrundstückszuordnungen werden abgewiesen. Flurstückszuordnungen
  hängen nicht von einer bestätigten Ermittlung ab.
- **Flurstück entfernen:** `flurstuecksverweise` listet Gebäude und
  Abstandspläne, die auf Flurstücke verweisen. Die Grundstücksinformationen
  fragen vor dem Entfernen nach und bereinigen die Verweise nach dem Speichern
  (`flurstuecksverweise_entfernen`, nur für nicht mehr erfasste Flurstücke).
  Bereinigte Abstandspläne sind neu zu prüfen.
- **Übernommene Flurstücke:** Werden Flurstücke aus einer früheren
  Baugrundstückszuordnung übernommen, trägt das Gebäude
  `flurstuecke_zu_pruefen`, bis die Zuordnung bewusst geändert oder bestätigt
  wird; GRZ/GFZ zeigen solange einen Hinweis.
- **Nachfolge bei Umgruppierung:** Berechnungen halten beim Speichern ihre
  Flurstücke fest (`BaugrundstueckBerechnung.flurstueck_ids`). Liegen nach einer
  Umgruppierung alle verbliebenen Flurstücke einer alten Berechnung in genau einer
  neuen Einheit und erbt diese nur von ihr, meldet `BaugrundstueckGruppe.vorgaenger`
  die Nachfolge; die Oberfläche übernimmt die eigenen Eingaben dann automatisch
  (als ungespeicherte Änderung). Aufteilungen und Zusammenlegungen bleiben eine
  ausdrückliche Entscheidung.
- **Einzelgeschoss:** `einzelgeschoss_vollgeschoss` mit Grundlage; mit geprüfter
  Einstufung geht ein Gebäude ohne einzeln erfasste Geschosse automatisch in
  GFZ und Geschossigkeit ein.
  Ist die Einstufung im Gebäude festgelegt, übernimmt die GFZ sie bei jeder
  Prüfung (bisher blieb die beim Verknüpfen gewählte Einstufung als Kopie
  stehen); die Auswahl in der GFZ erscheint nur noch ohne Gebäudeangabe.
- **Verknüpfte Flächen gehören dem Adapter:** Passt ein verknüpftes Gebäude
  oder Geschoss nicht mehr eindeutig (anderes Grundstück, Geschoss entfernt),
  entfällt der Eintrag mit Hinweis, statt die Prüfung zu blockieren.

- **Gebäudeeinbindung (`plattform::einbindung`):** Ein Gebäude, das genau
  einem Baugrundstück zugeordnet ist (abgeleitet aus seinen Flurstücken), geht mit Grundfläche (GRZ) und Geschossen
  (GFZ/Geschossigkeit) automatisch in dessen Flächenaufstellung ein, sofern die
  Grundlagen vollständig sind. Ohne gültige Zuordnung (leer, frühere
  Projektfläche oder nicht mehr bestehende Einheit) zählt es nur bei genau einem
  Baugrundstück. Fehlende Angaben, mehrdeutige und verwaiste Zuordnungen ergeben
  Hinweise, nie Nullflächen. Gleichnamige manuelle Bauteile werden nicht doppelt
  übernommen.
- **Ein Rechenweg:** `CoreServices::projekt_mit_gebaeuden` liefert den Projektstand
  mit eingebundenen Gebäuden. Formulare (`formular_quellen`) und die XBau-Flächen
  der Abgabe verwenden ihn; damit stimmen sie mit Berechnung und Nachweis
  überein. Kann der Abgleich nicht erfolgen, bleibt der gespeicherte Stand mit
  Hinweis.
- **Quellenstand der Baugrundstücksermittlung:** Nur Vorhabenflurstücke mit
  Kataster-, Grundbuch- und Flächenangaben. Nachbar-/Baulastflurstücke,
  Bemerkungen, Grundbuchauszüge und Eigentümer lassen die Ermittlung aktuell.
  Gespeicherte Stände nach altem Verfahren (`fingerprint_v1`) bleiben gültig,
  bis sich die Angaben ändern.
- **Veraltete Gebäudezuordnungen:** Unveränderte, inzwischen verwaiste
  Zuordnungen blockieren das Speichern eines Gebäudes nicht; nur neue
  Zuordnungen müssen auf eine aktuelle Einheit zeigen. Die Oberfläche zeigt sie
  als entfernbar an. `gebaeudezuordnung_uebertragen` (nur Altbestand) setzt für
  Gebäude einer nicht mehr bestehenden Einheit ausdrücklich die Flurstücke einer
  aktuellen Einheit (Desktop-Command
  und `POST /projekte/{id}/gebaeudezuordnung`). Der GRZ/GFZ-Bereich bietet bei
  verwaisten Berechnungen „Eingaben übernehmen“ und überträgt die
  Gebäudezuordnungen nach dem Speichern.
- **Baufenster je Baugrundstück:** `BaugrenzenGeometrie.grundstueck_schluessel`
  (optional). Ohne Angabe gilt der Umrissvergleich nur bei genau einem
  Baugrundstück; bei mehreren wird er keinem still zugerechnet (Hinweis).
- **Gebäudehöhen:** `GebaeudeGrundlagen.hoehen_nhn` (Gelände, Traufe, First in
  m ü. NHN). Das Höhenmodul übernimmt Traufe/First auf Wunsch
  (`gebaeudedaten_uebernehmen`); Abstandswände können über `gebaeude_id` mit
  einem Gebäude verknüpft werden und Gelände/Traufe übernehmen.
- **Bauantrag:** `anzahl_gebaeude` folgt den erfassten Gebäuden
  (`gebaeude_zentral`). Widersprüche zwischen Landes-Verfahrensart und
  Verfahrensakte (Art, Prüfverfahren) sowie eine fehlende Anschrift bei
  erfasstem Standort erscheinen als Hinweise; maßgebend bleibt die Akte.

## Kompatibilität und Verbraucher

Keine automatische Datenmigration. Gebäude mit früherer Baugrundstückszuordnung
bleiben lesbar und rechnen weiter; Gebäude/Geschosse bietet an, die Flurstücke
des bisherigen Baugrundstücks zu übernehmen. Vertragsänderung: neue Gebäude
können nur noch Flurstücken zugeordnet werden (Verbraucher: Desktop, Server-API).
Neue Felder sind optional (`flurstueck_ids`, `flurstuecke_zu_pruefen`,
`BaugrundstueckBerechnung.flurstueck_ids`, `einzelgeschoss_vollgeschoss`,
`gebaeude_id`, `grundstueck_schluessel`) oder reine Ausgaben (`hoehen_nhn`, `gebaeude_zentral`).
Regeländerung: GRZ/GFZ-Nachweise, deren zugeordnete Gebäude bisher nicht
verknüpft waren, gelten als neu zu prüfen; vollständig verknüpfte Altstände
behalten ihren Fingerprint. Bei mehreren Baugrundstücken erscheint die Lage im
Baufenster erst nach Zuordnung des Vorhabenumrisses.

## Validierung

- Core: Einbindung (je Grundstück, idempotent, unvollständig, mehrdeutig,
  verwaist, Projektfläche), Quellenstand inkl. Altstand, Baufenster je
  Grundstück, Abgleich der Verfahrensart.
- Storage-Integration: GRZ/GFZ ohne Neueingabe, Nachweis veraltet bei neuer
  Zuordnung, Umgruppierung von Projektfläche zu Flurstücken, Formularwert aus
  Gebäude, Gebäudeanzahl im Bauantrag; Fachberechnungen mit Oberirdisch-Kriterium.
- `svelte-check`, Produktionsbuild, Node-Tests.
