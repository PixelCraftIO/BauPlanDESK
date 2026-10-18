-- Anmeldung mit Kennwort (Netzwerkbetrieb). Bestehende Benutzer behalten
-- ihre Daten und haben zunächst kein Kennwort (kennwort_hash NULL).
-- Gespeichert wird nur der Argon2id-Hash im PHC-Format.
ALTER TABLE benutzer ADD COLUMN kennwort_hash TEXT;
-- TRUE = Erstkennwort eines Administrators, muss geändert werden.
ALTER TABLE benutzer ADD COLUMN kennwort_aendern BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE benutzer ADD COLUMN kennwort_geaendert_am TIMESTAMPTZ;
ALTER TABLE benutzer ADD COLUMN fehlversuche INTEGER NOT NULL DEFAULT 0;
ALTER TABLE benutzer ADD COLUMN gesperrt_bis TIMESTAMPTZ;
ALTER TABLE benutzer ADD COLUMN letzte_anmeldung TIMESTAMPTZ;
