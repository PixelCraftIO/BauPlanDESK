//! Bürostandard getrennt von Projektberechnungen. Jede Ausgabe kann ihn überschreiben.
use super::*;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default, deny_unknown_fields)]
pub struct Druckgestaltung {
    pub kopfzeile: String,
    pub fusszeile: String,
    pub anschreiben: String,
}
impl Default for Druckgestaltung {
    fn default() -> Self {
        Self {
            kopfzeile: "{Projekt}".into(),
            fusszeile: "Stand: {Datum}".into(),
            anschreiben: "Sehr geehrte Damen und Herren,\n\nanbei erhalten Sie die im Inhaltsverzeichnis aufgeführten Unterlagen zum Bauvorhaben zur Prüfung.\n\nMit freundlichen Grüßen".into(),
        }
    }
}
impl Druckgestaltung {
    pub fn validate(&self) -> CoreResult<()> {
        if self.kopfzeile.len() > 1000
            || self.fusszeile.len() > 1000
            || self.anschreiben.len() > 16000
        {
            return Err(fehler(
                "Kopf-/Fußzeile höchstens 1.000, Anschreiben höchstens 16.000 Zeichen",
            ));
        }
        for t in [&self.kopfzeile, &self.fusszeile, &self.anschreiben] {
            if t.chars()
                .any(|c| c.is_control() && !['\n', '\r', '\t'].contains(&c))
            {
                return Err(fehler("Nicht druckbare Steuerzeichen im Briefbogen"));
            }
        }
        for (text, breite) in [(&self.kopfzeile, 499.), (&self.fusszeile, 394.)] {
            let probe = text
                .replace("{Projekt}", "Bauvorhaben")
                .replace("{Datum}", "16.10.2026")
                .replace("{Seite}", "300")
                .replace("{Seiten}", "300");
            if super::pdf::umbruch(&probe, breite, 9.)?.len() > 3 {
                return Err(fehler("Kopf-/Fußzeile auf maximal drei Druckzeilen kürzen"));
            }
        }
        Ok(())
    }
}
impl CoreServices {
    pub async fn druckgestaltung(&self) -> CoreResult<Druckgestaltung> {
        let g = self
            .repos
            .lizenz
            .einstellung("druckgestaltung-1")
            .await?
            .map(|s| serde_json::from_str::<Druckgestaltung>(&s).map_err(fehler))
            .transpose()?
            .unwrap_or_default();
        g.validate()?;
        Ok(g)
    }
    pub async fn druckgestaltung_speichern(
        &self,
        g: Druckgestaltung,
        wer: &crate::benutzer::Benutzer,
    ) -> CoreResult<Druckgestaltung> {
        self.pruefe(wer, crate::benutzer::Recht::BueroangabenAendern)
            .await?;
        g.validate()?;
        self.repos
            .lizenz
            .einstellung_setzen(
                "druckgestaltung-1",
                &serde_json::to_string(&g).map_err(fehler)?,
            )
            .await?;
        Ok(g)
    }
}
