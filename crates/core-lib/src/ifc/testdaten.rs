//! Erzeugt kleine, vollständig bekannte IFC-Modelle für Tests, Oberflächen-
//! Fixtures und das Handbuch. Maße in Millimetern, damit die Umrechnung der
//! Längeneinheit mitgeprüft wird.
use super::step::{global_id, real, verschluesseln};

/// Schrittweiser Aufbau einer IFC4-Datei.
pub struct Bauer {
    zeilen: Vec<String>,
    n: u64,
    gid: u128,
}

impl Bauer {
    pub fn neu() -> Self {
        Self {
            zeilen: Vec::new(),
            n: 0,
            gid: 0x5bd1_e000_0000_0000_0000_0000_0000_0000,
        }
    }
    pub fn add(&mut self, s: impl AsRef<str>) -> u64 {
        self.n += 1;
        self.zeilen.push(format!("#{}={};", self.n, s.as_ref()));
        self.n
    }
    /// Feste, reproduzierbare GlobalId als STEP-Text.
    pub fn gid(&mut self) -> String {
        self.gid += 1;
        verschluesseln(&global_id(uuid::Uuid::from_u128(self.gid)))
    }
    pub fn pkt(&mut self, p: &[f64]) -> u64 {
        let k: Vec<String> = p.iter().map(|v| real(*v)).collect();
        self.add(format!("IFCCARTESIANPOINT(({}))", k.join(",")))
    }
    pub fn richtung(&mut self, p: &[f64]) -> u64 {
        let k: Vec<String> = p.iter().map(|v| real(*v)).collect();
        self.add(format!("IFCDIRECTION(({}))", k.join(",")))
    }
    pub fn achse(&mut self, o: [f64; 3], x: Option<[f64; 3]>) -> u64 {
        let p = self.pkt(&o);
        match x {
            Some(x) => {
                let z = self.richtung(&[0., 0., 1.]);
                let x = self.richtung(&x);
                self.add(format!("IFCAXIS2PLACEMENT3D(#{p},#{z},#{x})"))
            }
            None => self.add(format!("IFCAXIS2PLACEMENT3D(#{p},$,$)")),
        }
    }
    pub fn lage(&mut self, rel: Option<u64>, o: [f64; 3], x: Option<[f64; 3]>) -> u64 {
        let a = self.achse(o, x);
        let rel = rel.map(|r| format!("#{r}")).unwrap_or("$".into());
        self.add(format!("IFCLOCALPLACEMENT({rel},#{a})"))
    }
    pub fn text(&self) -> String {
        let mut s = String::from(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');\nFILE_NAME('Beispielmodell.ifc','2026-10-28T12:00:00',('BauPlanIO'),('BauPlanIO'),'','BauPlanIO Testdaten','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n",
        );
        for z in &self.zeilen {
            s.push_str(z);
            s.push('\n');
        }
        s.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        s
    }
}

/// Kennwerte des Beispielhauses, gegen die Tests prüfen.
pub mod efh {
    /// Außenmaße in Metern.
    pub const LAENGE: f64 = 10.;
    pub const BREITE: f64 = 8.;
    pub const WAND: f64 = 0.365;
    pub const INNENWAND: f64 = 0.115;
    pub const GESCHOSSHOEHE: f64 = 2.8;
    pub const LICHTE_HOEHE: f64 = 2.55;
    /// Lichte Fläche je Raum: (10 − 2 · 0,365 − 0,115) / 2 × (8 − 2 · 0,365).
    pub fn raumflaeche() -> f64 {
        (LAENGE - 2. * WAND - INNENWAND) / 2. * (BREITE - 2. * WAND)
    }
    /// Im Modell eingetragene, absichtlich abweichende NetFloorArea je Raum.
    pub const RAUM_MENGE: f64 = 33.0;
    pub const FIRST: f64 = 5.6;
}

/// Zweigeschossiges Einfamilienhaus (EG, OG), Flachdach, IFC4, Millimeter:
/// Außenwände 36,5 cm, Innenwand 11,5 cm, je Geschoss zwei Räume mit
/// Basismengen, Bodenplatte, Geschossdecke, Dach als Aggregat, ein
/// Schornstein als Flächenmodell, ein unklassifiziertes Bauteil (Proxy) und
/// eine Georeferenz (EPSG:25832).
pub fn efh_ifc4() -> String {
    use efh::*;
    let mm = |m: f64| m * 1000.;
    let mut b = Bauer::neu();
    let einheit = b.add("IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.)");
    let flaeche = b.add("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)");
    let volumen = b.add("IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)");
    let winkel = b.add("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)");
    let einheiten = b.add(format!(
        "IFCUNITASSIGNMENT((#{einheit},#{flaeche},#{volumen},#{winkel}))"
    ));
    let welt = b.achse([0., 0., 0.], None);
    let kontext = b.add(format!(
        "IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#{welt},$)"
    ));
    let body = b.add(format!(
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#{kontext},$,.MODEL_VIEW.,$)"
    ));
    let g = b.gid();
    let projekt = b.add(format!(
        "IFCPROJECT({g},$,'EFH Mustermann',$,$,'Neubau Einfamilienhaus',$,(#{kontext}),#{einheiten})"
    ));
    let crs = b.add("IFCPROJECTEDCRS('EPSG:25832','ETRS89 / UTM zone 32N',$,$,$,$,$)");
    b.add(format!(
        "IFCMAPCONVERSION(#{kontext},#{crs},356000.,5645000.,92.5,1.,0.,$)"
    ));
    let site_lage = b.lage(None, [0., 0., 0.], None);
    let g = b.gid();
    let site = b.add(format!(
        "IFCSITE({g},$,'Grundstück',$,$,#{site_lage},$,$,.ELEMENT.,$,$,92500.,$,$)"
    ));
    let bau_lage = b.lage(Some(site_lage), [0., 0., 0.], None);
    let g = b.gid();
    let haus = b.add(format!(
        "IFCBUILDING({g},$,'Wohnhaus',$,$,#{bau_lage},$,$,.ELEMENT.,$,$,$)"
    ));
    let g = b.gid();
    b.add(format!("IFCRELAGGREGATES({g},$,$,$,#{projekt},(#{site}))"));
    let g = b.gid();
    b.add(format!("IFCRELAGGREGATES({g},$,$,$,#{site},(#{haus}))"));

    // Hilfsfunktionen für Extrusionen.
    let extrusion = |b: &mut Bauer, profil: u64, lage: [f64; 3], tiefe: f64, _ziel: u64| -> u64 {
        let pos = b.achse(lage, None);
        let r = b.richtung(&[0., 0., 1.]);
        let solid = b.add(format!(
            "IFCEXTRUDEDAREASOLID(#{profil},#{pos},#{r},{})",
            real(tiefe)
        ));
        let rep = b.add(format!(
            "IFCSHAPEREPRESENTATION(#{body},'Body','SweptSolid',(#{solid}))"
        ));
        b.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))"))
    };
    let rechteck = |b: &mut Bauer, mitte: [f64; 2], x: f64, y: f64| -> u64 {
        let p = b.pkt(&mitte);
        let pos = b.add(format!("IFCAXIS2PLACEMENT2D(#{p},$)"));
        b.add(format!(
            "IFCRECTANGLEPROFILEDEF(.AREA.,$,#{pos},{},{})",
            real(x),
            real(y)
        ))
    };
    let polygonprofil = |b: &mut Bauer, pts: &[[f64; 2]]| -> u64 {
        let mut ids: Vec<String> = pts.iter().map(|p| format!("#{}", b.pkt(p))).collect();
        ids.push(ids[0].clone());
        let pl = b.add(format!("IFCPOLYLINE(({}))", ids.join(",")));
        b.add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#{pl})"))
    };
    let (l, br, t, ti) = (mm(LAENGE), mm(BREITE), mm(WAND), mm(INNENWAND));
    let wandhoehe = mm(GESCHOSSHOEHE) - 250.;
    let mut storeys = Vec::new();
    let mut alle_raeume = Vec::new();
    for (nr, (name, hoehe)) in [("Erdgeschoss", 0.), ("Obergeschoss", mm(GESCHOSSHOEHE))]
        .into_iter()
        .enumerate()
    {
        let lage = b.lage(Some(bau_lage), [0., 0., hoehe], None);
        let g = b.gid();
        let st = b.add(format!(
            "IFCBUILDINGSTOREY({g},$,'{}',$,$,#{lage},$,$,.ELEMENT.,{})",
            if nr == 0 { "EG" } else { "OG" },
            real(hoehe)
        ));
        let _ = name;
        storeys.push(st);
        let mut enthalten = Vec::new();
        // Außenwände (Achse in Wandmitte), Innenwand mittig.
        type Wand<'a> = ([f64; 3], [f64; 3], f64, f64, &'a str);
        let waende: [Wand; 5] = [
            ([0., t / 2., 0.], [1., 0., 0.], l, t, "Außenwand Süd"),
            ([0., br - t / 2., 0.], [1., 0., 0.], l, t, "Außenwand Nord"),
            (
                [t / 2., t, 0.],
                [0., 1., 0.],
                br - 2. * t,
                t,
                "Außenwand West",
            ),
            (
                [l - t / 2., t, 0.],
                [0., 1., 0.],
                br - 2. * t,
                t,
                "Außenwand Ost",
            ),
            ([l / 2., t, 0.], [0., 1., 0.], br - 2. * t, ti, "Innenwand"),
        ];
        let mut wand_ids = Vec::new();
        for (o, x, laenge, dicke, wname) in waende {
            let wl = b.lage(Some(lage), o, Some(x));
            let prof = rechteck(&mut b, [laenge / 2., 0.], laenge, dicke);
            let shape = extrusion(&mut b, prof, [0., 0., 0.], wandhoehe, 0);
            let g = b.gid();
            let w = b.add(format!(
                "IFCWALL({g},$,{},$,$,#{wl},#{shape},$,.STANDARD.)",
                verschluesseln(wname)
            ));
            enthalten.push(w);
            wand_ids.push((w, wname.starts_with("Außen")));
        }
        // Pset_WallCommon.IsExternal
        for aussen in [true, false] {
            let ids: Vec<String> = wand_ids
                .iter()
                .filter(|(_, a)| *a == aussen)
                .map(|(w, _)| format!("#{w}"))
                .collect();
            let p = b.add(format!(
                "IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.{}.),$)",
                if aussen { "T" } else { "F" }
            ));
            let g = b.gid();
            let ps = b.add(format!("IFCPROPERTYSET({g},$,'Pset_WallCommon',$,(#{p}))"));
            let g = b.gid();
            b.add(format!(
                "IFCRELDEFINESBYPROPERTIES({g},$,$,$,({}),#{ps})",
                ids.join(",")
            ));
        }
        // Decke über dem Geschoss (EG) bzw. Dach (OG).
        let platte = polygonprofil(&mut b, &[[0., 0.], [l, 0.], [l, br], [0., br]]);
        let sl = b.lage(Some(lage), [0., 0., 0.], None);
        let shape = extrusion(&mut b, platte, [0., 0., wandhoehe], 250., 0);
        let g = b.gid();
        if nr == 0 {
            let d = b.add(format!(
                "IFCSLAB({g},$,{},$,$,#{sl},#{shape},$,.FLOOR.)",
                verschluesseln("Decke über EG")
            ));
            enthalten.push(d);
            let bp = polygonprofil(&mut b, &[[0., 0.], [l, 0.], [l, br], [0., br]]);
            let bl = b.lage(Some(lage), [0., 0., 0.], None);
            let shape = extrusion(&mut b, bp, [0., 0., -250.], 250., 0);
            let g = b.gid();
            enthalten.push(b.add(format!(
                "IFCSLAB({g},$,'Bodenplatte',$,$,#{bl},#{shape},$,.BASESLAB.)"
            )));
            // Schornstein als Flächenmodell (Quader 50 × 50 cm, 6,5 m hoch).
            let (x0, y0, x1, y1, z1) = (7000., 6000., 7500., 6500., mm(FIRST) + 900.);
            let ecken: Vec<u64> = [
                [x0, y0, 0.],
                [x1, y0, 0.],
                [x1, y1, 0.],
                [x0, y1, 0.],
                [x0, y0, z1],
                [x1, y0, z1],
                [x1, y1, z1],
                [x0, y1, z1],
            ]
            .iter()
            .map(|p| b.pkt(p))
            .collect();
            let flaechen = [
                [0, 3, 2, 1],
                [4, 5, 6, 7],
                [0, 1, 5, 4],
                [1, 2, 6, 5],
                [2, 3, 7, 6],
                [3, 0, 4, 7],
            ];
            let mut faces = Vec::new();
            for f in flaechen {
                let pl = b.add(format!(
                    "IFCPOLYLOOP((#{},#{},#{},#{}))",
                    ecken[f[0]], ecken[f[1]], ecken[f[2]], ecken[f[3]]
                ));
                let bd = b.add(format!("IFCFACEOUTERBOUND(#{pl},.T.)"));
                faces.push(format!("#{}", b.add(format!("IFCFACE((#{bd}))"))));
            }
            let shell = b.add(format!("IFCCLOSEDSHELL(({}))", faces.join(",")));
            let brep = b.add(format!("IFCFACETEDBREP(#{shell})"));
            let rep = b.add(format!(
                "IFCSHAPEREPRESENTATION(#{body},'Body','Brep',(#{brep}))"
            ));
            let shape = b.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))"));
            let cl = b.lage(Some(lage), [0., 0., 0.], None);
            let g = b.gid();
            enthalten.push(b.add(format!(
                "IFCCHIMNEY({g},$,'Schornstein',$,$,#{cl},#{shape},$,$)"
            )));
            // Unklassifiziertes Bauteil: Kellerlichtschacht als Proxy.
            let prof = rechteck(&mut b, [500., -400.], 1000., 800.);
            let pl = b.lage(Some(lage), [1000., 0., 0.], None);
            let shape = extrusion(&mut b, prof, [0., 0., -800.], 800., 0);
            let g = b.gid();
            enthalten.push(b.add(format!(
                "IFCBUILDINGELEMENTPROXY({g},$,'Lichtschacht',$,$,#{pl},#{shape},$,$)"
            )));
        } else {
            let platte_slab = b.add(format!(
                "IFCSLAB({g},$,'Dachplatte',$,$,#{sl},#{shape},$,.ROOF.)"
            ));
            let rl = b.lage(Some(lage), [0., 0., 0.], None);
            let g = b.gid();
            let dach = b.add(format!(
                "IFCROOF({g},$,'Flachdach',$,$,#{rl},$,$,.FLAT_ROOF.)"
            ));
            let g = b.gid();
            b.add(format!(
                "IFCRELAGGREGATES({g},$,$,$,#{dach},(#{platte_slab}))"
            ));
            enthalten.push(dach);
        }
        let ids: Vec<String> = enthalten.iter().map(|i| format!("#{i}")).collect();
        let g = b.gid();
        b.add(format!(
            "IFCRELCONTAINEDINSPATIALSTRUCTURE({g},$,$,$,({}),#{st})",
            ids.join(",")
        ));
        // Räume: links und rechts der Innenwand.
        let breite_raum = (l - 2. * t - ti) / 2.;
        let tiefe_raum = br - 2. * t;
        let mut raeume = Vec::new();
        for (k, (nummer, rname, x0)) in [
            (
                format!("{}.01", nr),
                if nr == 0 { "Wohnen" } else { "Schlafen" },
                t,
            ),
            (
                format!("{}.02", nr),
                if nr == 0 { "Küche" } else { "Kind" },
                l / 2. + ti / 2.,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let prof = rechteck(
                &mut b,
                [breite_raum / 2., tiefe_raum / 2.],
                breite_raum,
                tiefe_raum,
            );
            let rl = b.lage(Some(lage), [x0, t, 0.], None);
            let shape = extrusion(&mut b, prof, [0., 0., 0.], mm(LICHTE_HOEHE), 0);
            let g = b.gid();
            let r = b.add(format!(
                "IFCSPACE({g},$,'{nummer}',$,$,#{rl},#{shape},{},.ELEMENT.,.INTERNAL.,$)",
                verschluesseln(rname)
            ));
            let q = b.add(format!(
                "IFCQUANTITYAREA('NetFloorArea',$,$,{},$)",
                real(RAUM_MENGE)
            ));
            let qh = b.add(format!(
                "IFCQUANTITYLENGTH('Height',$,$,{},$)",
                real(mm(LICHTE_HOEHE))
            ));
            let g = b.gid();
            let qs = b.add(format!(
                "IFCELEMENTQUANTITY({g},$,'Qto_SpaceBaseQuantities',$,$,(#{q},#{qh}))"
            ));
            let g = b.gid();
            b.add(format!("IFCRELDEFINESBYPROPERTIES({g},$,$,$,(#{r}),#{qs})"));
            let _ = k;
            raeume.push(r);
        }
        let ids: Vec<String> = raeume.iter().map(|i| format!("#{i}")).collect();
        let g = b.gid();
        b.add(format!(
            "IFCRELAGGREGATES({g},$,$,$,#{st},({}))",
            ids.join(",")
        ));
        alle_raeume.extend(raeume);
    }
    let ids: Vec<String> = storeys.iter().map(|i| format!("#{i}")).collect();
    let g = b.gid();
    b.add(format!(
        "IFCRELAGGREGATES({g},$,$,$,#{haus},({}))",
        ids.join(",")
    ));
    let _ = alle_raeume;
    b.text()
}

/// Minimales IFC2X3-Modell (Meter, `IfcWallStandardCase` mit acht Attributen,
/// Geschoss ohne Elevation, Grundstück mit geographischer Lage).
pub fn wand_ifc2x3() -> String {
    "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2.0]'),'2;1');\nFILE_NAME('wand.ifc','2026-10-28T12:00:00',(''),(''),'','Revit 2024','');\nFILE_SCHEMA(('IFC2X3'));\nENDSEC;\nDATA;\n\
#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n#2=IFCUNITASSIGNMENT((#1));\n\
#3=IFCPROJECT('3YfS1x0ab1ZPmJYjeT7N2c',$,'Wand',$,$,$,$,$,#2);\n\
#4=IFCCARTESIANPOINT((0.,0.,0.));\n#5=IFCAXIS2PLACEMENT3D(#4,$,$);\n#6=IFCLOCALPLACEMENT($,#5);\n\
#7=IFCSITE('0kF7Zh6RT0FBXBzdeuWbE8',$,'Gelände',$,$,#6,$,$,.ELEMENT.,(51,30,0,0),(7,0,0,0),110.,$,$);\n\
#8=IFCBUILDING('2nR8D2zbb4LeyCQ4SkYLzE',$,'Haus',$,$,#6,$,$,.ELEMENT.,$,$,$);\n\
#9=IFCCARTESIANPOINT((0.,0.,3.));\n#10=IFCAXIS2PLACEMENT3D(#9,$,$);\n#11=IFCLOCALPLACEMENT(#6,#10);\n\
#12=IFCBUILDINGSTOREY('1f7QkxmCX9pPt8wH3fGJHD',$,'1. OG',$,$,#11,$,$,.ELEMENT.,$);\n\
#13=IFCCARTESIANPOINT((0.,0.));\n#14=IFCCARTESIANPOINT((5.,0.));\n#15=IFCCARTESIANPOINT((5.,0.24));\n#16=IFCCARTESIANPOINT((0.,0.24));\n\
#17=IFCPOLYLINE((#13,#14,#15,#16,#13));\n#18=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#17);\n\
#19=IFCDIRECTION((0.,0.,1.));\n#20=IFCEXTRUDEDAREASOLID(#18,#5,#19,2.5);\n\
#21=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#20));\n#22=IFCPRODUCTDEFINITIONSHAPE($,$,(#21));\n\
#23=IFCLOCALPLACEMENT(#11,#5);\n\
#24=IFCWALLSTANDARDCASE('2O2Fr$t4X7Zf8NOew3FLOH',$,'Wand 1',$,$,#23,#22,$);\n\
#25=IFCRELCONTAINEDINSPATIALSTRUCTURE('0Yv9yG5Y15K8M2Qq7d1Wvl',$,$,$,(#24),#12);\n\
#26=IFCRELAGGREGATES('1Yv9yG5Y15K8M2Qq7d1Wvl',$,$,$,#8,(#12));\n\
#27=IFCRELAGGREGATES('2Yv9yG5Y15K8M2Qq7d1Wvl',$,$,$,#7,(#8));\n\
ENDSEC;\nEND-ISO-10303-21;\n".into()
}
