CREATE TABLE benutzer (
    id          UUID PRIMARY KEY,
    anmeldename TEXT NOT NULL UNIQUE,
    anzeigename TEXT NOT NULL,
    rolle       TEXT NOT NULL,
    aktiv       BOOLEAN NOT NULL DEFAULT TRUE,
    erstellt_am TIMESTAMPTZ NOT NULL
);

-- Höchstens eine Sperre je Dokument (Primärschlüssel).
CREATE TABLE document_locks (
    document_id   UUID PRIMARY KEY,
    project_id    UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    benutzer_id   UUID NOT NULL,
    benutzer_name TEXT NOT NULL,
    seit          TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_document_locks_project ON document_locks(project_id);

-- Bewusst ohne Fremdschlüssel: Das Protokoll überdauert gelöschte
-- Projekte und Benutzer und speichert deshalb auch den Namen.
CREATE TABLE protokoll (
    id            BIGSERIAL PRIMARY KEY,
    zeit          TIMESTAMPTZ NOT NULL,
    benutzer_id   UUID NOT NULL,
    benutzer_name TEXT NOT NULL,
    aktion        TEXT NOT NULL,
    project_id    UUID,
    document_id   UUID,
    details       TEXT NOT NULL
);

CREATE INDEX idx_protokoll_project ON protokoll(project_id, id);
