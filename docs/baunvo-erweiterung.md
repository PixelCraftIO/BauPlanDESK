# BauNVO-Berechnungen gezielt erweitern

Die technische Vorbereitung ist vorhanden; sie ersetzt keine fachliche Prüfung der einzelnen Anrechnungsfälle. Die aktuell implementierten Regeln decken Grundfälle ab. Sonderfälle und eine fachlich freigegebene vollständige BauNVO-Abbildung bleiben weitere Arbeit.

## Bestehender Datenfluss

1. Rechtsgrundlagen legt die für das konkrete Vorhaben anzuwendende BauNVO-Fassung fest. Maßgebend ist die anwendbare Rechtsgrundlage des Plans, nicht das heutige Kalenderjahr.
2. Das Baugrundstücksmodul stellt die einzelnen Grundstückseinheiten und ihre Bezugsflächen bereit. Mehrere Einheiten bleiben getrennt.
3. Der Nutzer erfasst Bauteiltyp und Fläche. Der Core übernimmt die Projektfassung und bestimmt die Anrechnung aus dem zugehörigen Regelwerk.
4. GRZ/GFZ/Geschossigkeit liefern ihre bestehenden Ergebnisverträge mit Ansätzen, Kennzahlen und Regelkennung. Formulare und andere Verbraucher müssen die Anrechnungsregeln nicht selbst nachbauen.
5. Gespeicherte Nachweisstände enthalten Eingaben und Regeln einschließlich Hash und Version; neue Regeln überschreiben keine alten Nachweisstände.

## Regeldateien

Die sechs Fassungen liegen in `regelwerke/baunvo/1962.json`, `1968.json`, `1977.json`, `1990.json`, `2017.json` und `2021.json`. Die Fassung 2021 (Baulandmobilisierungsgesetz, in Kraft seit 23.06.2021) rechnet wie 2017 und ist die geltende Fassung ohne Bebauungsplan. Das urbane Gebiet (MU) gibt es seit 2017, das dörfliche Wohngebiet (MDW) seit 2021; ein Plan einer älteren Fassung kann sie nicht festsetzen, der Core weist das beim Speichern ab. Ältere widersprüchliche Stände bleiben speicherbar, solange Fassung und Baugebiet unverändert bleiben. Core und Oberfläche beziehen diese Dateien ein. BauNVO-Regeln werden derzeit mit der Anwendung gebaut und ausgeliefert; sie sind noch keine vom Endnutzer austauschbaren Laufzeitpakete. Die LBO-Skripte besitzen bereits einen eigenen externen Ladeweg.

Jedes Regelwerk enthält insbesondere:

| Eintrag | Aufgabe |
| --- | --- |
| `id`, `version`, `fassung` | Nachvollziehbarer Regelstand |
| `bauteile` | Bauteilkennung, Bezeichnung und automatische Anrechnung |
| `grz_ii` | Unterscheidung zwischen älterem Ansatz und GRZ I/II |
| `grz_ii_faktor`, `grz_ii_kappung` | Aktuell implementierter zusätzlicher GRZ-Rahmen |
| `nichtvollgeschoss_faktor`, `nichtvollgeschoss_planabweichung` | GFZ-Anrechnung und zulässige Erfassung einer begründeten Planfestsetzung |
| `gemeinschaftsanteile` und Rechtsgrundlage | Unterstützung der zugeordneten Gemeinschaftsflächen |
| `quellen` | Quellenangaben zum implementierten Ansatz |

Eine Anrechnung lautet `grz`, `grz_ii` oder `keine`. Der Bauteiltyp `garage_abstandsflaeche` steht für Garagen und Carports, die nach Landesrecht in der Abstandsfläche oder an der Grenze zulässig sind. Nach den Fassungen 1962 bis 1977 werden sie nicht angerechnet (§ 19 Abs. 4 a. F.), ab 1990 zählen sie zur GRZ II. Der Typ `garage` gilt in den alten Fassungen für Garagen außerhalb der Abstandsfläche. In den heutigen älteren Regeldateien werden beispielsweise bestimmte Nebenanlagen nicht angesetzt, während die Fassungen 1990/2017 zusätzliche Flächen der GRZ II zuordnen. Das beschreibt den heutigen Implementierungsstand und keine pauschal abschließend geprüfte Einordnung jedes Einzelfalls.

## Welche Änderung gehört wohin?

**Andere Anrechnung eines vorhandenen Bauteiltyps:** Betroffene Regeldateien ändern, Regelversion erhöhen und fachliche Regressionen ergänzen. Die Eingabemaske verwendet die Bezeichnungen und Zuordnungen der ausgewählten Fassung.

**Zusätzlicher Bauteiltyp:** Den Typ in `crates/core-lib/src/flaechen.rs` ergänzen und seine Behandlung in allen fünf Regeldateien ausdrücklich festlegen. Eine nicht anzurechnende Fläche erhält ausdrücklich `keine`. Die TypeScript-Typen werden aus Rust erzeugt; die Auswahlliste entsteht aus den Regeldateien. Fehlende Zuordnungen dürfen nicht als Nullansatz verschwinden.

**Komplexere Berechnung:** Fachlogik im Flächen-/BauNVO-Modul erweitern, etwa zusätzliche Bedingungen, dokumentierte Sonderfälle oder geometrische Überlagerungen. Ein neuer Algorithmus benötigt einen passenden Rechenstand und aussagekräftige Testfälle. Die Grundstücksermittlung bleibt Lieferant der Bezugsflächen. PDF, DMS und andere Verbraucher behalten ihren Ergebnisvertrag, solange dessen Bedeutung unverändert bleibt. Zusätzliche benötigte Eingaben werden ausdrücklich typisiert, mit Einheiten und sicherer Behandlung fehlender Werte ergänzt.

**Neue Fassung:** Rust-Fassung, Regeldatei, Katalog und Falltests ergänzen. Eine unbekannte Fassung darf nicht auf die neueste Fassung zurückfallen.

## Prüfung und Archiv

`crates/core-lib/src/flaechen.rs` prüft bereits alle fünf Fassungen mit denselben Beispielbauteilen und unterschiedlichen erwarteten Ansätzen. Weitere Tests sichern Projektfassung vor lokaler Eingabefassung, Grundstückszuschläge und getrennte Geschosszahlen mehrerer Gebäude ab.

Die Pipeline prüft zudem, dass die generierten Typen nicht von Rust abweichen:

```sh
cargo run -p core-lib --example vertraege > /tmp/plattform.ts
cmp apps/desktop/src/lib/generated/plattform.ts /tmp/plattform.ts
cargo test -p core-lib
```

Archivierte LBO-Skripte und BauNVO-Regelkonfigurationen können unabhängig vom aktuell ausgelieferten Regelpaket wiederholt werden. BauNVO-Nachweise verwenden dafür ausschließlich ihre archivierten JSON-Pakete, einschließlich Bauteilanrechnung, Nichtvollgeschossansatz und GRZ-II-Grenzen. Fehlende, unvollständige, doppelte oder widersprüchliche Pakete führen zu einem Fehler; ein Rückfall auf aktuelle Regeln ist ausgeschlossen. Hash, Regelkennung und Version werden überprüft.

Die vorhandenen Berechnungsaufrufe bleiben erhalten. `FlaechenAufstellung::berechne_mit_regeln`, `festsetzungen::pruefe_mit_regeln` und `BaugrundstuecksPlan::pruefe_mit_regeln` bilden die zusätzliche Katalogschnittstelle. Der Ergebnisvertrag bleibt unverändert. Integrationstests prüfen einen gespeicherten historischen Nachweis mit abweichender Anrechnung sowie beschädigte Archive.

Regelkonfiguration und Algorithmus sind getrennt: Die Wiederholung benötigt weiterhin einen unterstützten **Rechenkernstand** (`plattform-1`). Ein künftiger Algorithmuswechsel muss historische Rechenkerne ausdrücklich unterstützen oder die Wiederholung mit einer klaren Meldung verweigern. Das archivierte Ergebnis bleibt lesbar. Eine Endnutzeroberfläche zum Installieren eigener BauNVO-Pakete ist damit noch nicht verbunden.
