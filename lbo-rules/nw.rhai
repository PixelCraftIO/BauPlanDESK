// LBO-Regelpaket Nordrhein-Westfalen
//
// BEISPIELHAFTE UMSETZUNG – vor produktivem Einsatz fachlich und rechtlich
// prüfen.

fn meta() {
    #{
        land: "NW",
        version: "2026.2",
        rechtsstand: "BauO NRW 2018, § 6 Abs. 5 (Abstandsflächen), Fassung ab 01.09.2026",
    }
}

// § 6 Abs. 5 Satz 1 und 2 BauO NRW: 0,4 H, in Gewerbe- und Industriegebieten
// 0,2 H, in Kerngebieten 0,25 H, jeweils mindestens 3 m.
fn abstandsflaeche(input) {
    let faktor = if input.gebiet == "GE" || input.gebiet == "GI" {
        0.2
    } else if input.gebiet == "MK" {
        0.25
    } else {
        0.4
    };
    let mindesttiefe = 3.0;
    #{
        tiefe_m: aufrunden(max(faktor * input.wandhoehe_m, mindesttiefe), 2),
        faktor: faktor,
        mindesttiefe_m: mindesttiefe,
        rechtsgrundlage: "§ 6 Abs. 5 BauO NRW",
    }
}
