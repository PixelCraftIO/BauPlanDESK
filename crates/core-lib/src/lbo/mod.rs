//! Modulare LBO-Berechnungs-Engine.
//!
//! Die Engine (Rust) ist von den Rechtsgrundlagen (Rhai-Skripte, ein
//! "Regelpaket" je Bundesland) getrennt. Regelpakete werden zur Laufzeit
//! geladen und können jederzeit ersetzt werden – ein LBO-Update ist damit
//! eine neue `.rhai`-Datei, keine neue App-Version.
//!
//! ## Vertrag eines Regelpakets
//!
//! Jedes Skript muss folgende Funktionen definieren:
//!
//! ```text
//! fn meta()                  -> #{ land: "BY", version: "2026.1", rechtsstand: "…" }
//! fn abstandsflaeche(input)  -> #{ tiefe_m: 3.0, faktor: 0.4, mindesttiefe_m: 3.0,
//!                                  rechtsgrundlage: "Art. 6 Abs. 5 BayBO" }
//! ```
//!
//! `input` ist eine Map mit den Feldern von [`AbstandsflaecheInput`].
//!
//! ## Sandbox
//!
//! Skripte haben keinen Datei-, Netz- oder Modulzugriff und laufen mit
//! Begrenzungen für Operationen, Rekursionstiefe und Datengrößen.

mod rulepack;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use rhai::{AST, Dynamic, Engine, Scope};
use serde::{Deserialize, Serialize};

pub use rulepack::{RulePackInfo, RulePackSource};

use crate::calc::Baugebiet;
use crate::domain::Bundesland;
use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbstandsflaecheInput {
    /// Wandhöhe H in m (Geländeoberfläche bis Schnittpunkt Wand/Dachhaut)
    pub wandhoehe_m: f64,
    pub gebiet: Baugebiet,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbstandsflaecheErgebnis {
    pub tiefe_m: f64,
    pub faktor: f64,
    pub mindesttiefe_m: f64,
    pub rechtsgrundlage: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Berechnung<T> {
    pub ergebnis: T,
    /// Welches Regelpaket (Land + Version) das Ergebnis erzeugt hat – für
    /// Nachvollziehbarkeit im Bauantrag.
    pub regelpaket: RulePackInfo,
}

struct LoadedRulePack {
    info: RulePackInfo,
    ast: Arc<AST>,
}

pub struct LboEngine {
    engine: Engine,
    packs: RwLock<BTreeMap<Bundesland, LoadedRulePack>>,
}

impl Default for LboEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LboEngine {
    /// Erzeugt eine Engine mit Sandbox-Limits und der nativen Hilfs-API für
    /// Skripte. Es sind noch keine Regelpakete geladen.
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Sandbox: kein `import`, begrenzte Ressourcen.
        engine.set_module_resolver(rhai::module_resolvers::DummyModuleResolver::new());
        engine.set_max_operations(500_000);
        engine.set_max_call_levels(32);
        engine.set_max_expr_depths(64, 32);
        engine.set_max_string_size(64 * 1024);
        engine.set_max_array_size(10_000);
        engine.set_max_map_size(10_000);
        engine.disable_symbol("eval");

        // Native Hilfsfunktionen, die allen Regelpaketen zur Verfügung stehen.
        engine.register_fn("aufrunden", |wert: f64, stellen: i64| -> f64 {
            let f = 10f64.powi(stellen.clamp(0, 6) as i32);
            (wert * f - 1e-9).ceil() / f
        });

        Self {
            engine,
            packs: RwLock::new(BTreeMap::new()),
        }
    }

    /// Kompiliert ein Regelpaket, validiert dessen Metadaten und ersetzt ein
    /// ggf. bereits geladenes Paket desselben Landes atomar (Hot-Reload).
    pub fn load_rule_pack(&self, source: RulePackSource) -> CoreResult<RulePackInfo> {
        let ast = self
            .engine
            .compile(&source.script)
            .map_err(|e| CoreError::Script {
                land: source.land,
                message: format!("{} ({}): {e}", source.origin, e.position()),
            })?;

        let meta: rulepack::Meta = self.call(source.land, &ast, "meta", ())?;
        let info = RulePackInfo::from_meta(meta, &source)?;

        // Probelauf, damit ein fehlerhaftes Paket nie ein funktionierendes ersetzt.
        let probe = AbstandsflaecheInput {
            wandhoehe_m: 6.0,
            gebiet: Baugebiet::WA,
        };
        self.run_abstandsflaeche(source.land, &ast, &probe)?;

        self.packs.write().expect("LBO-Lock vergiftet").insert(
            source.land,
            LoadedRulePack {
                info: info.clone(),
                ast: Arc::new(ast),
            },
        );
        tracing::info!(land = ?info.land, version = %info.version, origin = %info.origin, "LBO-Regelpaket geladen");
        Ok(info)
    }

    /// Lädt alle `<land>.rhai`-Dateien eines Verzeichnisses (z. B. die mit der
    /// App ausgelieferten Ressourcen oder ein Update-Verzeichnis). Dateien, die
    /// keinem Bundesland zugeordnet werden können, werden ignoriert. Fehler
    /// einzelner Pakete werden gesammelt zurückgegeben, statt das Laden der
    /// übrigen Pakete abzubrechen.
    pub fn load_directory(&self, dir: &Path) -> CoreResult<(Vec<RulePackInfo>, Vec<CoreError>)> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| CoreError::InvalidInput(format!("{}: {e}", dir.display())))?;
        let mut loaded = Vec::new();
        let mut errors = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rhai") {
                continue;
            }
            let Some(land) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse::<Bundesland>().ok())
            else {
                continue;
            };
            let result = std::fs::read_to_string(&path)
                .map_err(|e| CoreError::InvalidInput(format!("{}: {e}", path.display())))
                .and_then(|script| {
                    self.load_rule_pack(RulePackSource {
                        land,
                        script,
                        origin: path.display().to_string(),
                    })
                });
            match result {
                Ok(info) => loaded.push(info),
                Err(e) => errors.push(e),
            }
        }
        loaded.sort_by_key(|i| i.land);
        Ok((loaded, errors))
    }

    pub fn rule_packs(&self) -> Vec<RulePackInfo> {
        self.packs
            .read()
            .expect("LBO-Lock vergiftet")
            .values()
            .map(|p| p.info.clone())
            .collect()
    }

    pub fn abstandsflaeche(
        &self,
        land: Bundesland,
        input: &AbstandsflaecheInput,
    ) -> CoreResult<Berechnung<AbstandsflaecheErgebnis>> {
        if !(input.wandhoehe_m.is_finite() && input.wandhoehe_m >= 0.0) {
            return Err(CoreError::InvalidInput("Wandhöhe muss ≥ 0 sein".into()));
        }
        // AST klonen (Arc) und Lock sofort freigeben, damit ein Hot-Reload
        // nicht auf laufende Berechnungen warten muss.
        let (ast, info) = {
            let packs = self.packs.read().expect("LBO-Lock vergiftet");
            let pack = packs.get(&land).ok_or(CoreError::RulePackMissing(land))?;
            (pack.ast.clone(), pack.info.clone())
        };
        let ergebnis = self.run_abstandsflaeche(land, &ast, input)?;
        Ok(Berechnung {
            ergebnis,
            regelpaket: info,
        })
    }

    fn run_abstandsflaeche(
        &self,
        land: Bundesland,
        ast: &AST,
        input: &AbstandsflaecheInput,
    ) -> CoreResult<AbstandsflaecheErgebnis> {
        let arg = rhai::serde::to_dynamic(input).map_err(|e| CoreError::Script {
            land,
            message: e.to_string(),
        })?;
        let ergebnis: AbstandsflaecheErgebnis = self.call(land, ast, "abstandsflaeche", (arg,))?;
        if !(ergebnis.tiefe_m.is_finite() && ergebnis.tiefe_m >= 0.0) {
            return Err(CoreError::Script {
                land,
                message: format!("unplausible Abstandsflächentiefe {}", ergebnis.tiefe_m),
            });
        }
        Ok(ergebnis)
    }

    fn call<T: serde::de::DeserializeOwned>(
        &self,
        land: Bundesland,
        ast: &AST,
        name: &str,
        args: impl rhai::FuncArgs,
    ) -> CoreResult<T> {
        let script_err = |message: String| CoreError::Script { land, message };
        let result: Dynamic = self
            .engine
            .call_fn(&mut Scope::new(), ast, name, args)
            .map_err(|e| script_err(format!("{name}(): {e}")))?;
        rhai::serde::from_dynamic(&result).map_err(|e| script_err(format!("{name}(): {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEISPIEL: &str = r#"
        fn meta() { #{ land: "BY", version: "test-1", rechtsstand: "Test" } }
        fn abstandsflaeche(input) {
            let faktor = if input.gebiet == "GE" { 0.2 } else { 0.4 };
            let tiefe = max(faktor * input.wandhoehe_m, 3.0);
            #{ tiefe_m: aufrunden(tiefe, 2), faktor: faktor, mindesttiefe_m: 3.0,
               rechtsgrundlage: "Test" }
        }
    "#;

    fn engine_mit(script: &str) -> CoreResult<LboEngine> {
        let engine = LboEngine::new();
        engine.load_rule_pack(RulePackSource {
            land: Bundesland::By,
            script: script.into(),
            origin: "test".into(),
        })?;
        Ok(engine)
    }

    #[test]
    fn berechnet_abstandsflaeche() {
        let engine = engine_mit(BEISPIEL).unwrap();
        let hoch = AbstandsflaecheInput {
            wandhoehe_m: 10.0,
            gebiet: Baugebiet::WA,
        };
        let r = engine.abstandsflaeche(Bundesland::By, &hoch).unwrap();
        assert!((r.ergebnis.tiefe_m - 4.0).abs() < 1e-9);
        assert_eq!(r.regelpaket.version, "test-1");

        let niedrig = AbstandsflaecheInput {
            wandhoehe_m: 5.0,
            gebiet: Baugebiet::WA,
        };
        let r = engine.abstandsflaeche(Bundesland::By, &niedrig).unwrap();
        assert!((r.ergebnis.tiefe_m - 3.0).abs() < 1e-9);
    }

    #[test]
    fn fehlendes_paket() {
        let engine = LboEngine::new();
        let input = AbstandsflaecheInput {
            wandhoehe_m: 5.0,
            gebiet: Baugebiet::WA,
        };
        assert!(matches!(
            engine.abstandsflaeche(Bundesland::Nw, &input),
            Err(CoreError::RulePackMissing(Bundesland::Nw))
        ));
    }

    #[test]
    fn falsches_land_im_paket_wird_abgelehnt() {
        let script = BEISPIEL.replace("land: \"BY\"", "land: \"NW\"");
        assert!(engine_mit(&script).is_err());
    }

    #[test]
    fn endlosschleife_wird_abgebrochen() {
        let script = BEISPIEL.replace("let faktor", "loop {} let faktor");
        assert!(engine_mit(&script).is_err());
    }

    #[test]
    fn fehlerhaftes_update_behaelt_altes_paket() {
        let engine = engine_mit(BEISPIEL).unwrap();
        let kaputt = engine.load_rule_pack(RulePackSource {
            land: Bundesland::By,
            script: "fn meta() { 42 }".into(),
            origin: "kaputt".into(),
        });
        assert!(kaputt.is_err());
        assert_eq!(engine.rule_packs()[0].version, "test-1");
    }

    #[test]
    fn mitgelieferte_regelpakete_laden() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lbo-rules");
        let engine = LboEngine::new();
        let (loaded, errors) = engine.load_directory(&dir).unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        assert!(!loaded.is_empty());
    }
}
