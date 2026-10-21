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

Im Bauantrag lässt sich eine Liste auch über „Codeliste einspielen …“ wählen
(Recht „Büroangaben ändern“). Die App prüft die Datei, legt sie in diesem
Ordner des Datenverzeichnisses ab und verwendet sie sofort, ohne Neustart.

Landeslisten der Verfahrensarten (`urn:xoev-de:xbau:<land>:codeliste:verfahrensart`,
Typ 4) werden nicht mitgeliefert. XBau gibt keine bundesweite Liste vor. Für
Nordrhein-Westfalen war bei Redaktionsschluss keine Liste im XRepository zu
finden. Ohne Liste enthält die XBau-Nachricht keine Verfahrensart; das ist
zulässig, weil das Element optional ist. Stellt das Land oder die Bauaufsicht
eine Liste bereit, wird sie wie oben eingespielt.

Eingebaut ist nur der Verzeichnisdienst (`DVDV`, Version 1).
