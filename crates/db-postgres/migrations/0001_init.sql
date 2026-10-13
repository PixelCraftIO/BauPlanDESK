CREATE TABLE projects (
    id                     UUID PRIMARY KEY,
    name                   TEXT NOT NULL,
    bundesland             TEXT NOT NULL,
    grundstuecksflaeche_m2 DOUBLE PRECISION,
    version                BIGINT NOT NULL DEFAULT 1,
    created_at             TIMESTAMPTZ NOT NULL,
    updated_at             TIMESTAMPTZ NOT NULL
);

CREATE TABLE document_versions (
    document_id UUID NOT NULL,
    project_id  UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version     BIGINT NOT NULL,
    file_name   TEXT NOT NULL,
    storage_key TEXT NOT NULL,
    sha256      CHAR(64) NOT NULL,
    size_bytes  BIGINT NOT NULL,
    created_by  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (document_id, version)
);

CREATE INDEX idx_document_versions_project ON document_versions(project_id);
