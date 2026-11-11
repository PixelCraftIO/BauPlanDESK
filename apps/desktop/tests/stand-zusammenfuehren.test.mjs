import test from "node:test";
import assert from "node:assert/strict";
import { standZusammenfuehren } from "../src/lib/standZusammenfuehren.ts";

test("fremde Änderung wird übernommen, eigene Eingabe bleibt", () => {
  const basis = { bezeichnung: "Wohnhaus", grundflaeche: null, dachneigung: 36 };
  const eigen = { bezeichnung: "Wohnhaus", grundflaeche: null, dachneigung: 38 };
  const neu = { bezeichnung: "Wohnhaus", grundflaeche: 99, dachneigung: 36 };
  const r = standZusammenfuehren(basis, eigen, neu);
  assert.deepEqual(r.daten, { bezeichnung: "Wohnhaus", grundflaeche: 99, dachneigung: 38 });
  assert.deepEqual(r.konflikte, []);
});

test("dasselbe Feld an beiden Stellen geändert: eigene Eingabe gewinnt und wird gemeldet", () => {
  const r = standZusammenfuehren({ a: 1, b: 1 }, { a: 2, b: 1 }, { a: 3, b: 5 });
  assert.deepEqual(r.daten, { a: 2, b: 5 });
  assert.deepEqual(r.konflikte, ["a"]);
});

test("Listen mit gleichen Elementen werden je Element zusammengeführt", () => {
  const basis = { geschosse: [{ id: "kg", flaeche: null, hoehe: 2.4 }, { id: "eg", flaeche: null, hoehe: 2.6 }] };
  const eigen = { geschosse: [{ id: "kg", flaeche: null, hoehe: 2.5 }, { id: "eg", flaeche: null, hoehe: 2.6 }] };
  const neu = { geschosse: [{ id: "kg", flaeche: 99, hoehe: 2.4 }, { id: "eg", flaeche: 99, hoehe: 2.6 }] };
  const r = standZusammenfuehren(basis, eigen, neu);
  assert.deepEqual(r.daten.geschosse, [{ id: "kg", flaeche: 99, hoehe: 2.5 }, { id: "eg", flaeche: 99, hoehe: 2.6 }]);
  assert.deepEqual(r.konflikte, []);
});

test("Listen mit anderer Zusammensetzung gelten als ein Feld", () => {
  const basis = { geschosse: [{ id: "eg" }] };
  const eigen = { geschosse: [{ id: "eg" }, { id: "dg" }] };
  const neu = { geschosse: [{ id: "eg", flaeche: 99 }] };
  const r = standZusammenfuehren(basis, eigen, neu);
  assert.deepEqual(r.daten, eigen);
  assert.deepEqual(r.konflikte, ["geschosse"]);
});

test("neuer Entwurf ohne Basis behält die eigenen Eingaben", () => {
  const r = standZusammenfuehren(null, { a: 1 }, { a: 2 });
  assert.deepEqual(r.daten, { a: 1 });
});
