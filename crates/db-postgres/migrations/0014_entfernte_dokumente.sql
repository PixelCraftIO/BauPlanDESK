CREATE TABLE entfernte_dokumente (document_id UUID PRIMARY KEY NOT NULL, project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE);
