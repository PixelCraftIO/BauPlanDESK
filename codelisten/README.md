# XBau-Codelisten

Der XBau-Export braucht Codelisten, die nicht in den XBau-Schemas stehen
(Typ 3 und 4). Sie kommen als Genericode-Dateien (`*.xml`) vom XRepository:
www.xrepository.de, Suche nach der Kennung, Download „Genericode“.

Benötigt für den Bauantrag:

| Kennung                                    | Inhalt                      | Mitgeliefert                  |
|--------------------------------------------|-----------------------------|-------------------------------|
| `urn:xoev-de:xbau:codeliste:anlagen`       | Art der Anlage (Bauvorlage) | Version 4.0 (Entwurf, 07.10.2026) |
| `urn:xoev-de:xbau:codeliste:xbau-mimetypes`| Dateitypen der Anlagen      | Version 1 (19.12.2018)        |

Die mitgelieferten Dateien sind aus den Tabellen der XRepository-Seiten
übertragen. Die MIME-Types-Liste erlaubt nur PDF, IFC (auch XML/ZIP) und BCF;
andere Dateien (DXF, Bilder) müssen als PDF beigefügt werden.

Die Dateien aus diesem Ordner werden mit der App ausgeliefert. Neuere
Versionen legt man ohne Programm-Update in den Ordner `codelisten` im
Datenverzeichnis der App (Server: `BAUPROJEKT_CODELISTEN_DIR`); eine Datei
dort ersetzt die mitgelieferte Liste mit derselben Kennung.

Eingebaut ist nur der Verzeichnisdienst (`DVDV`, Version 1).
