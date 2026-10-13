-- Kundendatenbank (Bauherren, Bauträger, Investoren, Baufirmen …).
CREATE TABLE kunden (
    id              UUID PRIMARY KEY,
    typ             TEXT NOT NULL,
    kategorie       TEXT NOT NULL,
    kundennummer    TEXT NOT NULL DEFAULT '',
    vorname         TEXT NOT NULL DEFAULT '',
    familienname    TEXT NOT NULL DEFAULT '',
    firma           TEXT NOT NULL DEFAULT '',
    ansprechpartner TEXT NOT NULL DEFAULT '',
    strasse         TEXT NOT NULL DEFAULT '',
    hausnummer      TEXT NOT NULL DEFAULT '',
    postleitzahl    TEXT NOT NULL DEFAULT '',
    ort             TEXT NOT NULL DEFAULT '',
    telefon         TEXT NOT NULL DEFAULT '',
    email           TEXT NOT NULL DEFAULT '',
    notizen         TEXT NOT NULL DEFAULT '',
    version         BIGINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL
);

-- Kunden als Bauherren eines Projekts, in der Reihenfolge `position`.
-- Ein zugeordneter Kunde kann nicht gelöscht werden (Fremdschlüssel).
CREATE TABLE projekt_bauherren (
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    kunde_id   UUID NOT NULL REFERENCES kunden(id),
    position   INTEGER NOT NULL,
    PRIMARY KEY (project_id, kunde_id)
);

CREATE INDEX idx_projekt_bauherren_kunde ON projekt_bauherren(kunde_id);
