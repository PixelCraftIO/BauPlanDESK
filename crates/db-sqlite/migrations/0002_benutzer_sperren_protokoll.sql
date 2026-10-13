CREATE TABLE benutzer (
    id          TEXT PRIMARY KEY NOT NULL,
    anmeldename TEXT NOT NULL UNIQUE,
    anzeigename TEXT NOT NULL,
    rolle       TEXT NOT NULL,
    aktiv       INTEGER NOT NULL DEFAULT 1,
    erstellt_am TEXT NOT NULL
);

-- Höchstens eine Sperre je Dokument (Primärschlüssel).
CREATE TABLE document_locks (
    document_id   TEXT PRIMARY KEY NOT NULL,
    project_id    TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    benutzer_id   TEXT NOT NULL,
    benutzer_name TEXT NOT NULL,
    seit          TEXT NOT NULL
);

CREATE INDEX idx_document_locks_project ON document_locks(project_id);

-- Bewusst ohne Fremdschlüssel: Das Protokoll überdauert gelöschte
-- Projekte und Benutzer und speichert deshalb auch den Namen.
CREATE TABLE protokoll (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    zeit          TEXT NOT NULL,
    benutzer_id   TEXT NOT NULL,
    benutzer_name TEXT NOT NULL,
    aktion        TEXT NOT NULL,
    project_id    TEXT,
    document_id   TEXT,
    details       TEXT NOT NULL
);

CREATE INDEX idx_protokoll_project ON protokoll(project_id, id);
