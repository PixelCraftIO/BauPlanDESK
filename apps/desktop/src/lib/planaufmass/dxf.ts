/** DXF-Unterlage in der Zeichenfläche: Seitenpunkte sind Zeichnungseinheiten
 * ab der linken oberen Ecke des Zeichnungsumfangs, y nach unten (spiegelt
 * `core_lib::planaufmass::seitenpunkt`). Gemessen wird im Core über die
 * Kalibrierung „Zeichnungseinheit“; hier wird nur dargestellt und gefangen.
 */
import type { Plan, Punkt } from "../api";
import type { Point, SnapGeometry } from "./pdfGeometry";

export interface DxfBlatt { breite: number; hoehe: number; min: Punkt; max: Punkt }

export function dxfBlatt(plan: Plan): DxfBlatt | null {
  const g = plan.grenzen;
  if (!g) return null;
  return { breite: Math.max(g.max[0] - g.min[0], 1e-9), hoehe: Math.max(g.max[1] - g.min[1], 1e-9), min: g.min, max: g.max };
}

/** Zeichnungspunkt → Seitenpunkt. */
export const seitenpunkt = (p: Punkt, b: DxfBlatt): Point => [p[0] - b.min[0], b.max[1] - p[1]];

const r = (n: number) => +n.toFixed(4);
/** Vordergrundfarbe für Ebenen ohne eigene Farbe (ACI 7) auf weißem Blatt. */
const VORDERGRUND = "#222222";

/** Ein SVG-Pfad je Farbe über alle sichtbaren Linienzüge. */
export function dxfPfade(plan: Plan, b: DxfBlatt, sichtbar: boolean[]): { farbe: string; d: string }[] {
  const m = new Map<string, string[]>();
  for (const l of plan.linien) {
    if (!(sichtbar[l.ebene] ?? true) || !l.punkte.length) continue;
    const farbe = l.farbe ?? plan.ebenen[l.ebene]?.farbe ?? VORDERGRUND;
    const teile = l.punkte.map((p, i) => { const q = seitenpunkt(p, b); return `${i ? "L" : "M"}${r(q[0])} ${r(q[1])}`; });
    if (l.geschlossen) teile.push("Z");
    let d = m.get(farbe);
    if (!d) m.set(farbe, (d = []));
    d.push(teile.join(" "));
  }
  return [...m].map(([farbe, ds]) => ({ farbe, d: ds.join(" ") }));
}

export interface DxfText { x: number; y: number; hoehe: number; zeilenabstand: number; drehung: number; anker: "start" | "middle" | "end"; farbe: string; zeilen: string[] }

export function dxfTexte(plan: Plan, b: DxfBlatt, sichtbar: boolean[]): DxfText[] {
  return plan.texte.filter(t => (sichtbar[t.ebene] ?? true) && t.hoehe > 0).map(t => {
    const [x, y] = seitenpunkt(t.position, b);
    return {
      x, y, hoehe: t.hoehe, zeilenabstand: t.zeilenabstand, drehung: t.drehung,
      anker: t.ausrichtung === "mitte" ? "middle" : t.ausrichtung === "rechts" ? "end" : "start",
      farbe: t.farbe ?? plan.ebenen[t.ebene]?.farbe ?? VORDERGRUND,
      zeilen: t.inhalt.split("\n"),
    };
  });
}

/** Fanggeometrie der sichtbaren Linienzüge (begrenzt wie bei PDF-Vektoren). */
export function dxfGeometrie(plan: Plan, b: DxfBlatt, sichtbar: boolean[], limit = 100000): SnapGeometry {
  const g: SnapGeometry = { points: [], segments: [], limited: false };
  for (const l of plan.linien) {
    if (!(sichtbar[l.ebene] ?? true)) continue;
    let vorher: Point | null = null, erster: Point | null = null;
    for (const p of l.punkte) {
      if (g.points.length + g.segments.length >= limit) { g.limited = true; return g; }
      const q = seitenpunkt(p, b);
      g.points.push(q);
      if (vorher) g.segments.push([vorher, q]);
      vorher = q; erster ??= q;
    }
    if (l.geschlossen && erster && vorher && l.punkte.length > 2) g.segments.push([vorher, erster]);
  }
  return g;
}
