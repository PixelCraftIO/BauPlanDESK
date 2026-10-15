CREATE TABLE fachhistorie (
 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 modul TEXT NOT NULL,
 objekt TEXT NOT NULL,
 schema_version BIGINT NOT NULL,
 revision BIGINT NOT NULL,
 payload TEXT NOT NULL,
 PRIMARY KEY(project_id,modul,objekt,revision)
);
INSERT INTO fachhistorie SELECT project_id,modul,objekt,schema_version,revision,payload FROM fachstaende;
