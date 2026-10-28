import test from "node:test";
import assert from "node:assert/strict";
import { vergleichsform } from "../src/lib/vergleichsform.ts";

test("Fehlende Listen und Angaben gelten als unverändert", () => {
  const gespeichert = { id: "a", bescheide: [{ id: "b", auflagen: [] }] };
  const angezeigt = {
    bescheide: [{ auflagen: [], id: "b", ersetzt: null }],
    id: "a",
    abweichungen: [],
    gebuehren: [],
    bautechnische_nachweise: [],
  };
  assert.equal(vergleichsform(angezeigt), vergleichsform(gespeichert));
});
test("Echte Änderungen bleiben sichtbar", () => {
  const alt = { liste: ["x"], name: "A", menge: 0, aktiv: false };
  assert.notEqual(vergleichsform({ ...alt, liste: [] }), vergleichsform(alt));
  assert.notEqual(vergleichsform({ ...alt, name: "" }), vergleichsform(alt));
  assert.notEqual(vergleichsform({ ...alt, menge: null }), vergleichsform(alt));
  assert.notEqual(vergleichsform({ ...alt, aktiv: true }), vergleichsform(alt));
  assert.notEqual(vergleichsform(["a", "b"]), vergleichsform(["b", "a"]));
});
