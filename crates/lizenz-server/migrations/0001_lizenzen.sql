-- Lizenzen: Abos aus Stripe und von Hand angelegte Lizenzen.
CREATE TABLE lizenzen (
    id              UUID PRIMARY KEY,
    -- Was der Kunde eingibt: BPIO-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX
    schluessel      TEXT NOT NULL UNIQUE,
    lizenznehmer    TEXT NOT NULL,
    email           TEXT,
    edition         TEXT NOT NULL,
    arbeitsplaetze  INTEGER NOT NULL CHECK (arbeitsplaetze > 0),
    -- aktiv | inaktiv (Abo beendet, Zahlung offen) | gesperrt (vom Hersteller)
    status          TEXT NOT NULL,
    -- Feste Laufzeit (von Hand angelegte Mietlizenzen); NULL = ohne Ende.
    gueltig_bis     DATE,
    wartung_bis     DATE,
    stripe_kunde    TEXT,
    stripe_abo      TEXT UNIQUE,
    stripe_checkout TEXT UNIQUE,
    -- Ende der bezahlten Abrechnungsperiode (nur zur Anzeige).
    periode_bis     TIMESTAMPTZ,
    -- Zeitstempel des zuletzt angewandten Stripe-Ereignisses: Stripe
    -- liefert nicht garantiert in Reihenfolge.
    stripe_stand    BIGINT,
    erstellt_am     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Ein Rechner (Machine-ID) je Platz. Wer sich länger als die Ticketdauer
-- nicht gemeldet hat, verliert den Platz automatisch (sein Ticket ist
-- dann ohnehin abgelaufen).
CREATE TABLE aktivierungen (
    lizenz_id    UUID NOT NULL REFERENCES lizenzen (id) ON DELETE CASCADE,
    machine_id   TEXT NOT NULL,
    rechnername  TEXT NOT NULL,
    app_version  TEXT NOT NULL,
    aktiviert_am TIMESTAMPTZ NOT NULL,
    zuletzt_am   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (lizenz_id, machine_id)
);

-- Bereits verarbeitete Stripe-Ereignisse (Stripe stellt mehrfach zu).
CREATE TABLE stripe_ereignisse (
    id           TEXT PRIMARY KEY,
    typ          TEXT NOT NULL,
    empfangen_am TIMESTAMPTZ NOT NULL DEFAULT now()
);
