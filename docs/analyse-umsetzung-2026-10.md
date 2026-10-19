# Umsetzung der Plausibilitätsanalyse (Oktober 2026)

## Verhalten und Verträge

- **BauNVO 2021:** neue Fassung `"2021"` mit eigenem Regelwerk. Ohne Bebauungsplan
  gilt sie als aktuelle Fassung (`BauNvo::AKTUELL`); die Rechenwerte entsprechen
  2017. `Baugebiet::eingefuehrt` liefert die erste Fassung eines Baugebiets
  (MU 2017, MDW 2021). `Planungsrecht::einordnung_pruefen` weist ein Baugebiet ab,
  das die erfasste Planfassung nicht kennt.
- **Grenzgarage:** neuer Bauteiltyp `garage_abstandsflaeche`. Bis 1977 keine
  Anrechnung, ab 1990 GRZ II. Alle Regelwerke tragen den Typ; ihre Version ist
  auf 1.1.0 gestiegen.
- **Vollgeschossvorlage:** Das Oberirdisch-Kriterium ist standardmäßig aktiv.
  Ohne Deckenhöhe über Gelände bleibt das Geschoss offen.
- **Abstandsflächen BW:** WB mit 0,2 H, Dachregeln 45°/70°, Hinweise zu Giebel
  und kurzen Wänden. Rhai-Paket und Katalog sind per Test gleichgeschaltet.
- **Verfahrensakte:** neue Verfahrensarten `nutzungsaenderung`, `beseitigung`,
  `widerspruch`; Felder `pruefverfahren` und `bearbeitungsfrist`; Bescheidmerkmal
  `teilbaugenehmigung`; Schrittarten `einvernehmen`, `fachbeteiligung`,
  `nachbarbeteiligung`; Terminarten `beteiligung` und `bearbeitungsfrist`.
  Der Vorbescheid erhält eine Geltungsdauer von drei Jahren nach § 75 MBO.
- Das Register „Anhörung / Zurückstellung“ heißt jetzt „Behördliche Schritte“.

## Kompatibilität und Verbraucher

Keine Datenmigration. Alle neuen Felder sind optional und werden bei älteren
Akten leer gelesen; leere Werte werden nicht geschrieben. Bestehende Bescheide
gelten als vollständige Entscheidung wie bisher. Gespeicherte Projekte mit
Fassung 2017 bleiben unverändert; ein bereits widersprüchliches Baugebiet bleibt
speicherbar, solange Fassung und Baugebiet unverändert bleiben. Neue
Enum-Werte erweitern die TypeScript-Verträge in `plattform.ts`; Verbraucher mit
vollständigen Fallunterscheidungen müssen sie berücksichtigen. Vorbescheide
zeigen nach dem Update eine berechnete Geltungsdauer und eine Wiedervorlage.

## Validierung

- Core: BauNVO-Bindung, Grenzgarage in allen Fassungen, Vollgeschoss mit
  Oberirdisch-Kriterium, Vorbescheid-, Nutzungsänderungs-, Widerspruchs- und
  Teilbaugenehmigungsfristen, Konsistenz von Rhai-Paketen und Katalog.
- Storage-Integration: Einvernehmen mit Fiktionsende, Bearbeitungsfrist mit
  Überfälligkeit, Teilbaugenehmigung ohne Verfahrensabschluss, Widerspruch nur
  mit Vorgängerbescheid.
