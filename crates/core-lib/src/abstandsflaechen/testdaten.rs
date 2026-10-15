//! Fiktiver Kontrollfall, kein amtlicher Musterfall.
use super::*;
pub fn punkt(x: f64, y: f64) -> APunkt {
    APunkt { x, y }
}
pub fn rechteck(x: f64, y: f64, b: f64, h: f64) -> Vec<APunkt> {
    vec![
        punkt(x, y),
        punkt(x + b, y),
        punkt(x + b, y + h),
        punkt(x, y + h),
    ]
}
pub fn plan() -> AbstandsPlan {
    let g = Uuid::from_u128(1);
    AbstandsPlan{bezeichnung:"Neubau Musterstraße – Abstandsflächen".into(),regeln:katalog().unwrap().into_iter().find(|r|r.land==Bundesland::Nw).unwrap(),recht:AbstandsRecht{gemeinde:"Musterstadt".into(),fassung:"Für diesen fiktiven Kontrollfall bestätigter Ansatz".into(),anwendung:"Fiktiver Neubau; keine Übergangssituation".into(),gebiet:"WA".into(),satzung:"Im Kontrollfall keine abweichende Satzung".into(),planfestsetzung:"Keine abweichende Grenzbebauung im Kontrollfall".into(),geprueft:true,pruefvermerk:"Unabhängige Beispielprüfung: 8 m × 0,4 = 3,2 m; keine juristische Freigabe".into()},hoehenbezug:"Lokales Höhensystem, maßgebliches ebenes Gelände = 100 m".into(),koordinatenbezug:"Lokales kartesisches System, Meter".into(),grundstuecke:vec![AbstandsGrundstueck{id:g,bezeichnung:"Baugrundstück A".into(),flurstueck_ids:vec![],grenze:rechteck(0.,0.,20.,20.),innenringe:vec![],grundlage:"Fiktiver vermessener Kontrollumriss, rechtliche Einheit separat bestätigt".into()}],waende:vec![AbstandsWand{id:Uuid::from_u128(2),gebaeude:"Wohnhaus".into(),bezeichnung:"Südwand".into(),grundstueck_id:Some(g),von:punkt(5.,5.),bis:punkt(15.,5.),aussen:Aussenrichtung::Rechts,gelaende_a_m:Some(100.),gelaende_b_m:Some(100.),abschluss_a_m:Some(108.),abschluss_b_m:Some(108.),dachart:Dachart::Flach,dachhoehe_m:None,dachneigung_grad:None,dachfaktor_manuell:None,hoehenquelle:"Kontrollschnitt: Gelände 100 m, Wandabschluss 108 m".into(),dachgrundlage:String::new(),fall:Wandfall::Regel,grenzkennung:String::new(),nutzung_geprueft:false,sonderansatz:None,polygon_manuell:None}],flaechen:vec![],quellen:vec![],umfang_geprueft:true,umfang_vermerk:"Fiktiver Einzelwandtest; vollständiger Wandabschnitt einschließlich Gelände und Außenseite geprüft".into()}
}
