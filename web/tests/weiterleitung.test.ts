// Weiterleitung nach der Anmeldung bleibt auf der eigenen Seite.
import assert from "node:assert/strict";
import { test } from "node:test";
import { sicheresZiel } from "../src/lib/weiterleitung.ts";

test("interne Ziele bleiben erhalten", () => {
  assert.equal(sicheresZiel("/handbuch/05-ermittlung?x=1#messwerte"), "/handbuch/05-ermittlung?x=1#messwerte");
  assert.equal(sicheresZiel("/"), "/");
});

test("fremde Ziele führen zur Startseite", () => {
  for (const z of ["//evil.com", "/\\evil.com", "/\t/evil.com", "/\n/evil.com", "https://evil.com", "evil.com", "", undefined, ["/a"]]) {
    assert.equal(sicheresZiel(z as string), "/", String(z));
  }
});
