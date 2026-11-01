// Prüfung der Konstruktionsbefehle: npm run test:geometrie
import { test } from "node:test";
import assert from "node:assert/strict";
import { spiegelung, versatz, stutzen, reihe, fenster, imFenster, punkteVon, kantenVon, elementSpiegeln, geradenschnitt } from "../src/lib/planaufmass/konstruktion.ts";
import type { Plangeometrie } from "../src/lib/generated/plattform";

const nah = (a: number[], b: number[]) => a.forEach((v, i) => assert.ok(Math.abs(v - b[i]) < 1e-9, `${a} ≠ ${b}`));

test("Spiegeln an senkrechter Achse", () => {
  const s = spiegelung([5, 0], [5, 10]);
  nah(s([2, 3]), [8, 3]);
  nah(s([5, 7]), [5, 7]);
});
test("Maßkette wechselt beim Spiegeln die Seite", () => {
  const m = elementSpiegeln({ art: "masskette", von: [0, 0], bis: [4, 0], abstand: -1 }, [0, 5], [10, 5]);
  assert.equal(m.abstand, 1);
});
test("Versatz einer Strecke und eines Rechtecks nach außen", () => {
  const s = versatz({ art: "strecke", von: [0, 0], bis: [10, 0] }, 2) as Extract<Plangeometrie, { art: "strecke" }>;
  nah(s.von, [0, -2]); nah(s.bis, [10, -2]);
  const r = versatz({ art: "rechteck", von: [0, 0], bis: [10, 10] }, 1) as Extract<Plangeometrie, { art: "polygon" }>;
  const xs = r.punkte.map(p => p[0]), ys = r.punkte.map(p => p[1]);
  assert.deepEqual([Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)].map(v => Math.round(v * 1e9) / 1e9), [-1, 11, -1, 11]);
  const innen = versatz({ art: "polygon", punkte: [[0, 10], [10, 10], [10, 0], [0, 0]] }, -1) as Extract<Plangeometrie, { art: "polygon" }>;
  assert.ok(innen.punkte.every(p => p[0] >= 1 - 1e-9 && p[0] <= 9 + 1e-9));
});
test("Stutzen kürzt, Dehnen verlängert", () => {
  nah(stutzen([0, 0], [10, 0], [6, -5], [6, 5])!.flat(), [0, 0, 6, 0]);
  nah(stutzen([0, 0], [10, 0], [15, -5], [15, 5])!.flat(), [0, 0, 15, 0]);
  nah(stutzen([0, 0], [10, 0], [-3, -5], [-3, 5])!.flat(), [-3, 0, 10, 0]);
  assert.equal(stutzen([0, 0], [10, 0], [0, 2], [10, 2]), null);
  assert.equal(geradenschnitt([0, 0], [1, 0], [0, 1], [1, 1]), null);
});
test("Reihe liefert Anzahl − 1 Versätze", () => {
  assert.deepEqual(reihe(4, 2, 0), [[2, 0], [4, 0], [6, 0]]);
  assert.deepEqual(reihe(1, 2, 0), []);
  assert.equal(reihe(1000, 1, 1).length, 199);
});
test("Fenster: links→rechts nur enthalten, rechts→links auch berührt", () => {
  const g: Plangeometrie = { art: "rechteck", von: [0, 0], bis: [10, 10] };
  const f = fenster([5, -5], [20, 20]);
  assert.equal(imFenster(f, punkteVon(g), kantenVon(g), false), false);
  assert.equal(imFenster(f, punkteVon(g), kantenVon(g), true), true);
  assert.equal(imFenster(fenster([-1, -1], [11, 11]), punkteVon(g), kantenVon(g), false), true);
  assert.equal(imFenster(fenster([2, 2], [3, 3]), punkteVon(g), kantenVon(g), true), true, "Fenster in der Fläche");
  const s: Plangeometrie = { art: "strecke", von: [0, 0], bis: [10, 0] };
  assert.equal(imFenster(fenster([4, -1], [6, 1]), punkteVon(s), kantenVon(s), true), true);
  assert.equal(imFenster(fenster([4, 1], [6, 3]), punkteVon(s), kantenVon(s), true), false);
});
