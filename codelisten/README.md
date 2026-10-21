# XBau-Codelisten

Der XBau-Export braucht Codelisten, die nicht in den XBau-Schemas stehen
(Typ 3 und 4). Sie kommen als Genericode-Dateien (`*.xml`) vom XRepository:
www.xrepository.de, Suche nach der Kennung, Download „Genericode“.

Benötigt für den Bauantrag:

| Kennung                                    | Inhalt                      | Mitgeliefert                  |
|--------------------------------------------|-----------------------------|-------------------------------|
| `urn:xoev-de:xbau:codeliste:anlagen`       | Art der Anlage (Bauvorlage) | Version 4.0 (Entwurf, 07.10.2026) |
| `urn:xoev-de:xbau:codeliste:xbau-mimetypes`| Dateitypen der Anlagen      | Version 1 (19.12.2018)        |

Für beantragte Abweichungen und Erleichterungen (Element `abweichung` bzw.
`erleichterung` im Antrag):

| Kennung                                              | Inhalt                                   | Mitgeliefert |
|------------------------------------------------------|------------------------------------------|--------------|
| `urn:xoev-de:xbau:codeliste:abweichungart`           | Art der Abweichung (Befreiung, Ausnahme, Abweichung nach MBO) | Version 2.1 (Endfassung, 17.12.2024) |
| `urn:xoev-de:xbau:codeliste:erleichterung.gegenstand`| Gegenstand einer Erleichterung für Sonderbauten (Ziffern § 51 MBO) | Version 1.0 (Endfassung) |

Anlagen und MIME-Types sind aus den Tabellen der XRepository-Seiten
übertragen. Abweichungsart und Erleichterung sind die unveränderten
Genericode-Dateien des XRepository (abgerufen am 21.10.2026, siehe
[Prüfprotokoll](../docs/verfahrensakte/rechtspruefung-nrw.md)).

Befreiung und Ausnahme erhalten ihren Code automatisch (Ausnahme nach
§ 31 Abs. 1 BauGB oder § 14 Abs. 2 BauGB je nach Rechtsgrundlage). Für eine
landesrechtliche Abweichung bietet die Liste zwei Codes nach der
Musterbauordnung (3: § 67 Abs. 1 MBO, 4: § 67 Abs. 2 Satz 2 MBO); welcher für
die Landesbauordnung passt, entscheidet das Büro im Feld „XBau-Code“. Das gilt
auch für Erleichterungen. Bis dahin nennt die Abgabe die möglichen Codes. Die MIME-Types-Liste erlaubt nur PDF, IFC (auch XML/ZIP) und BCF;
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
Nordrhein-Westfalen gibt es unter `urn:xoev-de:xbau:nw:codeliste:verfahrensart`
keine Liste im XRepository (Abfrage am 21.10.2026: HTTP 404; vorhanden sind
die Listen für BE, BW, BY, HE und HH). Ohne Liste enthält die XBau-Nachricht keine Verfahrensart; das ist
zulässig, weil das Element optional ist. Stellt das Land oder die Bauaufsicht
eine Liste bereit, wird sie wie oben eingespielt.

Eingebaut ist nur der Verzeichnisdienst (`DVDV`, Version 1).
