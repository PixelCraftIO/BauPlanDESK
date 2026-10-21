//! Übernahme einer archivierten E-Mail als Kommunikation einer Verfahrensakte,
//! entsprechend der XBau-Übernahme ([`crate::xbau::postfach`]): bewusst,
//! wiederholbar und projektweit höchstens einmal je Nachrichtenversion. Fristen
//! und Nachforderungen werden nicht abgeleitet; sie bleiben fachliche Erfassung.
use super::*;
use crate::{
    benutzer::{Benutzer, Recht},
    commit::Fachstand,
    domain::ProjectId,
    plattform::objekte::Dokumentbezug,
    verfahren::{Bezug, Kommunikation, Kommunikationsart, Verfahrensakte},
    zugriff::ProjektRecht,
};
use chrono::NaiveDate;
use uuid::Uuid;

/// Kennung der Kommunikation, die aus einer E-Mail-Version entsteht.
pub fn kommunikation_id(e: &Dokumentbezug) -> Uuid {
    Uuid::new_v5(
        &e.dokument_id.0,
        format!("bauprojektio-email-kommunikation/{}", e.version).as_bytes(),
    )
}

fn adressen(liste: &[MailAdresse]) -> String {
    liste
        .iter()
        .map(|a| match (a.name.trim(), a.adresse.trim()) {
            ("", adr) => adr.to_string(),
            (name, "") => name.to_string(),
            (name, adr) => format!("{name} <{adr}>"),
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Kürzt auf höchstens `max` Bytes an einer Zeichengrenze.
fn kuerzen(text: &str, max: usize, zusatz: &str) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut ende = max.saturating_sub(zusatz.len());
    while !text.is_char_boundary(ende) {
        ende -= 1;
    }
    format!("{}{zusatz}", &text[..ende])
}

impl CoreServices {
    /// Übernimmt die E-Mail-Version `email` als Kommunikation in die Akte
    /// `verfahren`. Datum ist `datum` (bestätigter Zugang bzw. Versand), sonst
    /// das Sendedatum der E-Mail als Ortsdatum. Bereits als Projektunterlage
    /// abgelegte Anhänge werden als Bezug verknüpft.
    #[allow(clippy::too_many_arguments)]
    pub async fn email_in_verfahren(
        &self,
        p: ProjectId,
        email: Dokumentbezug,
        verfahren: Uuid,
        basis: i64,
        eingehend: bool,
        datum: Option<NaiveDate>,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        self.projekt_zugriff(p, wer, ProjektRecht::Bearbeiten)
            .await?;
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        let mail = self
            .archivierte_email(email.dokument_id, email.version)
            .await?;
        if mail.dokument.project_id != p {
            return Err(fehler("Die E-Mail gehört nicht zu diesem Projekt"));
        }
        let (stand, mut akte) = self.verfahrensakte(p, verfahren).await?;
        let c = kommunikation_id(&email);
        if akte.kommunikation.iter().any(|k| k.id == c) {
            return Ok(stand);
        }
        for f in self.repos.commit.fachstaende(p, "verfahren").await? {
            if let Ok(a) = serde_json::from_value::<Verfahrensakte>(f.daten)
                && a.kommunikation.iter().any(|k| k.id == c)
            {
                return Err(fehler(format!(
                    "Diese E-Mail steht bereits in der Verfahrensakte „{}“.",
                    a.bezeichnung
                )));
            }
        }
        if stand.revision != basis {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: stand.revision,
            });
        }
        let i = &mail.inhalt;
        let gesendet = i
            .datum
            .as_deref()
            .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
            .map(|d| crate::kalender::datum_deutsch(d.with_timezone(&chrono::Utc)));
        let heute = crate::kalender::heute();
        let mut hinweise = Vec::new();
        let datum = match (datum, gesendet) {
            (Some(d), Some(g)) if d < g => {
                return Err(fehler(format!(
                    "Das Datum liegt vor dem Sendedatum der E-Mail ({})",
                    g.format("%d.%m.%Y")
                )));
            }
            (Some(d), _) => d,
            (None, Some(g)) => g,
            (None, None) => {
                hinweise.push("Die E-Mail enthält kein lesbares Sendedatum; Datum ist der Tag der Archivierung.".to_string());
                crate::kalender::datum_deutsch(mail.dokument.created_at)
            }
        };
        if datum > heute {
            return Err(fehler("Das Datum darf nicht in der Zukunft liegen"));
        }
        // Bereits abgelegte Anhänge genau dieser Nachrichtenversion.
        let mut bezuege = Vec::new();
        let mut abgelegt = std::collections::BTreeMap::new();
        for d in self.list_documents(p).await? {
            if let Some(z) = self
                .repos
                .commit
                .dokumentzusatz(d.document_id, d.version, "emailquelle")
                .await?
                && z.daten["dokument_id"] == serde_json::json!(email.dokument_id)
                && z.daten["version"] == serde_json::json!(email.version)
                && let Some(index) = z.daten["anhang"].as_u64()
            {
                abgelegt.entry(index).or_insert(Dokumentbezug {
                    dokument_id: d.document_id,
                    version: d.version,
                });
            }
        }
        bezuege.extend(
            abgelegt
                .values()
                .cloned()
                .map(|dokument| Bezug::Dokument { dokument }),
        );
        let mut beteiligte = format!("Von: {}", adressen(&i.absender));
        for (feld, liste) in [("An", &i.empfaenger), ("Cc", &i.cc)] {
            if !liste.is_empty() {
                beteiligte.push_str(&format!("\n{feld}: {}", adressen(liste)));
            }
        }
        let mut text = i.text.trim().to_string();
        if !i.anhaenge.is_empty() {
            text.push_str("\n\nAnhänge:");
            for (n, a) in i.anhaenge.iter().enumerate() {
                text.push_str(&format!(
                    "\n- {}{}",
                    a.name,
                    if abgelegt.contains_key(&(n as u64)) {
                        " (als Projektunterlage abgelegt)"
                    } else {
                        ""
                    }
                ));
            }
        }
        for h in hinweise.iter().chain(&i.hinweise) {
            text.push_str(&format!("\n\nHinweis: {h}"));
        }
        let betreff: String = match i.betreff.trim() {
            "" => "E-Mail ohne Betreff".into(),
            b => b.chars().take(500).collect(),
        };
        akte.kommunikation.push(Kommunikation {
            id: c,
            art: Kommunikationsart::Email,
            eingehend,
            datum,
            betreff,
            beteiligte: kuerzen(&beteiligte, 4000, " …"),
            inhalt: kuerzen(
                &text,
                100_000,
                "\n… (gekürzt; vollständiger Text im Original)",
            ),
            original: Some(email),
            bezuege,
        });
        self.verfahren_commit(p, verfahren, basis, akte, wer, false)
            .await
    }
}
