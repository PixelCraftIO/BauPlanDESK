-- Einstellungen der Installation, z. B. die eingespielte Lizenzdatei.
CREATE TABLE einstellungen (
    schluessel TEXT PRIMARY KEY,
    wert       TEXT NOT NULL
);

-- Belegte Arbeitsplätze.
CREATE TABLE arbeitsplaetze (
    arbeitsplatz   TEXT PRIMARY KEY,
    benutzer_name  TEXT NOT NULL,
    belegt_seit    TIMESTAMPTZ NOT NULL,
    letzte_meldung TIMESTAMPTZ NOT NULL
);
