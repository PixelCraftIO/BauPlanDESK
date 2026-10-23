import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { handbuch, sucheKapitel, kapitelFuerBereich } from "../src/lib/hilfe/handbuch.ts";
import { handbuchMarkdown, handbuchHtml } from "../src/lib/hilfe/export.ts";

import { handbuchBildData } from "../scripts/handbuch-bilder.mjs";

test("Alle Kapitel, Querverweise und Tabellen sind vollständig und eindeutig", () => {
  const ids = handbuch.kapitel.map(k => k.id);
  assert.equal(new Set(ids).size, ids.length);
  for (const k of handbuch.kapitel) {
    assert.match(k.id, /^[a-z][a-z0-9-]*$/);
    assert.ok(k.titel && k.kurztext && k.gruppe && k.abschnitte.length, k.id);
    for (const id of k.siehe) assert.ok(ids.includes(id), `${k.id} → ${id}`);
    for (const a of k.abschnitte) {
      assert.ok(a.text?.length || a.schritte?.length || a.liste?.length || a.tabelle || a.hinweis, k.id);
      if (a.tabelle) for (const r of a.tabelle.zeilen) assert.equal(r.length, a.tabelle.spalten.length, k.id);
    }
  }
});
test("Alle registrierten Projektbereiche besitzen eine kontextbezogene Anleitung", () => {
  // Register enthält Svelte-Komponenten; hier nur die tatsächlich registrierten IDs lesen.
  const ids = ["../src/lib/module.ts", "../src/lib/fachmodule/register.ts"].flatMap(p =>
    [...readFileSync(new URL(p, import.meta.url), "utf8").matchAll(/\bid:\s*["']([a-z0-9_]+)["']/g)].map(m => m[1]));
  assert.ok(ids.length > 30);
  for (const id of ids) assert.notEqual(kapitelFuerBereich(id), "start", id);
  assert.equal(kapitelFuerBereich("unbekannt"), "start");
});
test("Suche findet Arbeitsabläufe auch in Schritten, Hinweisen und Tabellen", () => {
  assert.ok(sucheKapitel("Nachforderung Nachreichung").some(k => k.id === "nachforderungen"));
  assert.ok(sucheKapitel("Gemeinschaftsflächen").some(k => k.id === "baugrundstueck"));
  assert.ok(sucheKapitel("grun").some(k => k.id === "freiflaechen"));
  assert.ok(sucheKapitel("Versionskonflikt").some(k => k.id === "stoerungen"));
  assert.deepEqual(sucheKapitel("xyz-keine-treffer-123"), []);
});
test("Offline-Ausgaben enthalten alle Kapitel und sichere interne Links", () => {
  const html = handbuchHtml(handbuchBildData), md = handbuchMarkdown();
  assert.ok(html.startsWith('<!doctype html><html lang="de">'));
  for (const k of handbuch.kapitel) {
    assert.ok(html.includes(`id="${k.id}"`), k.id);
    assert.ok(md.includes(k.titel), k.id);
  }
  for (const [, id] of html.matchAll(/href="#([^"]+)"/g)) assert.ok(html.includes(`id="${id}"`), id);
  assert.ok(!/<script|<iframe|<Platzhalter1>/i.test(html));
  assert.ok(html.includes("&lt;Platzhalter1&gt;"));
  assert.ok(!/src="https?:/i.test(html));
  assert.equal(readFileSync(new URL("../../../docs/handbuch/Bedienhandbuch.md", import.meta.url), "utf8"), md);
  assert.equal(readFileSync(new URL("../../../docs/handbuch/Bedienhandbuch.html", import.meta.url), "utf8"), html);
});
test("Die mitgelieferte PDF gehört zum aktuellen Handbuchstand", () => {
  const root = new URL("../../../docs/handbuch/", import.meta.url);
  const stand = JSON.parse(readFileSync(new URL("pdf-stand.json", root), "utf8"));
  const pdf = readFileSync(new URL("Bedienhandbuch.pdf", root));
  assert.equal(pdf.subarray(0, 5).toString(), "%PDF-");
  assert.equal(stand.stand, handbuch.stand);
  assert.equal(stand.pdf_sha256, createHash("sha256").update(pdf).digest("hex"));
  assert.equal(stand.html_sha256, createHash("sha256").update(handbuchHtml(handbuchBildData)).digest("hex"), "PDF nach Inhaltsänderung neu erzeugen");
});

test("Abbildungen sind vollständig mitgeliefert, beschriftet und offline eingebettet", () => {
  const bilder = handbuch.kapitel.flatMap(k => k.abschnitte.flatMap(a => a.abbildung ? [a.abbildung] : []));
  assert.ok(bilder.length >= 30);
  const manifest = JSON.parse(readFileSync(new URL("../public/handbuch/aufnahmen.json", import.meta.url), "utf8"));
  assert.equal(manifest.aufnahmen.length, bilder.length);
  const output = handbuchHtml(handbuchBildData);
  assert.equal([...output.matchAll(/<figure>/g)].length, bilder.length);
  for (const bild of bilder) {
    assert.match(bild.datei, /^[a-z0-9-]+\.png$/);
    assert.ok(bild.alt.length > 15 && bild.beschreibung.length > 60, bild.datei);
    const daten = readFileSync(new URL(`../public/handbuch/${bild.datei}`, import.meta.url));
    assert.equal(daten.subarray(1,4).toString(), "PNG");
    const eintrag = manifest.aufnahmen.find(b => b.datei === bild.datei);
    assert.equal(eintrag.sha256, createHash("sha256").update(daten).digest("hex"));
    assert.ok(output.includes(handbuchBildData(bild.datei)), bild.datei);
  }
  assert.throws(() => handbuchBildData("../secret.png"));
  assert.equal([...output.matchAll(/src="data:image\/png;base64,/g)].length, bilder.length);
});
