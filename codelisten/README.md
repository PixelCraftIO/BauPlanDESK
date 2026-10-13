# XBau-Codelisten

Der XBau-Export braucht Codelisten, die nicht in den XBau-Schemas stehen
(Typ 3 und 4). Sie kommen als Genericode-Dateien (`*.xml`) vom XRepository:
www.xrepository.de, Suche nach der Kennung, Download „Genericode“.

Benötigt für den Bauantrag:

| Kennung                                    | Inhalt                     |
|--------------------------------------------|----------------------------|
| `urn:xoev-de:xbau:codeliste:anlagen`       | Art der Anlage (Bauvorlage)|
| `urn:xoev-de:xbau:codeliste:xbau-mimetypes`| Dateitypen der Anlagen     |

Die Dateien aus diesem Ordner werden mit der App ausgeliefert. Neuere
Versionen legt man ohne Programm-Update in den Ordner `codelisten` im
Datenverzeichnis der App (Server: `BAUPROJEKT_CODELISTEN_DIR`); eine Datei
dort ersetzt die mitgelieferte Liste mit derselben Kennung.

Eingebaut ist nur der Verzeichnisdienst (`DVDV`, Version 1).
