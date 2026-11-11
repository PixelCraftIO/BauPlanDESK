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
    // Kalender-Abo: der Handler prüft den persönlichen Schlüssel selbst.
    if r.uri().path().starts_with("/kalender/") && r.method() == axum::http::Method::GET {
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
    // Gleiche Sicht wie der Router: axum dekodiert Pfadparameter vor dem
    // Handler. Die Prüfung arbeitet daher auf den dekodierten Segmenten, sonst
    // umgeht eine prozentkodierte ID (`%33…`) die Projektprüfung.
    let roh = r.uri().path();
    let path = roh
        .strip_prefix("/api/v1/")
        .or_else(|| roh.strip_prefix("/api/"))
        .unwrap_or("");
    let Some(teile) = path
        .split('/')
        .map(prozent_dekodieren)
        .collect::<Option<Vec<String>>>()
    else {
        return (StatusCode::BAD_REQUEST, "Ungültig kodierter Pfad").into_response();
    };
    let teile: Vec<&str> = teile.iter().map(String::as_str).collect();
    let id = match (teile.first().copied(), teile.get(1)) {
        (Some("projekte" | "dokumente" | "einreichungen"), Some(s)) => match Uuid::parse_str(s) {
            Ok(id) => Some(id),
            // Ohne lesbare ID ist keine Projektprüfung möglich: nicht durchlassen.
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Ungültige Kennung im Pfad").into_response();
            }
        },
        _ => None,
    };
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
        } else if (r.method() == axum::http::Method::POST
            && teile.len() == 7
            && teile[0] == "projekte"
            && teile[2] == "verfahren"
            && teile[4] == "portale"
            && teile[6] == "pruefen")
            || (r.method() == axum::http::Method::POST
                && teile.len() == 4
                && teile[0] == "projekte"
                && teile[2] == "xbau"
                && teile[3] == "pruefen")
            || r.method() == axum::http::Method::GET
            || (r.method() == axum::http::Method::POST
                && teile.len() == 3
                && teile[0] == "projekte"
                && [
                    "druckausgabe",
                    "bescheidfristen",
                    "abstandsplan",
                    "abstandscad",
                    "abstandscadhoehen",
                    "abstandsumrisse",
                    "abstandsgelaende",
                    "fachberechnung",
                    "planaufmass",
                    "planwerte",
                    "planraumbuch",
                    "planabstand",
                    "plangebaeude",
                    "raumbuchimport",
                    "raumbuchstand",
                    "gebaeudegrundlagen",
                ]
                .contains(&teile[2]))
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
/// Dekodiert ein Pfadsegment wie axum (`%XX`, UTF-8). `None` bei ungültiger
/// Kodierung; `+` bleibt im Pfad unverändert.
fn prozent_dekodieren(segment: &str) -> Option<String> {
    let b = segment.as_bytes();
    if !b.contains(&b'%') {
        return Some(segment.to_owned());
    }
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = b.get(i + 1..i + 3)?;
            if !hex.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            let wert = u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
            out.push(wert);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}
pub fn gesichert(core: CoreServices, quelle: ZugangQuelle) -> Router {
    let state = ZugangState {
        core: core.clone(),
        quelle,
    };
    routen(core).layer(middleware::from_fn_with_state(state, pruefen))
}
