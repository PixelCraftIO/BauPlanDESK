-- Datenstand ohne Tabellenänderung: Die Projektorganisation kann
-- „nicht_benoetigt“ (als nicht benötigt gekennzeichnete Navigationsbereiche)
-- führen. Ältere Versionen lesen das Feld nicht (deny_unknown_fields) und
-- stoppen deshalb beim Start mit dem Hinweis auf die neuere Datenbank, statt
-- beim Speichern der Organisation oder in der Planliste zu scheitern.
SELECT 1;
