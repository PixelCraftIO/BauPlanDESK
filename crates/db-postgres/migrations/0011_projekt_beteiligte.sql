-- Kunden in weiteren Rollen eines Projekts (Grundstückseigentümer,
-- Erbbauberechtigter, Bevollmächtigter, Fachplaner), je Rolle in der
-- Reihenfolge `position`. Ein zugeordneter Kunde kann nicht gelöscht werden.
CREATE TABLE projekt_beteiligte (
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    kunde_id   UUID NOT NULL REFERENCES kunden(id),
    rolle      TEXT NOT NULL,
    position   INTEGER NOT NULL,
    PRIMARY KEY (project_id, rolle, kunde_id)
);

CREATE INDEX idx_projekt_beteiligte_kunde ON projekt_beteiligte(kunde_id);
