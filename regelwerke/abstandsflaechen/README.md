# Abstandsflächen-Regelwerke je Land und Fassung

Eine Datei je Bundesland und Fassung des Abstandsflächenrechts:

```
<land>/<gültig ab>.json   z. B. nw/2024-01-01.json (BauO NRW 2018, 01.01.2024–31.08.2026)
<land>/pilot.json         Pilotansatz ohne belegte Fassung (wird ersetzt)
<land>/quellen/           Gesetzestext als PDF, mit SHA-256 in der Regeldatei
freigaben.json            Prüfsummen der fachlich freigegebenen Dateien
```

## Regeln für Änderungen

1. **Eine freigegebene Datei wird nie geändert.** Eine Gesetzesänderung bekommt
   eine neue Datei mit neuem Gültigkeitsbeginn; bei der Vorgängerdatei wird nur
   `gueltig_bis` gesetzt, solange sie noch nicht freigegeben ist. Ein Test
   vergleicht freigegebene Dateien mit ihrer Prüfsumme in `freigaben.json`.
2. **`"geprueft": true` setzt nur das Büro** (fachliche Abnahme), zusammen mit
   Prüfer, Datum und dem Eintrag in `freigaben.json`.
3. **Fassungen eines Landes schließen lückenlos an** (`gueltig_bis` + 1 Tag =
   nächstes `gueltig_ab`).
4. **Jede datierte Fassung bringt Wortlaut, Quelle und Beispielrechnungen mit.**
   Die Beispielrechnungen rechnet ein Test mit dem Rechenkern nach.
5. Neue Dateien in `crates/core-lib/src/abstandsflaechen/regelwerk.rs`
   (`DATEIEN`) eintragen; ein Test meldet vergessene Dateien.

## Aufbau einer Datei

| Feld | Inhalt |
| --- | --- |
| `fassung` | Gesetz, Paragraf, `gueltig_ab`, `gueltig_bis`, Fundstelle, Übergangsregel |
| `quellen` | Titel, Adresse, Abrufdatum, beigelegte Datei und deren SHA-256 |
| `pruefung` | `geprueft`, `geprueft_von`, `geprueft_am`, Vermerk |
| `wortlaut` | Gesetzestext je Absatz, wörtlich aus der Quelle |
| `pruefpunkte` | Tatbestände, die der Rechenkern nicht prüft, je Absatz als Prüffrage |
| `beispiele` | Einzelwand mit Gelände, Oberkante, Dach, Gebiet und Sollwerten für H und Tiefe |
| `regeln` | Rechenregeln (Tiefen je Gebiet und Fall, Wandhöhenansatz, Dach, Giebel, Gauben, Grenzanlagen, Überdeckung) |

Der Abstandsplan speichert eine Kopie von `regeln`; damit rechnen alte
Nachweise auch nach neuen Fassungen unverändert.

## Stand der Quellen (05.11.2026)

| Datei | Quelle | Stand |
| --- | --- | --- |
| `nw/2024-01-01.json` | Ausdruck § 6 von recht.nrw.de (Fassung 01.01.2024–31.08.2026) | Wortlaut übertragen, fachlich ungeprüft |
| `nw/2026-09-01.json` | Amtliche PDF von recht.nrw.de, abgerufen 05.11.2026 | Wortlaut übertragen, fachlich ungeprüft; gegenüber 2024 nur Abs. 1 Satz 3 und Verweis in Abs. 4 Satz 6 geändert |
| `by/pilot.json`, `bw/pilot.json` | keine | Pilotansatz, Fassung nicht belegt |
