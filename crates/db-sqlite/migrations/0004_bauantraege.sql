-- Angaben zum Bauantrag (XBau) je Projekt, als JSON.
CREATE TABLE bauantraege (
    project_id    TEXT PRIMARY KEY NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    daten         TEXT NOT NULL,
    version       INTEGER NOT NULL,
    geaendert_von TEXT NOT NULL,
    geaendert_am  TEXT NOT NULL
);
