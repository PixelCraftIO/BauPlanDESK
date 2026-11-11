/**
 * Ausrichtung zweier Planstände. Die Vergleichsunterlage (neuer Stand) liegt
 * über dem aktuellen Blatt: Ein Punkt q des neuen Blatts erscheint bei
 * `d + R(grad) · faktor · q`. Ohne Laufzeit-Importe (node-Test).
 */
export interface Planausrichtung { dx: number; dy: number; grad: number; faktor: number }
type P = [number, number];

/** Neuer Blattpunkt → Lage auf dem aktuellen Blatt. */
export function aufAltesBlatt(t: Planausrichtung): (q: P) => P {
  const a = t.grad * Math.PI / 180, c = Math.cos(a), s = Math.sin(a);
  return q => [t.dx + t.faktor * (q[0] * c - q[1] * s), t.dy + t.faktor * (q[0] * s + q[1] * c)];
}
/** Punkt des aktuellen Blatts → Punkt auf dem neuen Blatt. */
export function aufNeuesBlatt(t: Planausrichtung): (p: P) => P {
  if (![t.dx, t.dy, t.grad, t.faktor].every(Number.isFinite) || t.faktor <= 0) throw Error("Ausrichtung benötigt endliche Werte und eine positive Skalierung.");
  const a = -t.grad * Math.PI / 180, c = Math.cos(a), s = Math.sin(a);
  return p => { const x = (p[0] - t.dx) / t.faktor, y = (p[1] - t.dy) / t.faktor; return [x * c - y * s, x * s + y * c]; };
}
/**
 * Ausrichtung aus zwei Passpunkten: `alt` sind zwei Punkte im aktuellen Plan,
 * `angezeigt` dieselben zwei Punkte, wie sie die Vergleichsunterlage mit der
 * bisherigen Ausrichtung `bisher` zeigt. Liegen die Punkte eines Paars
 * aufeinander, gibt es keine Lösung (`null`).
 */
export function ausPasspunkten(alt: [P, P], angezeigt: [P, P], bisher: Planausrichtung): Planausrichtung | null {
  const zurueck = aufNeuesBlatt(bisher), q1 = zurueck(angezeigt[0]), q2 = zurueck(angezeigt[1]);
  const va: P = [alt[1][0] - alt[0][0], alt[1][1] - alt[0][1]], vq: P = [q2[0] - q1[0], q2[1] - q1[1]];
  const la = Math.hypot(va[0], va[1]), lq = Math.hypot(vq[0], vq[1]);
  if (!(la > 1e-6) || !(lq > 1e-6)) return null;
  const faktor = la / lq, w = Math.atan2(va[1], va[0]) - Math.atan2(vq[1], vq[0]), c = Math.cos(w), s = Math.sin(w);
  let grad = w * 180 / Math.PI; if (grad > 180) grad -= 360; if (grad <= -180) grad += 360;
  return { dx: alt[0][0] - faktor * (q1[0] * c - q1[1] * s), dy: alt[0][1] - faktor * (q1[0] * s + q1[1] * c), grad, faktor };
}
/** Rundet eine Ausrichtung für die Anzeige, ohne die Lage sichtbar zu ändern. */
export const gerundet = (t: Planausrichtung): Planausrichtung => ({ dx: Math.round(t.dx * 100) / 100, dy: Math.round(t.dy * 100) / 100, grad: Math.round(t.grad * 1000) / 1000, faktor: Math.round(t.faktor * 1e5) / 1e5 });
