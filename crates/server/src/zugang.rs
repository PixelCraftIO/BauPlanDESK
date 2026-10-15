//! Verifizierte Bearer-Zugänge. Die Hashliste wird pro Anfrage neu gelesen (Widerruf).
use super::routen;
use axum::{
    Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use core_lib::{
    CoreServices,
    benutzer::{Benutzer, BenutzerId},
    domain::{DocumentId, ProjectId},
    storage::ContentHash,
    zugriff::ProjektRecht,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Zugang {
    pub benutzer_id: BenutzerId,
    pub token_sha256: ContentHash,
    pub gueltig_bis: Option<DateTime<Utc>>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Zugaenge {
    pub schema_version: u32,
    pub zugaenge: Vec<Zugang>,
}
impl Zugaenge {
    pub fn pruefen(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.zugaenge.len() > 100000 {
            return Err("Unbekanntes oder zu großes Zugangsformat".into());
        }
        let mut hashes = std::collections::HashSet::new();
        for z in &self.zugaenge {
            if !hashes.insert(&z.token_sha256) {
                return Err("Ein Zugangstoken darf nur einmal zugeordnet sein".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone)]
pub enum ZugangQuelle {
    Datei(PathBuf),
}
#[derive(Clone)]
struct ZugangState {
    core: CoreServices,
    quelle: ZugangQuelle,
}
impl ZugangQuelle {
    async fn laden(&self) -> Result<Zugaenge, ()> {
        match self {
            Self::Datei(p) => {
                let m = tokio::fs::metadata(p).await.map_err(|_| ())?;
                if m.len() > 1024 * 1024 {
                    return Err(());
                }
                let raw = tokio::fs::read(p).await.map_err(|_| ())?;
                let z: Zugaenge = serde_json::from_slice(&raw).map_err(|_| ())?;
                z.pruefen().map_err(|_| ())?;
                Ok(z)
            }
        }
    }
}
fn gleich(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}
async fn anmelden(s: &ZugangState, header: Option<String>) -> Result<Benutzer, StatusCode> {
    let token = header
        .as_deref()
        .and_then(|h| h.strip_prefix("Bearer "))
        .filter(|t| t.len() >= 32 && t.len() <= 512)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let hash = ContentHash::of(token.as_bytes());
    let liste = s
        .quelle
        .laden()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let z = liste
        .zugaenge
        .iter()
        .find(|z| {
            gleich(z.token_sha256.as_str(), hash.as_str())
                && z.gueltig_bis.is_none_or(|d| d > Utc::now())
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let b = s
        .core
        .repos
        .benutzer
        .get(z.benutzer_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    if !b.aktiv || b.anmeldename == "system" {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(b)
}
async fn pruefen(State(s): State<ZugangState>, mut r: Request, next: Next) -> Response {
    if r.uri().path() == "/health" {
        if s.quelle.laden().await.is_err() {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
        return next.run(r).await;
    }
    let header = r
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .map(str::to_owned);
    let b = match anmelden(&s, header).await {
        Ok(b) => b,
        Err(code) => {
            return (code, "Anmeldung erforderlich oder Zugang nicht verfügbar").into_response();
        }
    };
    let path = r
        .uri()
        .path()
        .strip_prefix("/api/v1/")
        .or_else(|| r.uri().path().strip_prefix("/api/"))
        .unwrap_or("");
    let teile: Vec<_> = path.split('/').collect();
    let id = teile.get(1).and_then(|s| Uuid::parse_str(s).ok());
    let projekt = match (teile.first().copied(), id) {
        (Some("projekte"), Some(id)) => Some(ProjectId(id)),
        (Some("dokumente"), Some(id)) => {
            match s.core.repos.documents.latest(DocumentId(id)).await {
                Ok(d) => Some(d.project_id),
                Err(_) => return StatusCode::NOT_FOUND.into_response(),
            }
        }
        (Some("einreichungen"), Some(id)) => match s.core.repos.einreichungen.laden(id).await {
            Ok(e) => Some(e.stand.projekt_id),
            Err(_) => return StatusCode::NOT_FOUND.into_response(),
        },
        _ => None,
    };
    if let Some(p) = projekt {
        let min = if teile.contains(&"mitglieder") {
            ProjektRecht::Leitung
        } else if r.method() == axum::http::Method::GET
            || (r.method() == axum::http::Method::POST
                && teile.len() == 3
                && teile[0] == "projekte"
                && ["druckausgabe", "abstandsplan", "abstandscad"].contains(&teile[2]))
        {
            ProjektRecht::Lesen
        } else {
            ProjektRecht::Bearbeiten
        };
        if s.core.projekt_zugriff(p, &b, min).await.is_err() {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    r.extensions_mut().insert(b);
    next.run(r).await
}
pub fn gesichert(core: CoreServices, quelle: ZugangQuelle) -> Router {
    let state = ZugangState {
        core: core.clone(),
        quelle,
    };
    routen(core).layer(middleware::from_fn_with_state(state, pruefen))
}
