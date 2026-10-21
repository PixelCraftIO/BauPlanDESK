-- Fachrichtung eines Projektbeteiligten (z. B. „Tragwerksplanung“), für XBau
-- `weitereBeteiligte/fachrichtung`. Leer = nicht angegeben.
ALTER TABLE projekt_beteiligte ADD COLUMN fachrichtung TEXT NOT NULL DEFAULT '';
