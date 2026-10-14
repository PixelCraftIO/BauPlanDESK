CREATE TABLE dokumentkontexte (
 document_id UUID NOT NULL,
 version BIGINT NOT NULL,
 modul TEXT NOT NULL,
 schema_version BIGINT NOT NULL,
 payload TEXT NOT NULL,
 PRIMARY KEY(document_id,version,modul),
 FOREIGN KEY(document_id,version) REFERENCES document_versions(document_id,version) ON DELETE CASCADE
);
