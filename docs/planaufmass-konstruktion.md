# Zeicheneditor: Konstruieren, Darstellung, Planlayout, DXF

Ausbau vom 01.11.2026 (Ausbaukonzept, Schritte 5–10). Der Editor bleibt ein
Werkzeug für Genehmigungsplanung und Nachweise neben dem Büro-CAD.

| Thema | Oberfläche | Technik |
|---|---|---|
| Fensterauswahl, Kopieren, Spiegeln, Stutzen/Dehnen, Versatz, Reihe | Werkzeuge C, I, D; Bereich „Konstruieren“ | `planaufmass/konstruktion.ts` (reine Geometrie, Tests `apps/desktop/tests/konstruktion.test.ts`) |
| Ortho (F8), Winkelraster, Lot- und Verlängerungsfang, Koordinateneingabe (@), Befehlszeile (F2), Wiederholen (Enter) | `PdfPlan.svelte`, `Planaufmass.svelte` | `konstruktion.ts` (`eingabeAuswerten`, `BEFEHLE`), `fang.ts` |
| Stift, Linienart, Farbe, Schraffur, Bauzustand je Ebene/Element | Eigenschaften → Darstellung | `Darstellung` in `core_lib::planaufmass`, `darstellung.ts` |
| Öffnungen in Wänden, Planzeichen, Raumstempel | Werkzeuge F, G, N | `Zeichenform::Wand.oeffnungen`, `Symbol`, `Raumstempel`; `planzeichen.ts` |
| Planblatt mit Plankopf, Nordpfeil, Maßstabsleiste, Legende | „Planlayout“ | `core_lib::planlayout`, `druckausgabe::planlayout` |
| Plankopf-Stempel für fremde PDFs | Planliste → „Plankopf stempeln …“ | `plankopf_stempeln` |
| Planvergleich mit Revisionswolken | Planliste → „Planvergleich“ | `planvergleich.ts`, `plan_revisionswolken` |
| DXF-Ausgabe | „DXF fürs CAD …“ | `cad::dxfexport` (Crate `dxf`, MIT): Meter, y nach oben, eine Ebene je Zeichenebene |

Grenzen: keine Linientyp-Tabellen und Schraffuren im DXF, kein DWG, keine
assoziative Bemaßung, keine Geschossverwaltung in eigenen Zeichnungen.
