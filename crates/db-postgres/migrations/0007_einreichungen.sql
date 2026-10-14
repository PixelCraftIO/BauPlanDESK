CREATE TABLE einreichungen (
    id UUID PRIMARY KEY NOT NULL,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version BIGINT NOT NULL,
    created_at TEXT NOT NULL,
    payload TEXT NOT NULL
);
CREATE INDEX idx_einreichungen_project ON einreichungen(project_id);
