//! Nachricht `baugenehmigung.antrag.0200` (Beantragung einer Baugenehmigung).
//!
//! Die XBau-Schemas mischen qualifizierte Elemente (Präfix `xbau:`) und
//! unqualifizierte (ohne Namensraum, u. a. alles aus dem Kernmodul). Welches
//! Element wie zu schreiben ist, steht in den Schemas; die Tests prüfen die
//! Ausgabe mit `xmllint` gegen sie.

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use uuid::Uuid;

use super::codeliste::{ANLAGEN, Codelisten, MIMETYPES, VERZEICHNISDIENST};
use super::xml::XmlSchreiber;
use super::{Anschrift, Bauantrag, Beteiligter, Buero, XBAU_VERSION, land_schluessel, mime_typ};
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
    pub anlagen: &'a [Anlagendatei],
    pub codelisten: &'a Codelisten,
    pub nachricht_id: Uuid,
    pub zeitpunkt: DateTime<Utc>,
}

/// Erzeugt die Antragsnachricht. Prüft vorher die Pflichtangaben und die
/// Codes gegen die Codelisten.
pub fn baugenehmigung_antrag(d: &Nachrichtendaten<'_>) -> CoreResult<String> {
    schreiben(d, None)
}

/// Geänderte Bauvorhabendaten und ausgewählte Anlagen als vollständiger Datensatz.
pub fn baugenehmigung_aenderung(
    d: &Nachrichtendaten<'_>,
    bezug: &super::austausch::Nachrichtenkopf,
) -> CoreResult<String> {
    let erwartet = match bezug.element.as_str() {
        "baugenehmigung.antrag.0200" if bezug.namensraum == super::austausch::HOCHBAU => {
            &bezug.empfaenger
        }
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
    schreiben(d, Some(bezug))
}
fn schreiben(
    d: &Nachrichtendaten<'_>,
    bezug: Option<&super::austausch::Nachrichtenkopf>,
) -> CoreResult<String> {
    let fehlt = d.antrag.fehlende_angaben(d.buero, d.bauherren);
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
    x.auf(
        if bezug.is_some() {
            "xbau:baugenehmigung.antragGeaendert.0202"
        } else {
            "xbau:baugenehmigung.antrag.0200"
        },
        &[
            ("xmlns:xbau", NAMENSRAUM),
            ("xmlns:bn", "http://xoev.de/schemata/basisnachricht/g2g/1_1"),
            ("produkt", "BauProjektIO"),
            ("produkthersteller", "PixelCraft"),
            ("produktversion", env!("CARGO_PKG_VERSION")),
            ("standard", "XBau-Hochbau"),
            ("version", XBAU_VERSION),
        ],
    );

    // Nachrichtenkopf
    let zeit = d.zeitpunkt.to_rfc3339_opts(SecondsFormat::Secs, true);
    x.auf("nachrichtenkopf.g2g", &[])
        .auf("identifikation.nachricht", &[])
        .text("nachrichtenUUID", &d.nachricht_id.to_string())
        .auf(
            "nachrichtentyp",
            &[
                ("listURI", "urn:xoev-de:xbau:codeliste:xbau-nachrichten"),
                ("listVersionID", "5.0"),
            ],
        )
        .text("code", if bezug.is_some() { "0202" } else { "0200" })
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
        x.auf("xbau:referenzAntragsservice", &[])
            .text("referenzAntragsservice", &d.antrag.vorgang.to_string())
            .zu();
        for b in d.bauherren {
            x.auf("xbau:antragsteller", &[]);
            akteur(&mut x, b);
            x.zu();
        }
    }

    // Bauvorhaben
    let bezeichnung = match d.antrag.bezeichnung.trim() {
        "" => d.projekt.name.trim(),
        b => b,
    };
    x.auf(
        if bezug.is_some() {
            "xbau:bauvorhabenAenderungsdaten"
        } else {
            "xbau:bauvorhaben"
        },
        &[],
    )
    .text("xbau:bezeichnungDesBauvorhabens", bezeichnung);
    x.auf("xbau:beteiligte", &[]);
    for b in d.bauherren {
        x.auf("xbau:bauherr", &[]).auf("xbau:bauherr", &[]);
        akteur(&mut x, b);
        x.zu().zu();
    }
    // `fehlende_angaben` hat oben geprüft, dass es den Entwurfsverfasser gibt.
    let ev = d
        .antrag
        .entwurfsverfasser_von(d.buero)
        .ok_or_else(|| CoreError::InvalidInput("Entwurfsverfasser fehlt".into()))?;
    x.auf("xbau:entwurfsverfasser", &[])
        .auf("xbau:personendaten", &[]);
    akteur(
        &mut x,
        &Beteiligter::Person {
            id: ev.id,
            vorname: ev.vorname.clone(),
            familienname: ev.familienname.clone(),
            // Geschäftsanschrift des Büros.
            anschrift: d.buero.anschrift.clone(),
        },
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
    x.auf("xbau:gebuehrenpflichtiger", &[]);
    akteur(
        &mut x,
        d.antrag
            .gebuehrenpflichtiger
            .as_ref()
            .unwrap_or(&d.bauherren[0]),
    );
    x.zu();
    x.zu(); // beteiligte

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
    x.zu(); // gegenstand

    let g = &d.antrag.grundstueck;
    x.auf("xbau:verortung", &[])
        .auf("xbau:baugrundstueck", &[])
        .auf("xbau:grundstueck", &[]);
    if !g.anschrift.ist_leer() {
        x.auf("xbau:anschrift", &[]);
        anschrift_felder(&mut x, &g.anschrift);
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
        if gemarkung.chars().all(|c| c.is_ascii_digit()) {
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

    x.auf("xbau:anlagen", &[]);
    for a in d.anlagen {
        let mime = mime_typ(&a.dateiname).unwrap_or_default();
        x.auf("anlage", &[])
            .text("bezeichnung", a.bezeichnung.trim())
            .text("versionsnummer", &a.versionsnummer.to_string())
            .text("datum", &a.datum.to_string())
            .auf("anlageart", &[("listVersionID", anlagen.version.as_str())])
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

    x.zu();
    Ok(x.fertig())
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
fn akteur(x: &mut XmlSchreiber, b: &Beteiligter) {
    match b {
        Beteiligter::Person {
            id,
            vorname,
            familienname,
            anschrift,
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
            x.zu().zu();
        }
        Beteiligter::Organisation {
            id,
            name,
            anschrift,
        } => organisation(x, *id, name, anschrift),
    }
}

fn organisation(x: &mut XmlSchreiber, id: Uuid, name: &str, anschrift: &Anschrift) {
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
    x.zu();
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
                },
                Beteiligter::Organisation {
                    id: Uuid::new_v4(),
                    name: "Wohnbau Süd GmbH & Co. KG".into(),
                    anschrift: Anschrift::default(),
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
            anschrift: Anschrift::default(),
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

    fn anlagen() -> Vec<Anlagendatei> {
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
    fn schemafehler(xml: &str) -> Option<Result<(), String>> {
        let dir = tempfile_dir();
        let datei = dir.join("nachricht.xml");
        std::fs::write(&datei, xml).unwrap();
        let ergebnis = Command::new("xmllint")
            .args(["--noout", "--nonet", "--schema", "pruefschema.xsd"])
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

    fn daten<'a>(
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
            anschrift: Anschrift::default(),
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
}
