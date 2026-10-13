// LBO-Regelpaket Bayern
//
// BEISPIELHAFTE UMSETZUNG – vor produktivem Einsatz fachlich und rechtlich
// prüfen (Sonderregeln, Satzungen der Gemeinden, Giebelflächen usw.).

fn meta() {
    #{
        land: "BY",
        version: "2026.1",
        rechtsstand: "BayBO, Art. 6 Abs. 5 (Abstandsflächen)",
    }
}

// Art. 6 Abs. 5 Satz 1 BayBO: 0,4 H, mindestens 3 m;
// in Gewerbe- und Industriegebieten 0,2 H, mindestens 3 m.
fn abstandsflaeche(input) {
    let faktor = if input.gebiet == "GE" || input.gebiet == "GI" { 0.2 } else { 0.4 };
    let mindesttiefe = 3.0;
    #{
        tiefe_m: aufrunden(max(faktor * input.wandhoehe_m, mindesttiefe), 2),
        faktor: faktor,
        mindesttiefe_m: mindesttiefe,
        rechtsgrundlage: "Art. 6 Abs. 5 Satz 1 BayBO",
    }
}
