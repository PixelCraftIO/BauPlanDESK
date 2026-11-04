-- Datenstand ohne Tabellenänderung: Flurstücke können „Fläche wird im Aufmaß
-- ermittelt“ (flaeche_offen) führen. Ältere Versionen lesen das Feld nicht
-- (deny_unknown_fields) und stoppen deshalb beim Start mit dem Hinweis auf
-- die neuere Datenbank, statt beim Öffnen eines Projekts zu scheitern.
SELECT 1;
