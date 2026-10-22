import test from "node:test";
import assert from "node:assert/strict";
import {
  punkteFuer,
  nachweisZiel,
} from "../src/lib/arbeitsplatz/arbeitscheck.ts";

test("Verfahrensfilter erhält projektweite Prüfungen und trennt andere Akten", () => {
  const p = (id, thema, bereich, objekt) => ({
    id,
    thema,
    ziel: { bereich, objekt },
  });
  const punkte = [
    p("a", "verfahren", "verfahren", "A"),
    p("b", "verfahren", "verfahren", "B"),
    p("c", "verfahren", "verfahren", null),
    p("d", "nachweise", "nachweise", "N"),
  ];
  assert.deepEqual(
    punkteFuer(punkte, "verfahren", "A").map((p) => p.id),
    ["a", "c"],
  );
  assert.deepEqual(
    punkteFuer(punkte, "nachweise", "A").map((p) => p.id),
    ["d"],
  );
  assert.equal(punkteFuer(punkte, "verfahren").length, 3);
});
test("Nachweissprünge verwenden die Identität des Produzenten", () => {
  assert.deepEqual(
    nachweisZiel({
      art: "fach",
      objekt: "wohn-2",
      plan: { modul: "wohnflaechen" },
    }),
    {
      bereich: "wohnflaechen",
      objekt: "wohn-2",
      register: "daten",
      eintrag: null,
    },
  );
  assert.equal(
    nachweisZiel({ art: "abstand_projekt", objekt: "wand-2" }).objekt,
    "wand-2",
  );
  for (const [fach, bereich] of Object.entries({
    grz: "grz",
    gfz: "gfz",
    geschossigkeit: "geschosse",
    bmz: "volumen",
  }))
    assert.equal(nachweisZiel({ art: "nutzung", fach }).bereich, bereich);
  assert.equal(nachweisZiel({ art: "abstand" }).bereich, "abstand");
});
