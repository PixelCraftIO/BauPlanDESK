/**
 * Eigene Zeichnungen: Blatt in Metern (y nach unten), eingepasste Unterlagen als
 * Hintergrund und Zeichenelemente (Wand, Maßkette, Text). Reine Geometrie für die
 * Anzeige; Messwerte kommen weiterhin vom Core.
 */
import type { Planhintergrund, Zeichenelement, Zeichenform } from "../generated/plattform";
import type { Plan } from "../api";
import type { Point } from "./pdfGeometry";

/** Hintergrund samt geladener Unterlage (PDF-Seite oder DXF-Zeichnung). */
export type Hintergrundansicht = { h: Planhintergrund; pdfBase64?: string; plan?: Plan | null };

/** Kleinstes Blatt und Rand um die Inhalte (Meter). */
export const BLATT_MIN: [number, number] = [50, 35];
export const BLATT_RAND = 10;

/** Meter je PDF-Seitenpunkt bei Maßstab 1 : nenner (72 pt = 1 Zoll). */
export const mppAusMassstab = (nenner: number) => (nenner * 0.0254) / 72;

/** Punkt der Unterlage (Seitenpunkte) → Punkt der Zeichnung (Meter). */
export function hgPunkt(h: Planhintergrund, p: Point): Point {
  const w = (h.drehung_grad * Math.PI) / 180, c = Math.cos(w), s = Math.sin(w);
  const x = (p[0] - h.ursprung[0]) * h.meter_pro_punkt, y = (p[1] - h.ursprung[1]) * h.meter_pro_punkt;
  return [h.lage[0] + x * c - y * s, h.lage[1] + x * s + y * c];
}
/** SVG-Transformation der Unterlage auf das Blatt (gleiche Reihenfolge wie `hgPunkt`). */
export const hgTransform = (h: Planhintergrund) =>
  `translate(${h.lage[0]} ${h.lage[1]}) rotate(${h.drehung_grad}) scale(${h.meter_pro_punkt}) translate(${-h.ursprung[0]} ${-h.ursprung[1]})`;
/** Skaliert den Hintergrund so, dass die Strecke a→b (Blatt, m) die Länge `meter` erhält; a bleibt liegen. */
export function hgSkalieren(h: Planhintergrund, a: Point, b: Point, meter: number): Planhintergrund {
  const l = Math.hypot(b[0] - a[0], b[1] - a[1]);
  if (!(l > 0) || !(meter > 0)) return h;
  const f = meter / l;
  return { ...h, meter_pro_punkt: h.meter_pro_punkt * f, lage: [a[0] + (h.lage[0] - a[0]) * f, a[1] + (h.lage[1] - a[1]) * f] };
}
export function hgVerschieben(h: Planhintergrund, von: Point, bis: Point): Planhintergrund {
  return { ...h, lage: [h.lage[0] + bis[0] - von[0], h.lage[1] + bis[1] - von[1]] };
}
/** Dreht den Hintergrund um den Blattpunkt p (Grad, im Uhrzeigersinn bei y nach unten). */
export function hgDrehen(h: Planhintergrund, p: Point, grad: number): Planhintergrund {
  const w = (grad * Math.PI) / 180, c = Math.cos(w), s = Math.sin(w);
  const dx = h.lage[0] - p[0], dy = h.lage[1] - p[1];
  return { ...h, drehung_grad: h.drehung_grad + grad, lage: [p[0] + dx * c - dy * s, p[1] + dx * s + dy * c] };
}

/** Text, Planzeichen und Raumstempel haben einen Einfügepunkt, Wand und Maßkette eine Achse. */
export const punktform = (f: Zeichenform): f is Extract<Zeichenform, { punkt: [number, number] }> => "punkt" in f;
export function elementPunkte(f: Zeichenform): Point[] {
  return punktform(f) ? [f.punkt] : [f.von, f.bis];
}
export function elementSetzen<T extends Zeichenform>(f: T, index: number, p: Point): T {
  if (punktform(f)) return { ...f, punkt: p };
  return index === 0 ? { ...f, von: p } : { ...f, bis: p };
}
export function elementVerschieben<T extends Zeichenform>(f: T, dx: number, dy: number): T {
  const v = (p: Point): Point => [p[0] + dx, p[1] + dy];
  if (punktform(f)) return { ...f, punkt: v(f.punkt) };
  return { ...f, von: v(f.von), bis: v(f.bis) };
}
/** Wandkontur (vier Ecken) in Seitenpunkten; `dicke` ebenfalls in Seitenpunkten. */
export function wandKontur(von: Point, bis: Point, dicke: number): Point[] {
  const l = Math.hypot(bis[0] - von[0], bis[1] - von[1]) || 1;
  const nx = (-(bis[1] - von[1]) / l) * dicke / 2, ny = ((bis[0] - von[0]) / l) * dicke / 2;
  return [[von[0] + nx, von[1] + ny], [bis[0] + nx, bis[1] + ny], [bis[0] - nx, bis[1] - ny], [von[0] - nx, von[1] - ny]];
}
/** Maßkette: Maßlinie im Abstand `abstand` (Normale wie im Core: positiv = rechts der Richtung von→bis bei y nach unten), Hilfslinien, Textlage. */
export function masskette(von: Point, bis: Point, abstand: number) {
  const dx = bis[0] - von[0], dy = bis[1] - von[1], l = Math.hypot(dx, dy) || 1;
  const n: Point = [(-dy / l) * abstand, (dx / l) * abstand];
  const ueber: Point = [(-dy / l) * Math.sign(abstand || 1), (dx / l) * Math.sign(abstand || 1)];
  const a: Point = [von[0] + n[0], von[1] + n[1]], b: Point = [bis[0] + n[0], bis[1] + n[1]];
  let winkel = (Math.atan2(dy, dx) * 180) / Math.PI;
  if (winkel > 90 || winkel <= -90) winkel += 180;
  return { linie: [a, b] as [Point, Point], hilfen: [[von, a], [bis, b]] as [Point, Point][], ueber, mitte: [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2] as Point, winkel, laenge: l };
}
/** Blattgröße einer Zeichnung (Meter) aus allen Inhalten, mindestens `BLATT_MIN`, plus Rand. */
export function blattmass(punkte: Point[], hintergruende: { h: Planhintergrund; breite: number; hoehe: number }[]): { breite: number; hoehe: number } {
  let x = 0, y = 0;
  for (const p of punkte) { x = Math.max(x, p[0]); y = Math.max(y, p[1]); }
  for (const { h, breite, hoehe } of hintergruende)
    for (const e of [[0, 0], [breite, 0], [breite, hoehe], [0, hoehe]] as Point[]) {
      const q = hgPunkt(h, e); x = Math.max(x, q[0]); y = Math.max(y, q[1]);
    }
  return { breite: Math.max(BLATT_MIN[0], Math.ceil(x + BLATT_RAND)), hoehe: Math.max(BLATT_MIN[1], Math.ceil(y + BLATT_RAND)) };
}
/** Neuer Hintergrund für eine Unterlage; PDF-Seiten starten mit 1 : 100, DXF-Zeichnungen mit der Zeichnungseinheit. */
export function hintergrundNeu(dokument: { dokument_id: string; version: number }, seite: number, meterProPunkt: number): Planhintergrund {
  return { id: crypto.randomUUID(), dokument, seite, ursprung: [0, 0], lage: [0, 0], drehung_grad: 0, meter_pro_punkt: meterProPunkt, deckkraft: 0.6, sichtbar: true };
}
export const istElement = (e: Zeichenelement | undefined): e is Zeichenelement => !!e;
