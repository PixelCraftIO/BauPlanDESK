-- Ableitbarer Suchindex: ausschließlich normalisierte neue Suchtextstände.
-- Altbestände ohne Normalisierung bleiben über den kompatiblen Suchpfad lesbar.
CREATE VIRTUAL TABLE suchtext_fts USING fts5(text, project_id UNINDEXED, objekt UNINDEXED, tokenize='trigram case_sensitive 1');
INSERT INTO suchtext_fts(text,project_id,objekt)
  SELECT json_extract(payload,'$._suchtext_norm'),project_id,objekt FROM fachstaende
  WHERE modul='suchtexte' AND json_type(payload,'$._suchtext_norm')='text';
CREATE TRIGGER suchtext_insert AFTER INSERT ON fachstaende
WHEN NEW.modul='suchtexte' AND json_type(NEW.payload,'$._suchtext_norm')='text'
BEGIN INSERT INTO suchtext_fts(text,project_id,objekt) VALUES(json_extract(NEW.payload,'$._suchtext_norm'),NEW.project_id,NEW.objekt); END;
CREATE TRIGGER suchtext_update AFTER UPDATE ON fachstaende WHEN NEW.modul='suchtexte'
BEGIN
 DELETE FROM suchtext_fts WHERE project_id=OLD.project_id AND objekt=OLD.objekt;
 INSERT INTO suchtext_fts(text,project_id,objekt) SELECT json_extract(NEW.payload,'$._suchtext_norm'),NEW.project_id,NEW.objekt WHERE json_type(NEW.payload,'$._suchtext_norm')='text';
END;
CREATE TRIGGER suchtext_delete AFTER DELETE ON fachstaende WHEN OLD.modul='suchtexte'
BEGIN DELETE FROM suchtext_fts WHERE project_id=OLD.project_id AND objekt=OLD.objekt; END;
