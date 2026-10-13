CREATE TABLE projects (
    id                     TEXT PRIMARY KEY NOT NULL,
    name                   TEXT NOT NULL,
    bundesland             TEXT NOT NULL,
    grundstuecksflaeche_m2 REAL,
    version                INTEGER NOT NULL DEFAULT 1,
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL
);

CREATE TABLE document_versions (
    document_id TEXT NOT NULL,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version     INTEGER NOT NULL,
    file_name   TEXT NOT NULL,
    storage_key TEXT NOT NULL,
    sha256      TEXT NOT NULL,
    size_bytes  INTEGER NOT NULL,
    created_by  TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    PRIMARY KEY (document_id, version)
);

CREATE INDEX idx_document_versions_project ON document_versions(project_id);
