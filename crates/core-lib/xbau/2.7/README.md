# XBau 2.7 – Schemas zur Prüfung

`xsd_dev/` enthält die offiziellen XML-Schemas von XBau-Hochbau 2.7 mit dem
XBau-Kernmodul 1.4.0 in der Fassung mit lokalen Verweisen, unverändert aus der
Veröffentlichung der XLeitstelle Planen und Bauen (www.xleitstelle.de,
Download über www.xrepository.de). Die mitgelieferten XÖV-Schemas
(Basisnachricht, Codes, DIN 91379, latinchars) stehen unter
[CC BY-ND 4.0](https://creativecommons.org/licenses/by-nd/4.0/deed.de); sie
dürfen unverändert weitergegeben werden.

Die Schemas werden in Tests und bei der Laufzeitprüfung verwendet. Der
Plattformadapter bettet sie ein und prüft mit `xmllint`; siehe
[XBau-Austausch](../../../../docs/xbau/README.md).
`austausch.xsd`/`hochbau.xsd` sind eigene Aggregatoren für alle mitgelieferten
Nachrichten. Die offiziellen XSD-Dateien in `xsd_dev` bleiben unverändert.

`pruefschema.xsd` und `kernmodul.xsd` sind eigene Hilfsdateien. libxml2 lädt
pro Namensraum nur die erste importierte Schemadatei; die XBau-Schemas
importieren den Kernmodul-Namensraum aber aus mehreren Dateien.
`kernmodul.xsd` fasst diese Dateien zusammen und wird zuerst importiert.

```sh
cd crates/core-lib/xbau/2.7
xmllint --noout --nonet --schema pruefschema.xsd bauantrag.xml
```
