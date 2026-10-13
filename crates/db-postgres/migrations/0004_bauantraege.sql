-- Angaben zum Bauantrag (XBau) je Projekt, als JSON.
CREATE TABLE bauantraege (
    project_id    UUID PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    daten         JSONB NOT NULL,
    version       BIGINT NOT NULL,
    geaendert_von TEXT NOT NULL,
    geaendert_am  TIMESTAMPTZ NOT NULL
);
