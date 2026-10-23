import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { quellenstand } from "../src/lib/berechnungen/quellenstand.ts";

const projekt = JSON.parse(
  readFileSync(new URL("./fixtures/projekt.json", import.meta.url)),
);
const plan = {
  gebaeude_id: "haus",
  grundstueck_id: null,
  fachquellen: [],
  uebernahmen: [],
  planbindungen: [],
};
const gebaeude = [
  { objekt: "haus", revision: 1 },
  { objekt: "anderes", revision: 1 },
];
const stand = (
  p = projekt,
  e = plan,
  g = gebaeude,
  fehler = [],
  generation = 1,
) => quellenstand(p, e, g, [], [], [], generation, fehler);
test("Projekttitel und technische Projektversion verändern keine Fachvorschau", () => {
  assert.equal(
    stand(),
    stand({ ...projekt, name: "Neuer Titel", version: projekt.version + 1 }),
  );
});
test("Nur ausgewählte Gebäude invalidieren die Vorschau", () => {
  assert.equal(
    stand(),
    stand(projekt, plan, [gebaeude[0], { objekt: "anderes", revision: 2 }]),
  );
  assert.notEqual(
    stand(),
    stand(projekt, plan, [{ objekt: "haus", revision: 2 }, gebaeude[1]]),
  );
});
test("Rechtsänderung und fehlgeschlagene Quellenprüfung bleiben relevant", () => {
  assert.notEqual(
    stand(),
    stand({
      ...projekt,
      planungsrecht: { ...projekt.planungsrecht, bau_nvo: "1962" },
    }),
  );
  assert.notEqual(
    stand(),
    stand(projekt, plan, gebaeude, ["Quelle nicht verfügbar"]),
  );
  assert.notEqual(stand(), stand(projekt, plan, []));
});
test("Dokumentereignisse ohne Planbindung sind keine neue Geometrie", () => {
  assert.equal(stand(), stand(projekt, plan, gebaeude, [], 3));
  const gebunden = { ...plan, planbindungen: [{ markierung_id: "raum" }] };
  assert.notEqual(
    stand(projekt, gebunden),
    stand(projekt, gebunden, gebaeude, [], 3),
  );
});
test("Eine referenzierte Raumbuchrevision wird auch ohne Gebäude verfolgt", () => {
  const p = {
    ...plan,
    gebaeude_id: null,
    fachquellen: [{ art: "raumbuch", objekt: "r", revision: 1 }],
  };
  const s = (revision) =>
    quellenstand(projekt, p, [], [{ objekt: "r", revision }], [], [], 0, []);
  assert.notEqual(s(1), s(2));
});
