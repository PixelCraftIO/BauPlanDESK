/**
 * Wände mit Öffnungen und Planzeichen für die Anzeige im Zeicheneditor. Gleiche
 * Geometrie wie die PDF-Ausgabe im Core (`planlayout::wandbild`, `symbolbild`).
 * Ohne Laufzeit-Importe, damit `node --test` die Datei prüfen kann.
 */
import type { Oeffnung, Plansymbol } from "../generated/plattform";

type P = [number, number];

/** Wandstücke zwischen den Öffnungen und Linien für Fenster und Türen; `mJeEinheit` = Meter je Blatteinheit. */
export function wandbild(von: P, bis: P, dickeM: number, oeffnungen: Oeffnung[] | undefined, mJeEinheit: number) {
  const l = Math.max(1e-9, Math.hypot(bis[0] - von[0], bis[1] - von[1]));
  const u: P = [(bis[0] - von[0]) / l, (bis[1] - von[1]) / l];
  const d = dickeM / mJeEinheit / 2;
  const n: P = [-u[1] * d, u[0] * d];
  const at = (t: number, s: number): P => [von[0] + u[0] * t + n[0] * s, von[1] + u[1] * t + n[1] * s];
  const os = [...(oeffnungen ?? [])].sort((a, b) => a.lage_m - b.lage_m);
  const stuecke: P[][] = [], linien: P[][] = [], boegen: string[] = [];
  let t0 = 0;
  for (const o of os) {
    const a = Math.min(l, Math.max(t0, o.lage_m / mJeEinheit)), b = Math.min(l, Math.max(a, (o.lage_m + o.breite_m) / mJeEinheit));
    if (a > t0 + 1e-9) stuecke.push([at(t0, 1), at(a, 1), at(a, -1), at(t0, -1)]);
    if (o.art === "fenster") for (const s of [1, 0.25, -0.25, -1]) linien.push([at(a, s), at(b, s)]);
    else {
      const s = o.links ? 1 : -1, w = b - a, ns: P = [(n[0] / (d || 1)) * s, (n[1] / (d || 1)) * s], h = at(a, s);
      const ende: P = [h[0] + ns[0] * w, h[1] + ns[1] * w], ziel = at(b, s);
      linien.push([h, ende]);
      // Viertelkreis vom Türblatt zum Öffnungsende.
      const kreuz = ns[0] * u[1] - ns[1] * u[0];
      boegen.push(`M${ende[0]} ${ende[1]} A${w} ${w} 0 0 ${kreuz > 0 ? 0 : 1} ${ziel[0]} ${ziel[1]}`);
    }
    t0 = b;
  }
  if (l > t0 + 1e-9) stuecke.push([at(t0, 1), at(l, 1), at(l, -1), at(t0, -1)]);
  return { stuecke, linien, boegen };
}

/** Linienzüge eines Planzeichens: [Punkte, geschlossen, gefüllt]. */
export function symbolbild(s: Plansymbol, p: P, g: number, grad: number): [P[], boolean, boolean][] {
  const w = (-grad * Math.PI) / 180, sn = Math.sin(w), cs = Math.cos(w);
  const at = (x: number, y: number): P => [p[0] + (x * cs - y * sn) * g, p[1] + (x * sn + y * cs) * g];
  const kreis = (r: number) => Array.from({ length: 32 }, (_, i) => at(r * Math.cos((i / 32) * 2 * Math.PI), r * Math.sin((i / 32) * 2 * Math.PI)));
  switch (s) {
    case "baum_bestand": return [[kreis(0.5), true, false], [kreis(0.05), true, true]];
    case "baum_neu": return [[kreis(0.5), true, false], [kreis(0.2), true, false]];
    case "baum_faellung": return [[kreis(0.5), true, false], [[at(-0.45, -0.45), at(0.45, 0.45)], false, false], [[at(-0.45, 0.45), at(0.45, -0.45)], false, false]];
    case "stellplatz": return [[[at(-0.25, -0.5), at(0.25, -0.5), at(0.25, 0.5), at(-0.25, 0.5)], true, false], [[at(-0.08, 0.15), at(-0.08, -0.15), at(0.05, -0.15), at(0.1, -0.08), at(0.05, 0), at(-0.08, 0)], false, false]];
    case "zufahrt": return [[[at(0, 0.5), at(0, -0.5)], false, false], [[at(-0.2, -0.25), at(0, -0.5), at(0.2, -0.25)], true, true]];
    case "hauseingang": return [[[at(-0.3, 0.25), at(0.3, 0.25), at(0, -0.25)], true, true]];
    default: return [[kreis(0.15), true, false], [kreis(0.04), true, true]];
  }
}
export const PLANSYMBOLE: [Plansymbol, string][] = [["baum_bestand", "Baum (Bestand)"], ["baum_neu", "Baum (geplant)"], ["baum_faellung", "Baum (Fällung)"], ["stellplatz", "Stellplatz"], ["zufahrt", "Zufahrt"], ["hauseingang", "Hauseingang"], ["grenzpunkt", "Grenzpunkt"]];

/** Lage einer neuen Öffnung: Abstand (m) ab Wandanfang, mittig um den Klickpunkt; null, wenn sie nicht passt. */
export function oeffnungLage(von: P, bis: P, klick: P, breiteM: number, mJeEinheit: number, belegt: Oeffnung[] | undefined): number | null {
  const l = Math.hypot(bis[0] - von[0], bis[1] - von[1]);
  if (!(l > 0)) return null;
  const t = (((klick[0] - von[0]) * (bis[0] - von[0]) + (klick[1] - von[1]) * (bis[1] - von[1])) / l) * mJeEinheit;
  const laenge = l * mJeEinheit;
  const lage = Math.min(Math.max(0, t - breiteM / 2), laenge - breiteM);
  if (lage < 0) return null;
  const frei = (belegt ?? []).every(o => lage + breiteM <= o.lage_m + 1e-6 || o.lage_m + o.breite_m <= lage + 1e-6);
  return frei ? Math.round(lage * 1000) / 1000 : null;
}
