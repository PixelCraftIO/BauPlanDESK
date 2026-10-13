-- Einstellungen der Installation, z. B. die eingespielte Lizenzdatei.
CREATE TABLE einstellungen (
    schluessel TEXT PRIMARY KEY NOT NULL,
    wert       TEXT NOT NULL
);

-- Belegte Arbeitsplätze. Zeiten in Millisekunden seit 1970 (UTC), damit
-- der Vergleich auf „abgelaufen“ unabhängig vom Textformat ist.
CREATE TABLE arbeitsplaetze (
    arbeitsplatz   TEXT PRIMARY KEY NOT NULL,
    benutzer_name  TEXT NOT NULL,
    belegt_seit    INTEGER NOT NULL,
    letzte_meldung INTEGER NOT NULL
);
