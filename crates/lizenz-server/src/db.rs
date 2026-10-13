//! Datenbankzugriff (PostgreSQL).

use chrono::{DateTime, Duration, NaiveDate, Utc};
use core_lib::license::Edition;
use core_lib::license::ticket::{Ablehnungsgrund, TicketAnfrage};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::schluessel::neuer_lizenzschluessel;

pub type DbResult<T> = Result<T, sqlx::Error>;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LizenzZeile {
    pub id: Uuid,
    pub schluessel: String,
    pub lizenznehmer: String,
    pub email: Option<String>,
    pub edition: String,
    pub arbeitsplaetze: i32,
    pub status: String,
    pub gueltig_bis: Option<NaiveDate>,
    pub wartung_bis: Option<NaiveDate>,
    pub stripe_kunde: Option<String>,
    pub stripe_abo: Option<String>,
    pub periode_bis: Option<DateTime<Utc>>,
    pub erstellt_am: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Aktivierung {
    pub machine_id: String,
    pub rechnername: String,
    pub app_version: String,
    pub aktiviert_am: DateTime<Utc>,
    pub zuletzt_am: DateTime<Utc>,
}

/// Neue Lizenz von Hand (Admin-API).
#[derive(Debug, Clone, Deserialize)]
pub struct NeueLizenz {
    pub lizenznehmer: String,
    pub email: Option<String>,
    pub edition: Edition,
    pub arbeitsplaetze: u32,
    pub gueltig_bis: Option<NaiveDate>,
    pub wartung_bis: Option<NaiveDate>,
}

/// Änderungen an einer Lizenz (Admin-API); fehlende Felder bleiben.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Lizenzaenderung {
    pub lizenznehmer: Option<String>,
    pub email: Option<String>,
    pub status: Option<Status>,
    pub edition: Option<Edition>,
    pub arbeitsplaetze: Option<u32>,
    pub gueltig_bis: Option<NaiveDate>,
    pub wartung_bis: Option<NaiveDate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Aktiv,
    Inaktiv,
    Gesperrt,
}

impl Status {
    pub fn als_text(self) -> &'static str {
        match self {
            Self::Aktiv => "aktiv",
            Self::Inaktiv => "inaktiv",
            Self::Gesperrt => "gesperrt",
        }
    }
}

pub fn edition_text(e: Edition) -> &'static str {
    match e {
        Edition::Einzelplatz => "einzelplatz",
        Edition::Buero => "buero",
        Edition::Cloud => "cloud",
    }
}

pub fn edition_aus_text(t: &str) -> Option<Edition> {
    serde_json::from_value(serde_json::Value::String(t.trim().to_lowercase())).ok()
}

/// Was Stripe über ein Abo meldet.
#[derive(Debug, Clone)]
pub struct AboStand {
    pub abo: String,
    pub kunde: Option<String>,
    pub aktiv: bool,
    pub arbeitsplaetze: u32,
    pub edition: Option<Edition>,
    pub periode_bis: Option<DateTime<Utc>>,
    /// `created` des Ereignisses (Sekunden seit 1970).
    pub stand: i64,
}

/// Was Stripe nach dem Bezahlen über den Kunden meldet.
#[derive(Debug, Clone)]
pub struct Kauf {
    pub checkout: String,
    pub abo: String,
    pub kunde: Option<String>,
    pub lizenznehmer: String,
    pub email: Option<String>,
    pub edition: Option<Edition>,
}

const SPALTEN: &str = "id, schluessel, lizenznehmer, email, edition, arbeitsplaetze, status, \
    gueltig_bis, wartung_bis, stripe_kunde, stripe_abo, periode_bis, erstellt_am";

#[derive(Clone)]
pub struct Db {
    pub pool: PgPool,
}

impl Db {
    pub async fn verbinden(url: &str) -> DbResult<Self> {
        Self::mit_optionen(url.parse()?).await
    }

    pub async fn mit_optionen(optionen: PgConnectOptions) -> DbResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect_with(optionen)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    /// Ticket für einen Rechner: prüft die Lizenz und belegt oder erneuert
    /// den Platz dieses Rechners. Atomar je Lizenz (`FOR UPDATE`), damit zwei
    /// Rechner nicht gleichzeitig den letzten Platz bekommen.
    pub async fn platz_vergeben(
        &self,
        anfrage: &TicketAnfrage,
        ticket_dauer: Duration,
    ) -> DbResult<Result<LizenzZeile, Ablehnungsgrund>> {
        let mut tx = self.pool.begin().await?;
        let lizenz: Option<LizenzZeile> = sqlx::query_as(&format!(
            "SELECT {SPALTEN} FROM lizenzen WHERE schluessel = $1 FOR UPDATE"
        ))
        .bind(&anfrage.lizenzschluessel)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(lizenz) = lizenz else {
            return Ok(Err(Ablehnungsgrund::Unbekannt));
        };
        let jetzt = Utc::now();
        match lizenz.status.as_str() {
            "aktiv" => {}
            "gesperrt" => return Ok(Err(Ablehnungsgrund::Gesperrt)),
            _ => return Ok(Err(Ablehnungsgrund::Inaktiv)),
        }
        if lizenz
            .gueltig_bis
            .is_some_and(|bis| bis < jetzt.date_naive())
        {
            return Ok(Err(Ablehnungsgrund::Inaktiv));
        }
        // Rechner, deren Ticket längst abgelaufen ist, geben ihren Platz frei.
        sqlx::query("DELETE FROM aktivierungen WHERE lizenz_id = $1 AND zuletzt_am < $2")
            .bind(lizenz.id)
            .bind(jetzt - ticket_dauer)
            .execute(&mut *tx)
            .await?;
        let (andere, bekannt): (i64, bool) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE machine_id <> $2), \
                    bool_or(machine_id = $2) IS TRUE \
             FROM aktivierungen WHERE lizenz_id = $1",
        )
        .bind(lizenz.id)
        .bind(&anfrage.machine_id)
        .fetch_one(&mut *tx)
        .await?;
        if !bekannt && andere >= i64::from(lizenz.arbeitsplaetze) {
            return Ok(Err(Ablehnungsgrund::KeinePlaetze));
        }
        sqlx::query(
            "INSERT INTO aktivierungen \
                (lizenz_id, machine_id, rechnername, app_version, aktiviert_am, zuletzt_am) \
             VALUES ($1, $2, $3, $4, $5, $5) \
             ON CONFLICT (lizenz_id, machine_id) DO UPDATE SET \
                rechnername = EXCLUDED.rechnername, app_version = EXCLUDED.app_version, \
                zuletzt_am = EXCLUDED.zuletzt_am",
        )
        .bind(lizenz.id)
        .bind(&anfrage.machine_id)
        .bind(kuerzen(&anfrage.rechnername, 100))
        .bind(kuerzen(&anfrage.app_version, 30))
        .bind(jetzt)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Ok(lizenz))
    }

    /// Platz eines Rechners freigeben. `false`, wenn Schlüssel oder Rechner
    /// unbekannt sind.
    pub async fn abmelden(&self, schluessel: &str, machine_id: &str) -> DbResult<bool> {
        let r = sqlx::query(
            "DELETE FROM aktivierungen a USING lizenzen l \
             WHERE a.lizenz_id = l.id AND l.schluessel = $1 AND a.machine_id = $2",
        )
        .bind(schluessel)
        .bind(machine_id)
        .execute(&self.pool)
        .await?;
        Ok(r.rows_affected() > 0)
    }

    // --- Verwaltung --------------------------------------------------------

    pub async fn lizenzen(&self) -> DbResult<Vec<LizenzZeile>> {
        sqlx::query_as(&format!(
            "SELECT {SPALTEN} FROM lizenzen ORDER BY erstellt_am DESC"
        ))
        .fetch_all(&self.pool)
        .await
    }

    pub async fn lizenz(&self, id: Uuid) -> DbResult<Option<LizenzZeile>> {
        sqlx::query_as(&format!("SELECT {SPALTEN} FROM lizenzen WHERE id = $1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn aktivierungen(&self, id: Uuid) -> DbResult<Vec<Aktivierung>> {
        sqlx::query_as(
            "SELECT machine_id, rechnername, app_version, aktiviert_am, zuletzt_am \
             FROM aktivierungen WHERE lizenz_id = $1 ORDER BY aktiviert_am",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn anlegen(&self, neu: &NeueLizenz) -> DbResult<LizenzZeile> {
        sqlx::query_as(&format!(
            "INSERT INTO lizenzen \
                (id, schluessel, lizenznehmer, email, edition, arbeitsplaetze, status, \
                 gueltig_bis, wartung_bis) \
             VALUES ($1, $2, $3, $4, $5, $6, 'aktiv', $7, $8) RETURNING {SPALTEN}"
        ))
        .bind(Uuid::new_v4())
        .bind(neuer_lizenzschluessel())
        .bind(neu.lizenznehmer.trim())
        .bind(&neu.email)
        .bind(edition_text(neu.edition))
        .bind(plaetze(neu.arbeitsplaetze))
        .bind(neu.gueltig_bis)
        .bind(neu.wartung_bis)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn aendern(&self, id: Uuid, a: &Lizenzaenderung) -> DbResult<Option<LizenzZeile>> {
        sqlx::query_as(&format!(
            "UPDATE lizenzen SET \
                lizenznehmer = COALESCE($2, lizenznehmer), \
                email = COALESCE($3, email), \
                status = COALESCE($4, status), \
                edition = COALESCE($5, edition), \
                arbeitsplaetze = COALESCE($6, arbeitsplaetze), \
                gueltig_bis = COALESCE($7, gueltig_bis), \
                wartung_bis = COALESCE($8, wartung_bis) \
             WHERE id = $1 RETURNING {SPALTEN}"
        ))
        .bind(id)
        .bind(&a.lizenznehmer)
        .bind(&a.email)
        .bind(a.status.map(Status::als_text))
        .bind(a.edition.map(edition_text))
        .bind(a.arbeitsplaetze.map(plaetze))
        .bind(a.gueltig_bis)
        .bind(a.wartung_bis)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn aktivierung_loeschen(&self, id: Uuid, machine_id: &str) -> DbResult<bool> {
        let r = sqlx::query("DELETE FROM aktivierungen WHERE lizenz_id = $1 AND machine_id = $2")
            .bind(id)
            .bind(machine_id)
            .execute(&self.pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    // --- Stripe ------------------------------------------------------------

    /// Merkt sich ein Ereignis. `false`, wenn es schon verarbeitet wurde.
    pub async fn ereignis_neu(
        tx: &mut Transaction<'_, Postgres>,
        id: &str,
        typ: &str,
    ) -> DbResult<bool> {
        let r = sqlx::query(
            "INSERT INTO stripe_ereignisse (id, typ) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(typ)
        .execute(&mut **tx)
        .await?;
        Ok(r.rows_affected() > 0)
    }

    /// Bezahlt: Lizenz anlegen oder (falls das Abo-Ereignis zuerst kam)
    /// um Kundendaten ergänzen.
    pub async fn kauf(
        tx: &mut Transaction<'_, Postgres>,
        k: &Kauf,
        standard_edition: Edition,
    ) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO lizenzen \
                (id, schluessel, lizenznehmer, email, edition, arbeitsplaetze, status, \
                 stripe_kunde, stripe_abo, stripe_checkout) \
             VALUES ($1, $2, $3, $4, COALESCE($5, $6), 1, 'aktiv', $7, $8, $9) \
             ON CONFLICT (stripe_abo) DO UPDATE SET \
                lizenznehmer = EXCLUDED.lizenznehmer, \
                email = COALESCE(EXCLUDED.email, lizenzen.email), \
                stripe_kunde = COALESCE(EXCLUDED.stripe_kunde, lizenzen.stripe_kunde), \
                stripe_checkout = EXCLUDED.stripe_checkout, \
                edition = COALESCE($5, lizenzen.edition)",
        )
        .bind(Uuid::new_v4())
        .bind(neuer_lizenzschluessel())
        .bind(k.lizenznehmer.trim())
        .bind(&k.email)
        .bind(k.edition.map(edition_text))
        .bind(edition_text(standard_edition))
        .bind(&k.kunde)
        .bind(&k.abo)
        .bind(&k.checkout)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Abo angelegt, geändert oder beendet. Ältere Ereignisse als das zuletzt
    /// angewandte werden übergangen; eine Sperre bleibt bestehen.
    pub async fn abo(
        tx: &mut Transaction<'_, Postgres>,
        a: &AboStand,
        standard_edition: Edition,
    ) -> DbResult<()> {
        let status = if a.aktiv { "aktiv" } else { "inaktiv" };
        sqlx::query(
            "INSERT INTO lizenzen \
                (id, schluessel, lizenznehmer, edition, arbeitsplaetze, status, \
                 stripe_kunde, stripe_abo, periode_bis, stripe_stand) \
             VALUES ($1, $2, '', COALESCE($3, $4), $5, $6, $7, $8, $9, $10) \
             ON CONFLICT (stripe_abo) DO UPDATE SET \
                arbeitsplaetze = EXCLUDED.arbeitsplaetze, \
                status = CASE WHEN lizenzen.status = 'gesperrt' THEN 'gesperrt' \
                              ELSE EXCLUDED.status END, \
                periode_bis = EXCLUDED.periode_bis, \
                stripe_stand = EXCLUDED.stripe_stand, \
                stripe_kunde = COALESCE(EXCLUDED.stripe_kunde, lizenzen.stripe_kunde), \
                edition = COALESCE($3, lizenzen.edition) \
             WHERE lizenzen.stripe_stand IS NULL OR lizenzen.stripe_stand <= EXCLUDED.stripe_stand",
        )
        .bind(Uuid::new_v4())
        .bind(neuer_lizenzschluessel())
        .bind(a.edition.map(edition_text))
        .bind(edition_text(standard_edition))
        .bind(plaetze(a.arbeitsplaetze))
        .bind(status)
        .bind(&a.kunde)
        .bind(&a.abo)
        .bind(a.periode_bis)
        .bind(a.stand)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Lizenz zu einer Stripe-Checkout-Sitzung (Seite nach dem Bezahlen).
    pub async fn lizenz_zu_checkout(&self, checkout: &str) -> DbResult<Option<LizenzZeile>> {
        sqlx::query_as(&format!(
            "SELECT {SPALTEN} FROM lizenzen WHERE stripe_checkout = $1"
        ))
        .bind(checkout)
        .fetch_optional(&self.pool)
        .await
    }
}

fn plaetze(n: u32) -> i32 {
    i32::try_from(n.max(1)).unwrap_or(i32::MAX)
}

fn kuerzen(text: &str, max: usize) -> String {
    text.trim().chars().take(max).collect()
}
