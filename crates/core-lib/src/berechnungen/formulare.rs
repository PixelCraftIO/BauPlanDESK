use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Auswahl {
    pub wert: String,
    pub titel: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Feld {
    pub name: String,
    pub titel: String,
    pub typ: String,
    #[serde(default)]
    pub optionen: Vec<Auswahl>,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub gruppe: String,
    #[serde(default)]
    pub referenz: Option<String>,
    #[serde(default)]
    pub groesse: Option<Messgroesse>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct FormTabelle {
    pub pfad: String,
    pub titel: String,
    pub gruppe: String,
    pub felder: Vec<Feld>,
    #[ts(type = "JsonValue")]
    pub leer: serde_json::Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct BerechnungsVorlage {
    pub id: String,
    pub titel: String,
    pub hinweis: String,
    pub plan: FachPlan,
    pub felder: Vec<Feld>,
    pub tabellen: Vec<FormTabelle>,
}
pub fn vorlagen() -> CoreResult<Vec<BerechnungsVorlage>> {
    let daten = [
        include_str!("../../../../regelwerke/fachberechnungen/stellplaetze.json"),
        include_str!("../../../../regelwerke/fachberechnungen/wohnflaechen.json"),
        include_str!("../../../../regelwerke/fachberechnungen/din277.json"),
        include_str!("../../../../regelwerke/fachberechnungen/vollgeschosse.json"),
        include_str!("../../../../regelwerke/fachberechnungen/hoehen.json"),
        include_str!("../../../../regelwerke/fachberechnungen/freiflaechen.json"),
        include_str!("../../../../regelwerke/fachberechnungen/entwaesserung.json"),
        include_str!("../../../../regelwerke/fachberechnungen/ueberbaubarkeit.json"),
    ];
    daten
        .into_iter()
        .map(|d| serde_json::from_str(d).map_err(|e| err(e.to_string())))
        .collect()
}
