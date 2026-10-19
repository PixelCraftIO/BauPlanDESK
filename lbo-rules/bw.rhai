// LBO-Regelpaket Baden-Württemberg
//
// BEISPIELHAFTE UMSETZUNG – vor produktivem Einsatz fachlich und rechtlich
// prüfen (Sonderregeln für kurze Wände, Satzungen usw.).

fn meta() {
    #{
        land: "BW",
        version: "2026.2",
        rechtsstand: "LBO BW, § 5 Abs. 7 (Abstandsflächen), Reform 2025",
    }
}

// § 5 Abs. 7 LBO BW: 0,4 H allgemein; 0,2 H in Kern-, Dorf-, urbanen und
// besonderen Wohngebieten; 0,125 H in Gewerbe- und Industriegebieten;
// mindestens 2,5 m. Muss mit regelwerke/abstandsflaechen/pilot.json
// übereinstimmen (Test `shipped_lbo_pakete_entsprechen_dem_katalog`).
fn abstandsflaeche(input) {
    let faktor = switch input.gebiet {
        "MK" | "MD" | "MDW" | "MU" | "WB" => 0.2,
        "GE" | "GI" => 0.125,
        _ => 0.4,
    };
    let mindesttiefe = 2.5;
    #{
        tiefe_m: aufrunden(max(faktor * input.wandhoehe_m, mindesttiefe), 2),
        faktor: faktor,
        mindesttiefe_m: mindesttiefe,
        rechtsgrundlage: "§ 5 Abs. 7 LBO BW",
    }
}
