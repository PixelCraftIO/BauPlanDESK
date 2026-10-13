// LBO-Regelpaket Baden-Württemberg
//
// BEISPIELHAFTE UMSETZUNG – vor produktivem Einsatz fachlich und rechtlich
// prüfen (Sonderregeln für kurze Wände, Satzungen usw.).

fn meta() {
    #{
        land: "BW",
        version: "2026.1",
        rechtsstand: "LBO BW, § 5 Abs. 7 (Abstandsflächen)",
    }
}

// § 5 Abs. 7 LBO BW: 0,4 H allgemein; 0,2 H in Kern-, Dorf- und urbanen
// Gebieten; 0,125 H in Gewerbe- und Industriegebieten; mindestens 2,5 m.
fn abstandsflaeche(input) {
    let faktor = switch input.gebiet {
        "MK" | "MD" | "MDW" | "MU" => 0.2,
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
