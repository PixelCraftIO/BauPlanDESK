import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import test from "node:test";
import ts from "typescript";

const source = readFileSync(new URL("../src/lib/kalender.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
});
const { heuteDeutsch } = await import(
  `data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`
);

test("Einreichungsfelder und Exportnamen verwenden deutsche Kalendertage", () => {
  for (const [zeitpunkt, datum] of [
    ["2026-01-15T22:59:59Z", "2026-01-15"],
    ["2026-01-15T23:00:00Z", "2026-01-16"],
    ["2026-07-15T21:59:59Z", "2026-07-15"],
    ["2026-07-15T22:00:00Z", "2026-07-16"],
    ["2026-03-29T01:00:00Z", "2026-03-29"],
    ["2026-10-25T01:00:00Z", "2026-10-25"],
  ]) {
    assert.equal(heuteDeutsch(new Date(zeitpunkt)), datum);
  }
});
