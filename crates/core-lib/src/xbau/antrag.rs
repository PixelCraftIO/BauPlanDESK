//! Antragsnachrichten: `baugenehmigung.antrag.0200`, `vorbescheid.antrag.0210`,
//! `genehmigungsfreistellung.anzeige.0600` und die geänderten Fassungen 0202/0212.
//!
//! Die XBau-Schemas mischen qualifizierte Elemente (Präfix `xbau:`) und
//! unqualifizierte (ohne Namensraum, u. a. alles aus dem Kernmodul). Welches
//! Element wie zu schreiben ist, steht in den Schemas; die Tests prüfen die
//! Ausgabe mit `xmllint` gegen sie.

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use uuid::Uuid;

use super::codeliste::{ANLAGEN, Codelisten, MIMETYPES, VERZEICHNISDIENST};
use super::inhalt::{self, Vorhabenangaben};
use super::xml::XmlSchreiber;
use super::{
    Anschrift, Bauantrag, Beteiligter, Buero, Kommunikation, XBAU_VERSION, land_schluessel,
    mime_typ,
};
use crate::domain::Project;
use crate::error::{CoreError, CoreResult};

const NAMENSRAUM: &str = "http://www.xleitstelle.de/xbau/2/7";
/// Version der Destatis-Liste der Bundesländer.
const BUNDESLAND_VERSION: &str = "2010-04-01";

/// Eine Anlage, wie sie im Paket liegt.
#[derive(Debug, Clone, PartialEq)]
pub struct Anlagendatei {
    pub dokumentid: Uuid,
    pub bezeichnung: String,
    pub versionsnummer: i64,
    pub datum: NaiveDate,
    pub anlageart: String,
    /// Dateiname im Paket (eindeutig).
    pub dateiname: String,
}

pub struct Nachrichtendaten<'a> {
    pub projekt: &'a Project,
    pub antrag: &'a Bauantrag,
    /// Die dem Projekt zugeordneten Bauherren, siehe [`crate::kunden`].
    pub bauherren: &'a [Beteiligter],
    pub buero: &'a Buero,
    /// Weitere Beteiligte, Gebäudeklasse, Nachtragsbezug und BauNVO-Flächen.
    pub vorhaben: &'a Vorhabenangaben,
    pub anlagen: &'a [Anlagendatei],
    pub codelisten: &'a Codelisten,
    pub nachricht_id: Uuid,
    pub zeitpunkt: DateTime<Utc>,
}

/// Welche Antragsnachricht geschrieben wird. Die Verfahrensart der Akte legt sie
/// fest, siehe [`crate::verfahren::Verfahrensart`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Antragsart<'a> {
    /// `baugenehmigung.antrag.0200`, geändert `baugenehmigung.antragGeaendert.0202`.
    Baugenehmigung,
    /// `vorbescheid.antrag.0210`, geändert `vorbescheid.antragGeaendert.0212`.
    /// Jede Frage wird ein `beantragterVorbescheid`, jede Zeile ein Textabsatz.
    Vorbescheid { fragen: &'a [String] },
    /// `genehmigungsfreistellung.anzeige.0600`. XBau kennt keine geänderte Anzeige.
    Genehmigungsfreistellung { als_bauantrag_weiterfuehren: bool },
}

impl Antragsart<'_> {
    /// Wurzelelement der Nachricht (ohne Präfix).
    pub fn element(&self, geaendert: bool) -> CoreResult<&'static str> {
        let w = self.wurzel(geaendert)?;
        Ok(w.strip_prefix("xbau:").unwrap_or(w))
    }

    fn wurzel(&self, geaendert: bool) -> CoreResult<&'static str> {
        Ok(match (self, geaendert) {
            (Self::Baugenehmigung, false) => "xbau:baugenehmigung.antrag.0200",
            (Self::Baugenehmigung, true) => "xbau:baugenehmigung.antragGeaendert.0202",
            (Self::Vorbescheid { .. }, false) => "xbau:vorbescheid.antrag.0210",
            (Self::Vorbescheid { .. }, true) => "xbau:vorbescheid.antragGeaendert.0212",
            (Self::Genehmigungsfreistellung { .. }, false) => {
                "xbau:genehmigungsfreistellung.anzeige.0600"
            }
            (Self::Genehmigungsfreistellung { .. }, true) => {
                return Err(CoreError::InvalidInput(
                    "XBau sieht für die Genehmigungsfreistellung keine geänderte Anzeige vor. \
                     Unterlagen ohne XBau-Bezug nachreichen."
                        .into(),
                ));
            }
        })
    }

    /// Dateiname der Nachricht im Paket.
    pub fn dateiname(&self, geaendert: bool) -> CoreResult<String> {
        Ok(format!("{}.xml", self.element(geaendert)?))
    }
}

/// Erzeugt die Antragsnachricht. Prüft vorher die Pflichtangaben und die
/// Codes gegen die Codelisten.
pub fn baugenehmigung_antrag(d: &Nachrichtendaten<'_>) -> CoreResult<String> {
    antrag(d, Antragsart::Baugenehmigung)
}

/// Geänderte Bauvorhabendaten und ausgewählte Anlagen als vollständiger Datensatz.
pub fn baugenehmigung_aenderung(
    d: &Nachrichtendaten<'_>,
    bezug: &super::austausch::Nachrichtenkopf,
) -> CoreResult<String> {
    antrag_geaendert(d, Antragsart::Baugenehmigung, bezug)
}

/// Erzeugt die Antragsnachricht der gewählten Art.
pub fn antrag(d: &Nachrichtendaten<'_>, art: Antragsart<'_>) -> CoreResult<String> {
    schreiben(d, art, None)
}

/// Geänderte Daten zu einem gestellten Antrag. Bezug ist der eigene Antrag
/// derselben Art oder eine behördliche Befundliste (1140) dazu.
pub fn antrag_geaendert(
    d: &Nachrichtendaten<'_>,
    art: Antragsart<'_>,
    bezug: &super::austausch::Nachrichtenkopf,
) -> CoreResult<String> {
    art.element(true)?;
    let ursprung = art.element(false)?;
    let erwartet = match bezug.element.as_str() {
        e if e == ursprung && bezug.namensraum == super::austausch::HOCHBAU => &bezug.empfaenger,
        "prozessnachrichten.formellePruefungBefundliste.1140"
            if bezug.namensraum == super::austausch::KERN =>
        {
            &bezug.absender
        }
        _ => {
            return Err(super::austausch::fehler(
                "Ungeeignete Bezugsnachricht für eine Nachreichung",
            ));
        }
    };
    if bezug.antragsreferenz != Some(d.antrag.vorgang)
        || erwartet != d.antrag.bauaufsicht.kennung.trim()
    {
        return Err(super::austausch::fehler(
            "Antragsreferenz oder Behörde der Nachreichung passen nicht zum gespeicherten Bauantrag",
        ));
    }
    schreiben(d, art, Some(bezug))
}

fn schreiben(
    d: &Nachrichtendaten<'_>,
    art: Antragsart<'_>,
    bezug: Option<&super::austausch::Nachrichtenkopf>,
) -> CoreResult<String> {
    let wurzel = art.wurzel(bezug.is_some())?;
    let element = art.element(bezug.is_some())?;
    let mut fehlt = d.antrag.fehlende_angaben(d.buero, d.bauherren);
    inhalt::fehlende(d.vorhaben, &mut fehlt);
    for a in d.anlagen {
        if a.anlageart.trim().is_empty() {
            fehlt.push(format!(
                "Art der Anlage für „{}“ (im Bauantrag unter Anlagen aufnehmen und die Art wählen)",
                a.bezeichnung.trim()
            ));
        }
    }
    let fragen: Vec<Vec<&str>> = match art {
        Antragsart::Vorbescheid { fragen } => fragen
            .iter()
            .map(|f| f.lines().map(str::trim).filter(|z| !z.is_empty()).collect())
            .filter(|f: &Vec<&str>| !f.is_empty())
            .collect(),
        _ => vec![],
    };
    if matches!(art, Antragsart::Vorbescheid { .. }) && fragen.is_empty() {
        fehlt.push("Frage zum Vorbescheid".into());
    }
    if !fehlt.is_empty() {
        return Err(CoreError::InvalidInput(format!(
            "Für den XBau-Export fehlen noch: {}.",
            fehlt.join(", ")
        )));
    }
    let anlagen = d.codelisten.benoetigt(ANLAGEN, "XBau Anlage Art")?;
    let mimes = d.codelisten.benoetigt(MIMETYPES, "XBau MIME-Types")?;
    let verzeichnis = d
        .codelisten
        .benoetigt(VERZEICHNISDIENST, "Verzeichnisdienst")?;
    let verfahrensart = d
        .codelisten
        .verfahrensart(d.projekt.bundesland, &d.antrag.verfahrensart)?;
    let kanaele = inhalt::kanaele(d.codelisten);
    for a in d.anlagen {
        if !anlagen.enthaelt(&a.anlageart) {
            return Err(CoreError::InvalidInput(format!(
                "Anlage „{}“: Die Art {} steht nicht in der Codeliste {} (Version {}).",
                a.bezeichnung, a.anlageart, anlagen.name, anlagen.version
            )));
        }
        mime_pruefen(&a.dateiname, mimes)?;
    }

    let mut x = XmlSchreiber::new();
    wurzel_auf(&mut x, wurzel, false);
    kopf(&mut x, d, element, verzeichnis, false);

    if let Some(k) = bezug {
        x.auf("xbau:bezug", &[])
            .text("referenz", &d.antrag.vorgang.to_string());
        if let Some(az) = &k.aktenzeichen {
            x.text("vorgang", az);
        }
        if k.element == "prozessnachrichten.formellePruefungBefundliste.1140" {
            x.auf("bezugNachricht", &[])
                .text("bn:nachrichtenUUID", &k.id.to_string())
                .auf(
                    "bn:nachrichtentyp",
                    &[
                        (
                            "listURI",
                            "urn:xoev-de:xbau-kernmodul:codeliste:xbau-kernmodul-nachrichten",
                        ),
                        ("listVersionID", "3.0"),
                    ],
                )
                .text("code", "1140")
                .zu()
                .text(
                    "bn:erstellungszeitpunkt",
                    &k.erstellt.to_rfc3339_opts(SecondsFormat::Secs, true),
                )
                .zu();
        }
        x.zu();
    } else {
        let (referenz, antragsteller) = match art {
            Antragsart::Genehmigungsfreistellung { .. } => {
                ("xbau:referenzAnzeigeservice", "xbau:anzeigender")
            }
            _ => ("xbau:referenzAntragsservice", "xbau:antragsteller"),
        };
        x.auf(referenz, &[])
            .text("referenzAntragsservice", &d.antrag.vorgang.to_string())
            .zu();
        for b in d.bauherren {
            x.auf(antragsteller, &[]);
            akteur(&mut x, b, kanaele);
            x.zu();
        }
    }
    if !matches!(art, Antragsart::Genehmigungsfreistellung { .. }) {
        // Nachtrag nur zur Baugenehmigung; der Vorbescheid kennt keinen.
        let nachtrag = d
            .vorhaben
            .hauptgenehmigung
            .as_deref()
            .map(str::trim)
            .filter(|a| !a.is_empty() && matches!(art, Antragsart::Baugenehmigung));
        if verfahrensart.is_some() || nachtrag.is_some() {
            x.auf("xbau:prozessmerkmale", &[]);
            if let Some((liste, code)) = &verfahrensart {
                x.auf(
                    "xbau:verfahrensart",
                    &[
                        ("listURI", liste.uri.as_str()),
                        ("listVersionID", liste.version.as_str()),
                    ],
                )
                .text("code", code)
                .zu();
            }
            if let Some(az) = nachtrag {
                x.auf("xbau:nachtragsbaugenehmigung", &[])
                    .text("xbau:vorgang", az)
                    .zu();
            }
            x.zu();
        }
    }
    for frage in &fragen {
        x.auf("xbau:beantragterVorbescheid", &[])
            .auf("xbau:vorbescheidfrage", &[]);
        for absatz in frage {
            x.text("textabsatz", absatz);
        }
        x.zu().zu();
    }

    // Abweichungen gehören nur zum Bauantrag; im geänderten Antrag stehen sie
    // vor den Vorhabendaten, im Antrag danach.
    let mit_abweichungen = matches!(art, Antragsart::Baugenehmigung);
    if mit_abweichungen && bezug.is_some() {
        inhalt::abweichungen(&mut x, d.vorhaben);
    }

    bauvorhaben(
        &mut x,
        d,
        kanaele,
        if bezug.is_some() {
            "xbau:bauvorhabenAenderungsdaten"
        } else {
            "xbau:bauvorhaben"
        },
    )?;
    if mit_abweichungen && bezug.is_none() {
        inhalt::abweichungen(&mut x, d.vorhaben);
    }

    anlagen_schreiben(&mut x, d, anlagen, mimes, "xbau:anlagen");
    if let Antragsart::Genehmigungsfreistellung {
        als_bauantrag_weiterfuehren,
    } = art
    {
        x.text(
            "xbau:alsBauantragWeiterzufuehren",
            if als_bauantrag_weiterfuehren {
                "true"
            } else {
                "false"
            },
        );
    }

    x.zu();
    x.fertig()
}

/// Wurzelelement einer Hochbau- (`xbau:`) oder Kernmodul-Nachricht (`k:`).
pub(super) fn wurzel_auf(x: &mut XmlSchreiber, wurzel: &'static str, kern: bool) {
    if kern {
        x.auf(
            wurzel,
            &[
                ("xmlns:k", super::austausch::KERN),
                ("xmlns:bn", "http://xoev.de/schemata/basisnachricht/g2g/1_1"),
                ("produkt", "BauPlanIO"),
                ("produkthersteller", "PixelCraft"),
                ("produktversion", env!("CARGO_PKG_VERSION")),
                ("standard", "XBau-Kernmodul"),
                ("version", "1.4.0"),
            ],
        );
    } else {
        x.auf(
            wurzel,
            &[
                ("xmlns:xbau", NAMENSRAUM),
                ("xmlns:bn", "http://xoev.de/schemata/basisnachricht/g2g/1_1"),
                ("produkt", "BauPlanIO"),
                ("produkthersteller", "PixelCraft"),
                ("produktversion", env!("CARGO_PKG_VERSION")),
                ("standard", "XBau-Hochbau"),
                ("version", XBAU_VERSION),
            ],
        );
    }
}

/// Nachrichtenkopf vom Büro (Autor) an die Bauaufsicht (Leser).
pub(super) fn kopf(
    x: &mut XmlSchreiber,
    d: &Nachrichtendaten<'_>,
    element: &str,
    verzeichnis: &super::Codeliste,
    kern: bool,
) {
    let zeit = d.zeitpunkt.to_rfc3339_opts(SecondsFormat::Secs, true);
    let (liste, version) = if kern {
        (
            "urn:xoev-de:xbau-kernmodul:codeliste:xbau-kernmodul-nachrichten",
            "3.0",
        )
    } else {
        ("urn:xoev-de:xbau:codeliste:xbau-nachrichten", "5.0")
    };
    x.auf("nachrichtenkopf.g2g", &[])
        .auf("identifikation.nachricht", &[])
        .text("nachrichtenUUID", &d.nachricht_id.to_string())
        .auf(
            "nachrichtentyp",
            &[("listURI", liste), ("listVersionID", version)],
        )
        .text("code", element.rsplit('.').next().unwrap_or_default())
        .zu()
        .text("erstellungszeitpunkt", &zeit)
        .zu();
    let dvdv = [("listVersionID", verzeichnis.version.as_str())];
    x.auf("leser", &[])
        .auf("verzeichnisdienst", &dvdv)
        .text("code", "DVDV")
        .zu()
        .text("kennung", d.antrag.bauaufsicht.kennung.trim())
        .text("name", d.antrag.bauaufsicht.name.trim())
        .zu();
    x.auf("autor", &[])
        .auf("verzeichnisdienst", &dvdv)
        .text("code", "DVDV")
        .zu()
        .text("kennung", d.buero.kennung.trim())
        .text("name", d.buero.name.trim())
        .zu();
    x.zu();
}

/// `xbau:Bauvorhaben`: Bezeichnung, Beteiligte, Gegenstand und Verortung.
pub(super) fn bauvorhaben(
    x: &mut XmlSchreiber,
    d: &Nachrichtendaten<'_>,
    kanaele: Option<&super::Codeliste>,
    element: &'static str,
) -> CoreResult<()> {
    let bezeichnung = match d.antrag.bezeichnung.trim() {
        "" => d.projekt.name.trim(),
        b => b,
    };
    x.auf(element, &[])
        .text("xbau:bezeichnungDesBauvorhabens", bezeichnung);
    beteiligte(x, d, kanaele)?;

    x.auf("xbau:gegenstand", &[])
        .auf("xbau:anzahlDerGebaeude", &[])
        .text(
            "xbau:anzahlDerGebaeudeImBauvorhaben",
            &d.antrag.anzahl_gebaeude.to_string(),
        )
        .zu();
    for m in &d.antrag.massnahmen {
        x.auf(
            "xbau:artDerMassnahme",
            &[
                ("listURI", "urn:xoev-de:xbau:codeliste:baumassnahmeart"),
                ("listVersionID", "2.0"),
            ],
        )
        .text("code", m)
        .zu();
    }
    if let Some(g) = d.vorhaben.gebaeudeklasse {
        x.auf("xbau:bauordnungsrechtlicheKlassifikation", &[])
            .auf(
                "xbau:gebaeudeklasse",
                &[
                    ("listURI", "urn:xoev-de:xbau:codeliste:gebaeudeklasse"),
                    ("listVersionID", "3.0"),
                ],
            )
            .text("code", &g.to_string())
            .zu()
            .zu();
    }
    if let Some(f) = &d.vorhaben.flaechen {
        f.schreiben(x);
    }
    x.zu(); // gegenstand

    let g = &d.antrag.grundstueck;
    x.auf("xbau:verortung", &[])
        .auf("xbau:baugrundstueck", &[])
        .auf("xbau:grundstueck", &[]);
    if !g.anschrift.ist_leer() {
        x.auf("xbau:anschrift", &[]);
        anschrift_felder(x, &g.anschrift);
        x.zu();
    }
    let land = land_schluessel(d.projekt.bundesland);
    for f in &g.flurstuecke {
        x.auf("xbau:flurstueck", &[])
            .auf("bundesland", &[("listVersionID", BUNDESLAND_VERSION)])
            .text("code", land)
            .zu()
            .auf("kodierungFlurstueck", &[])
            .auf("gemarkung", &[]);
        let gemarkung = f.gemarkung.trim();
        if !f.gemarkungsnummer.trim().is_empty() {
            x.text("nummer", f.gemarkungsnummer.trim());
        } else if gemarkung.chars().all(|c| c.is_ascii_digit()) {
            x.text("nummer", gemarkung);
        } else {
            x.text("bezeichnung", gemarkung);
        }
        x.zu()
            .text_optional("flur", &f.flur)
            .text("zaehler", f.zaehler.trim())
            .text_optional("nenner", &f.nenner)
            .zu()
            .zu();
    }
    x.zu().zu().zu(); // grundstueck, baugrundstueck, verortung
    x.zu(); // bauvorhaben
    Ok(())
}

/// `xbau:BeteiligteBauprojekt` als Element `xbau:beteiligte`.
pub(super) fn beteiligte(
    x: &mut XmlSchreiber,
    d: &Nachrichtendaten<'_>,
    kanaele: Option<&super::Codeliste>,
) -> CoreResult<()> {
    x.auf("xbau:beteiligte", &[]);
    for b in d.bauherren {
        x.auf("xbau:bauherr", &[]).auf("xbau:bauherr", &[]);
        akteur(x, b, kanaele);
        x.zu().zu();
    }
    if let Some(b) = inhalt::bevollmaechtigter(d.vorhaben) {
        x.auf("xbau:bevollmaechtigter", &[]);
        akteur(x, &b.beteiligter, kanaele);
        x.zu();
    }
    // `fehlende_angaben` hat oben geprüft, dass es den Entwurfsverfasser gibt.
    let ev = d
        .antrag
        .entwurfsverfasser_von(d.buero)
        .ok_or_else(|| CoreError::InvalidInput("Entwurfsverfasser fehlt".into()))?;
    x.auf("xbau:entwurfsverfasser", &[])
        .auf("xbau:personendaten", &[]);
    akteur(
        x,
        &Beteiligter::Person {
            id: ev.id,
            vorname: ev.vorname.clone(),
            familienname: ev.familienname.clone(),
            // Geschäftsanschrift des Büros.
            anschrift: d.buero.anschrift.clone(),
            kommunikation: Kommunikation::default(),
        },
        None,
    );
    x.zu();
    x.auf("xbau:berechtigung", &[]);
    if let Some(land) = ev.kammerland
        && !ev.mitgliedsnummer.trim().is_empty()
    {
        x.auf("xbau:mitgliedschaftInland", &[])
            .auf("xbau:mitgliedsnummerBundesland", &[])
            .text("xbau:mitgliedsnummer", ev.mitgliedsnummer.trim())
            .auf("xbau:bundesland", &[("listVersionID", BUNDESLAND_VERSION)])
            .text("code", land_schluessel(land))
            .zu()
            .zu()
            .zu();
    }
    if !ev.nachweis.trim().is_empty() {
        x.auf("xbau:nachweisBerechtigung", &[])
            .auf("xbau:nachweisBerechtigung", &[]);
        for absatz in ev.nachweis.lines().map(str::trim).filter(|a| !a.is_empty()) {
            x.text("textabsatz", absatz);
        }
        x.zu().zu();
    }
    if !ev.berufsbezeichnung.trim().is_empty() {
        x.auf("xbau:berufsbezeichnung", &[])
            .text("xbau:berufsbezeichnung", ev.berufsbezeichnung.trim())
            .zu();
    }
    x.zu().zu(); // berechtigung, entwurfsverfasser
    inhalt::weitere_beteiligte(x, d.vorhaben, kanaele);
    x.auf("xbau:gebuehrenpflichtiger", &[]);
    akteur(
        x,
        d.antrag
            .gebuehrenpflichtiger
            .as_ref()
            .unwrap_or(&d.bauherren[0]),
        kanaele,
    );
    x.zu();
    x.zu(); // beteiligte
    Ok(())
}

/// Anlagen der Nachricht mit Art und MIME-Typ aus den Codelisten.
pub(super) fn anlagen_schreiben(
    x: &mut XmlSchreiber,
    d: &Nachrichtendaten<'_>,
    anlagen: &super::Codeliste,
    mimes: &super::Codeliste,
    element: &'static str,
) {
    x.auf(element, &[]);
    for a in d.anlagen {
        let mime = mime_typ(&a.dateiname).unwrap_or_default();
        x.auf("anlage", &[])
            .text("bezeichnung", a.bezeichnung.trim())
            .text("versionsnummer", &a.versionsnummer.to_string())
            .text("datum", &a.datum.to_string())
            .auf(
                "anlageart",
                // Im Kernmodul ist die Liste nicht fest vorgegeben.
                &if element == "anlagen" {
                    vec![
                        ("listURI", ANLAGEN),
                        ("listVersionID", anlagen.version.as_str()),
                    ]
                } else {
                    vec![("listVersionID", anlagen.version.as_str())]
                },
            )
            .text("code", &a.anlageart)
            .zu()
            .auf(
                "mimeType",
                &[
                    ("listURI", MIMETYPES),
                    ("listVersionID", mimes.version.as_str()),
                ],
            )
            .text("code", mime)
            .zu()
            .auf("anhangOderVerlinkung", &[])
            .auf("anhang", &[])
            .text("dokumentid", &format!("urn:uuid:{}", a.dokumentid))
            .text("dateiname", &a.dateiname)
            .zu()
            .zu()
            .zu();
    }
    x.zu(); // anlagen
}

pub(crate) fn mime_pruefen(dateiname: &str, mimes: &super::Codeliste) -> CoreResult<()> {
    match mime_typ(dateiname) {
        Some(m) if mimes.enthaelt(m) => Ok(()),
        Some(m) => Err(CoreError::InvalidInput(format!(
            "{dateiname}: Der Dateityp {m} steht nicht in der Codeliste {} (Version {}).",
            mimes.name, mimes.version
        ))),
        None => Err(CoreError::InvalidInput(format!(
            "{dateiname}: Dieser Dateityp kann nicht als XBau-Anlage übermittelt werden. Bitte als PDF ablegen."
        ))),
    }
}

/// `xbauk:Akteur`: natürliche Person oder Organisation.
pub(super) fn akteur(x: &mut XmlSchreiber, b: &Beteiligter, kanaele: Option<&super::Codeliste>) {
    match b {
        Beteiligter::Person {
            id,
            vorname,
            familienname,
            anschrift,
            kommunikation,
        } => {
            x.auf("natuerlichePerson", &[])
                .text("id", &id.to_string())
                .auf("name", &[])
                .auf("familienname", &[])
                .text("name", familienname.trim())
                .zu()
                .auf("vorname", &[])
                .text("name", vorname.trim())
                .zu()
                .zu()
                .auf("anschrift", &[]);
            anschrift_felder(x, anschrift);
            x.zu();
            inhalt::kommunikation(x, kommunikation, kanaele);
            x.zu();
        }
        Beteiligter::Organisation {
            id,
            name,
            anschrift,
            ansprechpartner,
            kommunikation,
        } => {
            x.auf("organisation", &[])
                .text("id", &id.to_string())
                .auf("name", &[])
                .text("name", name.trim())
                .zu();
            if !anschrift.ist_leer() {
                x.auf("anschrift", &[]);
                anschrift_felder(x, anschrift);
                x.zu();
            }
            inhalt::kommunikation(x, kommunikation, kanaele);
            // Der Ansprechpartner ist als Freitext erfasst: Er geht ungeteilt
            // als Familienname, damit kein Name falsch zerlegt wird.
            if !ansprechpartner.trim().is_empty() {
                x.auf("ansprechpartner", &[])
                    .auf("name", &[])
                    .auf("familienname", &[])
                    .text("name", ansprechpartner.trim())
                    .zu()
                    .auf("vorname", &[])
                    .zu()
                    .zu()
                    .zu();
            }
            x.zu();
        }
    }
}

/// Kinder einer Anschrift, in der Reihenfolge der Schemas.
fn anschrift_felder(x: &mut XmlSchreiber, a: &Anschrift) {
    x.text_optional("strasse", &a.strasse)
        .text_optional("hausnummer", &a.hausnummer)
        .text_optional("postleitzahl", &a.postleitzahl)
        .text_optional("ort", &a.ort);
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use chrono::TimeZone;

    use super::*;
    use crate::domain::{Bundesland, DocumentId, ProjectId};
    use crate::xbau::{
        Anlagenwahl, Bauaufsicht, Baugrundstueck, Codeeintrag, Codeliste, Flurstueck,
    };

    /// Codelisten für Tests. Die Codes der Anlagenarten sind Platzhalter,
    /// keine echten Werte aus dem XRepository.
    pub(crate) fn test_codelisten() -> Codelisten {
        let mut c = Codelisten::eingebaut();
        c.hinzufuegen(Codeliste {
            uri: ANLAGEN.into(),
            version: "test".into(),
            name: "XBau Anlage Art".into(),
            eintraege: vec![Codeeintrag {
                code: "0100".into(),
                beschreibung: "Testeintrag".into(),
            }],
        });
        c.hinzufuegen(Codeliste {
            uri: MIMETYPES.into(),
            version: "test".into(),
            name: "XBau MIME-Types".into(),
            eintraege: vec![Codeeintrag {
                code: "application/pdf".into(),
                beschreibung: "PDF".into(),
            }],
        });
        c
    }

    pub(crate) fn beispiel() -> (Project, Bauantrag, Buero) {
        let projekt = Project {
            id: ProjectId::new(),
            name: "MFH Lindenweg".into(),
            bundesland: Bundesland::By,
            grundstuecksflaeche_m2: Some(812.0),
            planungsrecht: Default::default(),
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let anschrift = Anschrift {
            strasse: "Lindenweg".into(),
            hausnummer: "4".into(),
            postleitzahl: "80331".into(),
            ort: "München".into(),
        };
        let antrag = Bauantrag {
            bauaufsicht: Bauaufsicht {
                name: "Lokalbaukommission München".into(),
                kennung: "bab:091620000".into(),
            },
            bauherren: vec![
                Beteiligter::Person {
                    id: Uuid::new_v4(),
                    vorname: "Anna".into(),
                    familienname: "Müller-Øresund".into(),
                    anschrift: anschrift.clone(),
                    kommunikation: Kommunikation::default(),
                },
                Beteiligter::Organisation {
                    id: Uuid::new_v4(),
                    name: "Wohnbau Süd GmbH & Co. KG".into(),
                    anschrift: Anschrift {
                        strasse: "Leopoldstraße".into(),
                        hausnummer: "1".into(),
                        postleitzahl: "80802".into(),
                        ort: "München".into(),
                    },
                    ansprechpartner: "Dr. Jens Weber".into(),
                    kommunikation: Kommunikation {
                        telefon: "+49 89 123456".into(),
                        email: "bau@wohnbau-sued.example".into(),
                    },
                },
            ],
            grundstueck: Baugrundstueck {
                anschrift: Anschrift {
                    hausnummer: "12".into(),
                    ..anschrift
                },
                flurstuecke: vec![
                    Flurstueck {
                        gemarkung: "Schwabing".into(),
                        flur: "3".into(),
                        zaehler: "123".into(),
                        nenner: "4".into(),
                        ..Default::default()
                    },
                    Flurstueck {
                        gemarkung: "Freimann".into(),
                        gemarkungsnummer: "0921".into(),
                        zaehler: "7".into(),
                        ..Default::default()
                    },
                    Flurstueck {
                        gemarkung: "0915".into(),
                        zaehler: "124".into(),
                        ..Default::default()
                    },
                ],
            },
            anlagen: vec![Anlagenwahl {
                dokument: DocumentId::new(),
                anlageart: "0100".into(),
                bezeichnung: "Lageplan".into(),
            }],
            ..Default::default()
        };
        let buero = Buero {
            name: "Architekturbüro Muster".into(),
            anschrift: Anschrift {
                strasse: "Türkenstraße".into(),
                hausnummer: "20".into(),
                postleitzahl: "80333".into(),
                ort: "München".into(),
            },
            kennung: "arc:4711".into(),
            entwurfsverfasser: vec![crate::xbau::Entwurfsverfasser {
                id: Uuid::new_v4(),
                vorname: "Eva".into(),
                familienname: "Entwurf".into(),
                berufsbezeichnung: "Architektin".into(),
                mitgliedsnummer: "12345".into(),
                kammerland: Some(Bundesland::By),
                nachweis: String::new(),
            }],
        };
        (projekt, antrag, buero)
    }

    pub(crate) fn anlagen() -> Vec<Anlagendatei> {
        vec![Anlagendatei {
            dokumentid: Uuid::new_v4(),
            bezeichnung: "Lageplan § 7 BauVorlV".into(),
            versionsnummer: 2,
            datum: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            anlageart: "0100".into(),
            dateiname: "lageplan.pdf".into(),
        }]
    }

    fn schema_ordner() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("xbau/2.7")
    }

    /// Prüft XML mit `xmllint` gegen die XBau-Schemas. `None`, wenn
    /// `xmllint` fehlt; das ist lokal erlaubt, in CI (`CI` gesetzt) ein Fehler.
    pub(crate) fn schemafehler(xml: &str) -> Option<Result<(), String>> {
        let dir = tempfile_dir();
        let datei = dir.join("nachricht.xml");
        std::fs::write(&datei, xml).unwrap();
        let ergebnis = Command::new("xmllint")
            .args(["--noout", "--nonet", "--schema", "austausch.xsd"])
            .arg(&datei)
            .current_dir(schema_ordner())
            .output();
        std::fs::remove_dir_all(dir).ok();
        match ergebnis {
            Ok(o) if o.status.success() => Some(Ok(())),
            Ok(o) => Some(Err(String::from_utf8_lossy(&o.stderr).into_owned())),
            Err(e) if std::env::var_os("CI").is_none() => {
                eprintln!("xmllint nicht verfügbar ({e}), Schemaprüfung übersprungen");
                None
            }
            Err(e) => panic!("xmllint wird in CI benötigt: {e}"),
        }
    }

    pub(crate) fn gegen_schema_pruefen(xml: &str) {
        if let Some(Err(e)) = schemafehler(xml) {
            panic!("Nachricht ist nicht schemakonform:\n{e}\n{xml}");
        }
    }

    #[test]
    fn schemapruefung_erkennt_fehler() {
        let (projekt, antrag, buero) = beispiel();
        let anlagen = anlagen();
        let c = test_codelisten();
        let xml = baugenehmigung_antrag(&daten(&projekt, &antrag, &buero, &anlagen, &c)).unwrap();
        // Pflichtelement entfernen: Die Prüfung muss anschlagen.
        let kaputt = xml.replace("xbau:anlagen>", "xbau:anlagenX>");
        if let Some(r) = schemafehler(&kaputt) {
            assert!(r.is_err());
        }
    }

    fn tempfile_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("xbau-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    static OHNE_VORHABENANGABEN: Vorhabenangaben = Vorhabenangaben {
        beteiligte: Vec::new(),
        gebaeudeklasse: None,
        hauptgenehmigung: None,
        flaechen: None,
        abweichungen: Vec::new(),
    };

    pub(crate) fn daten<'a>(
        projekt: &'a Project,
        antrag: &'a Bauantrag,
        buero: &'a Buero,
        anlagen: &'a [Anlagendatei],
        codelisten: &'a Codelisten,
    ) -> Nachrichtendaten<'a> {
        Nachrichtendaten {
            projekt,
            antrag,
            // Die Tests führen die Bauherren der Einfachheit halber im Antrag.
            bauherren: &antrag.bauherren,
            buero,
            vorhaben: &OHNE_VORHABENANGABEN,
            anlagen,
            codelisten,
            nachricht_id: Uuid::new_v4(),
            zeitpunkt: Utc.with_ymd_and_hms(2026, 10, 13, 14, 0, 0).unwrap(),
        }
    }

    #[test]
    fn bauantrag_ist_schemakonform() {
        let (projekt, antrag, buero) = beispiel();
        let anlagen = anlagen();
        let c = test_codelisten();
        let xml = baugenehmigung_antrag(&daten(&projekt, &antrag, &buero, &anlagen, &c)).unwrap();
        assert!(xml.contains("<code>09</code>"), "Bayern");
        assert!(xml.contains("Wohnbau Süd GmbH &amp; Co. KG"));
        assert!(xml.contains("<bezeichnung>Schwabing</bezeichnung>"));
        assert!(xml.contains("<nummer>0915</nummer>"));
        // Die Gemarkungsnummer geht dem Namen vor.
        assert!(xml.contains("<nummer>0921</nummer>"));
        assert!(!xml.contains("<bezeichnung>Freimann</bezeichnung>"));
        assert!(xml.contains("MFH Lindenweg"), "Projektname als Bezeichnung");
        let kompakt: String = xml.lines().map(str::trim).collect();
        let ev = &kompakt[kompakt.find("<xbau:entwurfsverfasser>").unwrap()..];
        assert!(ev.starts_with("<xbau:entwurfsverfasser><xbau:personendaten><natuerlichePerson>"));
        assert!(ev.contains("<name>Entwurf</name>"));
        assert!(ev.contains("<xbau:mitgliedsnummer>12345</xbau:mitgliedsnummer>"));
        assert!(ev.contains("<xbau:berufsbezeichnung>Architektin</xbau:berufsbezeichnung>"));
        assert!(!ev[..ev.find("</xbau:entwurfsverfasser>").unwrap()].contains("Architekturbüro"));
        gegen_schema_pruefen(&xml);
    }

    #[test]
    fn entwurfsverfasser_mit_nachweis_ohne_kammer_ist_schemakonform() {
        let (projekt, antrag, mut buero) = beispiel();
        let e = &mut buero.entwurfsverfasser[0];
        e.mitgliedsnummer.clear();
        e.kammerland = None;
        e.berufsbezeichnung.clear();
        e.nachweis = "Bescheinigung nach Art. 61 Abs. 3 BayBO\n\nausgestellt 2019".into();
        buero.anschrift.ort = "München".into();
        let anlagen = anlagen();
        let c = test_codelisten();
        let xml = baugenehmigung_antrag(&daten(&projekt, &antrag, &buero, &anlagen, &c)).unwrap();
        assert!(!xml.contains("mitgliedschaftInland"));
        assert!(xml.contains("<textabsatz>Bescheinigung nach Art. 61 Abs. 3 BayBO</textabsatz>"));
        assert!(xml.contains("<textabsatz>ausgestellt 2019</textabsatz>"));
        gegen_schema_pruefen(&xml);
    }

    #[test]
    fn entwurfsverfasser_muss_bauvorlageberechtigt_und_eindeutig_sein() {
        let (_, mut antrag, mut buero) = beispiel();
        let bauherren = antrag.bauherren.clone();
        let fehlt = |a: &Bauantrag, b: &Buero| a.fehlende_angaben(b, &bauherren);
        assert!(
            !fehlt(&antrag, &buero)
                .iter()
                .any(|f| f.contains("Entwurfsverfasser"))
        );

        buero.entwurfsverfasser[0].mitgliedsnummer.clear();
        assert!(
            fehlt(&antrag, &buero)
                .iter()
                .any(|f| f.contains("Nachweis der Bauvorlageberechtigung"))
        );
        buero.entwurfsverfasser[0].mitgliedsnummer = "12345".into();
        buero.entwurfsverfasser[0].kammerland = None;
        assert!(fehlt(&antrag, &buero).contains(&"Entwurfsverfasser: Land der Kammer".to_string()));

        // Zwei Personen: ohne Wahl ist offen, wer zeichnet.
        let mut zweite = buero.entwurfsverfasser[0].clone();
        zweite.id = Uuid::new_v4();
        zweite.kammerland = Some(Bundesland::By);
        buero.entwurfsverfasser.push(zweite.clone());
        assert!(
            fehlt(&antrag, &buero)
                .iter()
                .any(|f| f.contains("bauvorlageberechtigte Person"))
        );
        antrag.entwurfsverfasser = Some(zweite.id);
        assert_eq!(antrag.entwurfsverfasser_von(&buero), Some(&zweite));
        assert!(
            !fehlt(&antrag, &buero)
                .iter()
                .any(|f| f.contains("Entwurfsverfasser"))
        );

        buero.entwurfsverfasser.pop();
        assert!(
            fehlt(&antrag, &buero)
                .iter()
                .any(|f| f.contains("nicht mehr in den Büroangaben"))
        );
        buero.entwurfsverfasser.clear();
        antrag.entwurfsverfasser = None;
        assert!(
            fehlt(&antrag, &buero)
                .iter()
                .any(|f| f.contains("bauvorlageberechtigte Person"))
        );
    }

    #[test]
    fn minimaler_antrag_ist_schemakonform() {
        let (projekt, mut antrag, buero) = beispiel();
        antrag.bauherren.truncate(1);
        antrag.grundstueck.anschrift = Anschrift::default();
        antrag.grundstueck.flurstuecke.truncate(1);
        antrag.gebuehrenpflichtiger = Some(Beteiligter::Organisation {
            id: Uuid::new_v4(),
            name: "Kämmerei".into(),
            anschrift: Anschrift {
                strasse: "Marienplatz".into(),
                hausnummer: "8".into(),
                postleitzahl: "80331".into(),
                ort: "München".into(),
            },
            ansprechpartner: String::new(),
            kommunikation: Kommunikation::default(),
        });
        antrag.massnahmen = vec!["12".into(), "14".into()];
        let anlagen = anlagen();
        let c = test_codelisten();
        let xml = baugenehmigung_antrag(&daten(&projekt, &antrag, &buero, &anlagen, &c)).unwrap();
        gegen_schema_pruefen(&xml);
    }

    #[test]
    fn fehlende_angaben_werden_genannt() {
        let (projekt, mut antrag, mut buero) = beispiel();
        buero.kennung.clear();
        antrag.bauherren.clear();
        antrag.grundstueck.flurstuecke[0].zaehler.clear();
        let anlagen = anlagen();
        let c = test_codelisten();
        let err = baugenehmigung_antrag(&daten(&projekt, &antrag, &buero, &anlagen, &c))
            .unwrap_err()
            .to_string();
        for teil in [
            "Absenderkennung",
            "Bauherr",
            "1. Flurstück: Flurstücksnummer",
        ] {
            assert!(err.contains(teil), "{teil} fehlt in: {err}");
        }
    }

    #[test]
    fn codes_werden_gegen_codelisten_geprueft() {
        let (projekt, antrag, buero) = beispiel();
        let c = test_codelisten();
        let mut anlagen = anlagen();
        anlagen[0].anlageart = "9999".into();
        let err = baugenehmigung_antrag(&daten(&projekt, &antrag, &buero, &anlagen, &c))
            .unwrap_err()
            .to_string();
        assert!(err.contains("9999"), "{err}");

        let mut anlagen = self::anlagen();
        anlagen[0].dateiname = "foto.png".into();
        let err = baugenehmigung_antrag(&daten(&projekt, &antrag, &buero, &anlagen, &c))
            .unwrap_err()
            .to_string();
        assert!(err.contains("image/png"), "{err}");

        let ohne = Codelisten::eingebaut();
        let err = baugenehmigung_antrag(&daten(&projekt, &antrag, &buero, &anlagen, &ohne))
            .unwrap_err()
            .to_string();
        assert!(err.contains("urn:xoev-de:xbau:codeliste:anlagen"), "{err}");
    }

    fn bezugskopf(element: &str, ba: &Bauantrag) -> super::super::austausch::Nachrichtenkopf {
        super::super::austausch::Nachrichtenkopf {
            id: Uuid::new_v4(),
            element: element.into(),
            namensraum: super::super::austausch::HOCHBAU.into(),
            titel: String::new(),
            erstellt: Utc.with_ymd_and_hms(2026, 10, 1, 9, 0, 0).unwrap(),
            absender: "buero:test".into(),
            empfaenger: ba.bauaufsicht.kennung.trim().into(),
            antragsreferenz: Some(ba.vorgang),
            aktenzeichen: Some("2026/42".into()),
            bezugsnachricht: None,
        }
    }

    #[test]
    fn vorbescheid_ist_schemakonform() {
        let (projekt, ba, buero) = beispiel();
        let anlagen = anlagen();
        let c = test_codelisten();
        let fragen = vec![
            "Ist ein drittes Vollgeschoss zulässig?\n\nBezug: Nachbarbebauung Lindenweg 3".into(),
            "  ".into(),
            "Ist die Garage an der Grenze zulässig?".into(),
        ];
        let art = Antragsart::Vorbescheid { fragen: &fragen };
        let d = daten(&projekt, &ba, &buero, &anlagen, &c);
        let xml = antrag(&d, art).unwrap();
        assert!(xml.contains("<xbau:vorbescheid.antrag.0210 "));
        assert!(xml.contains("<code>0210</code>"));
        assert_eq!(xml.matches("<xbau:beantragterVorbescheid>").count(), 2);
        assert!(xml.contains("<textabsatz>Bezug: Nachbarbebauung Lindenweg 3</textabsatz>"));
        gegen_schema_pruefen(&xml);
        let kopf = super::super::austausch::vorpruefen(xml.as_bytes()).unwrap();
        assert_eq!(kopf.element, "vorbescheid.antrag.0210");
        assert_eq!(kopf.antragsreferenz, Some(ba.vorgang));

        let geaendert =
            antrag_geaendert(&d, art, &bezugskopf("vorbescheid.antrag.0210", &ba)).unwrap();
        assert!(geaendert.contains("<code>0212</code>"));
        assert!(geaendert.contains("<xbau:beantragterVorbescheid>"));
        gegen_schema_pruefen(&geaendert);
        // Bezug auf einen Bauantrag passt nicht zum Vorbescheid.
        assert!(antrag_geaendert(&d, art, &bezugskopf("baugenehmigung.antrag.0200", &ba)).is_err());

        let leer = vec![" \n ".to_string()];
        let err = antrag(&d, Antragsart::Vorbescheid { fragen: &leer })
            .unwrap_err()
            .to_string();
        assert!(err.contains("Frage zum Vorbescheid"), "{err}");
    }

    #[test]
    fn genehmigungsfreistellung_ist_schemakonform() {
        let (projekt, ba, buero) = beispiel();
        let anlagen = anlagen();
        let c = test_codelisten();
        let d = daten(&projekt, &ba, &buero, &anlagen, &c);
        let art = Antragsart::Genehmigungsfreistellung {
            als_bauantrag_weiterfuehren: true,
        };
        let xml = antrag(&d, art).unwrap();
        assert!(xml.contains("<xbau:genehmigungsfreistellung.anzeige.0600 "));
        assert!(xml.contains("<xbau:anzeigender>"));
        assert!(!xml.contains("antragsteller"));
        assert!(
            xml.contains(
                "<xbau:alsBauantragWeiterzufuehren>true</xbau:alsBauantragWeiterzufuehren>"
            )
        );
        gegen_schema_pruefen(&xml);
        let kopf = super::super::austausch::vorpruefen(xml.as_bytes()).unwrap();
        assert_eq!(kopf.antragsreferenz, Some(ba.vorgang));
        assert_eq!(
            art.dateiname(false).unwrap(),
            "genehmigungsfreistellung.anzeige.0600.xml"
        );
        assert!(art.dateiname(true).is_err());
        assert!(
            antrag_geaendert(
                &d,
                art,
                &bezugskopf("genehmigungsfreistellung.anzeige.0600", &ba)
            )
            .is_err()
        );
    }

    fn liste(uri: &str, version: &str, eintraege: &[(&str, &str)]) -> Codeliste {
        Codeliste {
            uri: uri.into(),
            version: version.into(),
            name: uri.rsplit(':').next().unwrap_or_default().into(),
            eintraege: eintraege
                .iter()
                .map(|(c, b)| Codeeintrag {
                    code: (*c).into(),
                    beschreibung: (*b).into(),
                })
                .collect(),
        }
    }

    fn person(vorname: &str, familienname: &str) -> Beteiligter {
        Beteiligter::Person {
            id: Uuid::new_v4(),
            vorname: vorname.into(),
            familienname: familienname.into(),
            anschrift: Anschrift::default(),
            kommunikation: Kommunikation {
                telefon: String::new(),
                email: format!("{}@example.de", familienname.to_lowercase()),
            },
        }
    }

    fn vorhabenangaben() -> Vorhabenangaben {
        use crate::xbau::{FlaechenBauNvo, WeitereRolle, WeitererBeteiligter};
        let rollen = [
            (WeitereRolle::Bevollmaechtigter, person("Paul", "Vollmacht")),
            (
                WeitereRolle::Bevollmaechtigter,
                person("Petra", "Zweitvollmacht"),
            ),
            (
                WeitereRolle::Grundstueckseigentuemer,
                person("Erna", "Eigen"),
            ),
            (WeitereRolle::Erbbauberechtigter, person("Ernst", "Erbbau")),
            (WeitereRolle::Fachplaner, person("Frieda", "Statik")),
        ];
        Vorhabenangaben {
            beteiligte: rollen
                .into_iter()
                .map(|(rolle, beteiligter)| WeitererBeteiligter {
                    rolle,
                    beteiligter,
                    fachrichtung: String::new(),
                })
                .collect(),
            gebaeudeklasse: Some(4),
            hauptgenehmigung: Some("BG-2025-0815".into()),
            flaechen: Some(FlaechenBauNvo {
                anzahl_vollgeschosse: Some(3),
                zulaessige_vollgeschosse: Some(3),
                flaeche_baugrundstueck_m2: Some(812.0),
                grundflaeche_m2: Some(243.6),
                grz_soll: Some(0.4),
                grz_geplant: Some(0.3),
                geschossflaeche_m2: Some(730.8),
                gfz_soll: Some(1.2),
                gfz_geplant: Some(0.9),
                ..Default::default()
            }),
            abweichungen: vec![
                crate::xbau::BeantragteAbweichung {
                    id: Uuid::new_v4(),
                    erleichterung: false,
                    liste: crate::verfahren::abweichungen::ABWEICHUNGART.into(),
                    version: "2.0".into(),
                    code: "2".into(),
                    vorschrift: "§ 6 Abs. 5 BauO NRW".into(),
                    beschreibung: "Nordwand Garage\nAbstandsfläche 0,4 m über der Grenze".into(),
                    begruendung: "Schutzziel Belichtung gewahrt".into(),
                },
                crate::xbau::BeantragteAbweichung {
                    id: Uuid::new_v4(),
                    erleichterung: true,
                    liste: crate::verfahren::abweichungen::ERLEICHTERUNG_GEGENSTAND.into(),
                    version: "1.0".into(),
                    code: "01".into(),
                    vorschrift: "Rettungswege".into(),
                    beschreibung: String::new(),
                    begruendung: String::new(),
                },
            ],
        }
    }

    #[test]
    fn vollstaendiger_inhalt_ist_schemakonform() {
        let (projekt, mut ba, buero) = beispiel();
        ba.verfahrensart = "2".into();
        let anlagen = anlagen();
        let mut c = test_codelisten();
        c.hinzufuegen(liste(
            "urn:xoev-de:xbau:by:codeliste:verfahrensart",
            "1.0",
            &[
                ("1", "Baugenehmigungsverfahren"),
                ("2", "Vereinfachtes Verfahren"),
            ],
        ));
        c.hinzufuegen(liste(
            crate::xbau::codeliste::ERREICHBARKEIT,
            "3",
            &[
                ("01", "E-Mail"),
                ("02", "Telefon (Festnetz)"),
                ("03", "Mobiltelefon"),
            ],
        ));
        let v = vorhabenangaben();
        let d = Nachrichtendaten {
            vorhaben: &v,
            ..daten(&projekt, &ba, &buero, &anlagen, &c)
        };
        let xml = baugenehmigung_antrag(&d).unwrap();
        let k: String = xml.lines().map(str::trim).collect();
        assert!(k.contains("<xbau:prozessmerkmale><xbau:verfahrensart listURI=\"urn:xoev-de:xbau:by:codeliste:verfahrensart\" listVersionID=\"1.0\"><code>2</code></xbau:verfahrensart><xbau:nachtragsbaugenehmigung><xbau:vorgang>BG-2025-0815</xbau:vorgang>"), "{xml}");
        // Nur ein Bevollmächtigter als eigenes Element, der zweite mit Rolle 01.
        assert_eq!(xml.matches("<xbau:bevollmaechtigter>").count(), 1);
        assert!(k.contains("<xbau:bevollmaechtigter><natuerlichePerson>"));
        assert_eq!(xml.matches("<xbau:weitereBeteiligte>").count(), 4);
        for code in ["01", "02", "06"] {
            assert!(
                k.contains(&format!("listVersionID=\"2.0\"><code>{code}</code>")),
                "{code}"
            );
        }
        assert!(
            k.contains("<xbau:nichtGelisteterWert>Erbbauberechtigter</xbau:nichtGelisteterWert>")
        );
        assert!(k.contains("<xbau:gebaeudeklasse listURI=\"urn:xoev-de:xbau:codeliste:gebaeudeklasse\" listVersionID=\"3.0\"><code>4</code>"));
        assert!(k.contains("<xbau:grundflaechen><xbau:grundflaeche>243.60</xbau:grundflaeche><xbau:grundflaechenzahlSoll>0.40</xbau:grundflaechenzahlSoll><xbau:grundflaechenzahlGeplant>0.30</xbau:grundflaechenzahlGeplant></xbau:grundflaechen>"));
        assert!(!xml.contains("xbau:baumassen"), "ohne Werte keine Gruppe");
        // Abweichung und Erleichterung nach dem Bauvorhaben, vor den Anlagen.
        let abw = k.find("<xbau:beantragteAbweichung>").expect("Abweichung");
        assert!(k.find("</xbau:bauvorhaben>").unwrap() < abw);
        assert!(abw < k.find("<xbau:angefragteErleichterung>").unwrap());
        assert!(k.contains("<xbau:vorschrift><textabsatz>§ 6 Abs. 5 BauO NRW</textabsatz></xbau:vorschrift><xbau:beschreibung><textabsatz>Nordwand Garage</textabsatz><textabsatz>Abstandsfläche 0,4 m über der Grenze</textabsatz></xbau:beschreibung>"), "{xml}");
        // Organisation mit Erreichbarkeit und Ansprechpartner.
        assert!(k.contains("<kommunikation><kanal listVersionID=\"3\"><code>02</code></kanal><kennung>+49 89 123456</kennung></kommunikation>"));
        assert!(k.contains("<ansprechpartner><name><familienname><name>Dr. Jens Weber</name></familienname><vorname></vorname></name></ansprechpartner>"));
        gegen_schema_pruefen(&xml);
        assert!(k.contains(
            "<xbau:fachrichtung><xbau:nichtGelisteterWert>nicht angegeben</xbau:nichtGelisteterWert>"
        ));

        // Erfasste Fachrichtung: als Code der Schemaliste, sonst als Freitext.
        for (text, erwartet) in [
            (
                "tragwerksplanung",
                "<code>1</code></xbau:code></xbau:fachrichtung>",
            ),
            (
                "Wärmeschutz",
                "<code>6</code></xbau:code></xbau:fachrichtung>",
            ),
            (
                "Vermessung",
                "<xbau:fachrichtung><xbau:nichtGelisteterWert>Vermessung</xbau:nichtGelisteterWert>",
            ),
        ] {
            let mut v2 = vorhabenangaben();
            v2.beteiligte.last_mut().unwrap().fachrichtung = text.into();
            let d = Nachrichtendaten {
                vorhaben: &v2,
                ..daten(&projekt, &ba, &buero, &anlagen, &c)
            };
            let xml = baugenehmigung_antrag(&d).unwrap();
            let k: String = xml.lines().map(str::trim).collect();
            assert!(k.contains(erwartet), "{text}: {xml}");
            gegen_schema_pruefen(&xml);
        }

        // Ohne Codeliste Erreichbarkeit: Kanal als Zusatz.
        let ohne = test_codelisten();
        let ba2 = Bauantrag {
            verfahrensart: String::new(),
            ..ba.clone()
        };
        let d = Nachrichtendaten {
            vorhaben: &v,
            ..daten(&projekt, &ba2, &buero, &anlagen, &ohne)
        };
        let xml = baugenehmigung_antrag(&d).unwrap();
        let k: String = xml.lines().map(str::trim).collect();
        assert!(k.contains("<kennung>+49 89 123456</kennung><zusatz>Telefon</zusatz>"));
        assert!(!xml.contains("xbau:verfahrensart"));
        gegen_schema_pruefen(&xml);
        // Nachreichung und Vorbescheid tragen die Prozessmerkmale ebenfalls,
        // der Vorbescheid ohne Nachtragsbezug.
        let geaendert =
            baugenehmigung_aenderung(&d, &bezugskopf("baugenehmigung.antrag.0200", &ba2)).unwrap();
        assert!(geaendert.contains("<xbau:nachtragsbaugenehmigung>"));
        gegen_schema_pruefen(&geaendert);
        let fragen = vec!["Ist ein drittes Vollgeschoss zulässig?".to_string()];
        let vb = antrag(&d, Antragsart::Vorbescheid { fragen: &fragen }).unwrap();
        assert!(!vb.contains("nachtragsbaugenehmigung"));
        gegen_schema_pruefen(&vb);
        let gf = antrag(
            &d,
            Antragsart::Genehmigungsfreistellung {
                als_bauantrag_weiterfuehren: false,
            },
        )
        .unwrap();
        assert!(!gf.contains("prozessmerkmale"));
        gegen_schema_pruefen(&gf);
    }

    #[test]
    fn verfahrensart_wird_gegen_die_landesliste_geprueft() {
        let (projekt, mut ba, buero) = beispiel();
        let anlagen = anlagen();
        let mut c = test_codelisten();
        ba.verfahrensart = "2".into();
        let err = baugenehmigung_antrag(&daten(&projekt, &ba, &buero, &anlagen, &c))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("urn:xoev-de:xbau:by:codeliste:verfahrensart"),
            "{err}"
        );
        c.hinzufuegen(liste(
            "urn:xoev-de:xbau:by:codeliste:verfahrensart",
            "1.0",
            &[("1", "Baugenehmigungsverfahren")],
        ));
        let err = baugenehmigung_antrag(&daten(&projekt, &ba, &buero, &anlagen, &c))
            .unwrap_err()
            .to_string();
        assert!(err.contains("Code 2"), "{err}");
    }

    #[test]
    fn anschriften_zeichen_und_anlagenart_werden_vor_dem_export_geprueft() {
        let (projekt, mut ba, mut buero) = beispiel();
        if let Beteiligter::Person { anschrift, .. } = &mut ba.bauherren[0] {
            anschrift.ort.clear();
            anschrift.strasse = "Улица".into();
        }
        buero.anschrift.postleitzahl.clear();
        let mut anlagen = anlagen();
        anlagen[0].anlageart.clear();
        anlagen[0].bezeichnung = "Lageplan.pdf".into();
        let mut v = vorhabenangaben();
        if let Beteiligter::Person { familienname, .. } = &mut v.beteiligte[2].beteiligter {
            *familienname = "Σωκράτης".into();
        }
        let c = test_codelisten();
        let d = Nachrichtendaten {
            vorhaben: &v,
            ..daten(&projekt, &ba, &buero, &anlagen, &c)
        };
        let err = baugenehmigung_antrag(&d).unwrap_err().to_string();
        for teil in [
            "1. Bauherr: Anschrift (Ort)",
            "1. Bauherr: Straße: enthält Zeichen",
            "„У“ (U+0423)",
            "Büro: Anschrift (Postleitzahl)",
            "Grundstückseigentümer Erna",
            "Art der Anlage für „Lageplan.pdf“",
        ] {
            assert!(err.contains(teil), "{teil} fehlt in: {err}");
        }
        assert!(!err.contains("Die Art  steht"), "{err}");
    }

    #[test]
    fn flaechen_nur_aus_festsetzungen_als_soll() {
        use crate::festsetzungen::{Nachweis, NutzungsPruefung, Quelle};
        let n = |art: &'static str, ist, soll, quelle| Nachweis {
            gegenstand: None,
            art,
            ist: Some(ist),
            soll: Some(soll),
            quelle,
            zwingend: false,
            eingehalten: None,
            abweichung: None,
            ueberschreitung: None,
            ueberschreitung_prozent: None,
            flaechenueberschreitung_m2: None,
        };
        let pruefung = |quelle| NutzungsPruefung {
            aufstellung: None,
            projekt_version: 1,
            kennzahlen: crate::calc::Kennzahlen {
                grz: 0.3,
                grz_gesamt: 0.45,
                gfz: 0.9,
                bmz: None,
            },
            nachweise: vec![
                n("grz", 0.3, 0.4, quelle),
                n("gfz", 0.9, 1.2, quelle),
                n("vollgeschosse", 3.0, 3.0, quelle),
                n("vollgeschosse", 2.0, 3.0, quelle),
            ],
            bebauungsplan: quelle == Quelle::Festsetzung,
            hinweise: Vec::new(),
        };
        let f = crate::xbau::FlaechenBauNvo::aus_pruefung(&pruefung(Quelle::Festsetzung), 812.0)
            .unwrap();
        assert_eq!((f.grz_soll, f.grz_geplant), (Some(0.4), Some(0.3)));
        // Je Gebäude ein Nachweis: größte Zahl und gemeinsame Festsetzung.
        assert_eq!(
            (f.anzahl_vollgeschosse, f.zulaessige_vollgeschosse),
            (Some(3), Some(3))
        );
        assert_eq!(
            crate::xbau::FlaechenBauNvo::aus_pruefung_mit_hinweis(
                &pruefung(Quelle::Festsetzung),
                812.0
            )
            .1,
            None
        );
        // Wohnhaus (1) + Garage (0) bei festgesetzten II Vollgeschossen.
        let mut p = pruefung(Quelle::Festsetzung);
        p.nachweise.retain(|n| n.art != "vollgeschosse");
        p.nachweise
            .push(n("vollgeschosse", 1.0, 2.0, Quelle::Festsetzung));
        p.nachweise
            .push(n("vollgeschosse", 0.0, 2.0, Quelle::Festsetzung));
        let (f, h) = crate::xbau::FlaechenBauNvo::aus_pruefung_mit_hinweis(&p, 812.0);
        let f = f.unwrap();
        assert_eq!(
            (f.anzahl_vollgeschosse, f.zulaessige_vollgeschosse, h),
            (Some(1), Some(2), None)
        );
        // Mehrdeutig: kein stilles Weglassen, sondern Hinweis.
        p.nachweise.last_mut().unwrap().ist = None;
        p.nachweise[2].soll = Some(3.0);
        let (f, h) = crate::xbau::FlaechenBauNvo::aus_pruefung_mit_hinweis(&p, 812.0);
        let f = f.unwrap();
        assert_eq!(
            (f.anzahl_vollgeschosse, f.zulaessige_vollgeschosse),
            (None, None)
        );
        let h = h.unwrap();
        assert!(
            h.contains("anzahlVollgeschosse") && h.contains("zulaessigeAnzahlVollgeschosse"),
            "{h}"
        );
        let f =
            crate::xbau::FlaechenBauNvo::aus_pruefung(&pruefung(Quelle::Orientierungswert), 812.0)
                .unwrap();
        assert_eq!(
            (f.grz_soll, f.gfz_soll),
            (None, None),
            "Orientierungswert ist kein Soll"
        );
        assert_eq!(f.gfz_geplant, Some(0.9));
    }
}
