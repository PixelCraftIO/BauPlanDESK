// Prüfung der Planausrichtung über Passpunkte: npm run test:geometrie
import { test } from "node:test";
import assert from "node:assert/strict";
import { aufAltesBlatt, aufNeuesBlatt, ausPasspunkten } from "../src/lib/planaufmass/ausrichtung.ts";

const nah = (a: number[], b: number[]) => a.forEach((v, i) => assert.ok(Math.abs(v - b[i]) < 1e-6, `${a} ≠ ${b}`));

test("Hin- und Rückabbildung heben sich auf", () => {
  const t = { dx: 12, dy: -7, grad: 30, faktor: 1.25 };
  nah(aufNeuesBlatt(t)(aufAltesBlatt(t)([40, 55])), [40, 55]);
});

test("Zwei Passpunkte bestimmen Versatz, Drehung und Skalierung", () => {
  // Der neue Planstand ist um 90° gedreht, halb so groß gezeichnet und versetzt.
  const wahr = { dx: 100, dy: 20, grad: 90, faktor: 2 };
  const q1: [number, number] = [10, 10], q2: [number, number] = [60, 10];
  const alt: [[number, number], [number, number]] = [aufAltesBlatt(wahr)(q1), aufAltesBlatt(wahr)(q2)];
  // Die Vergleichsunterlage liegt bisher mit einer anderen Ausrichtung auf dem Blatt.
  const bisher = { dx: 5, dy: 5, grad: 0, faktor: 1 };
  const t = ausPasspunkten(alt, [aufAltesBlatt(bisher)(q1), aufAltesBlatt(bisher)(q2)], bisher)!;
  nah([t.dx, t.dy, t.grad, t.faktor], [100, 20, 90, 2]);
  nah(aufNeuesBlatt(t)(alt[1]), q2);
});

test("Aufeinanderliegende Passpunkte ergeben keine Ausrichtung", () => {
  assert.equal(ausPasspunkten([[1, 1], [1, 1]], [[0, 0], [5, 5]], { dx: 0, dy: 0, grad: 0, faktor: 1 }), null);
});
