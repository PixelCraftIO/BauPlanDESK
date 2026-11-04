// Prüft, dass das Web-Handbuch alle Kapitel, Bilder und Verweise aus
// docs/handbuch/benutzerhandbuch/ auflösen kann. Aufruf: npm test
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { HANDBUCH_ORDNER, kapitel, kapitelListe, linkAdresse, startseite } from "../src/lib/handbuch.ts";

test("Links werden auf Web-Adressen umgeschrieben", () => {
  assert.equal(linkAdresse("README.md"), "/handbuch");
  assert.equal(linkAdresse("05-ermittlung.md#messwerte"), "/handbuch/05-ermittlung#messwerte");
  assert.equal(linkAdresse("Benutzerhandbuch.pdf"), "/handbuch/pdf");
  assert.equal(linkAdresse("#oben"), "#oben");
  assert.equal(linkAdresse("../../../scripts/handbuch-bilder/README.md"), null);
});

test("alle Kapitel rendern, Bilder existieren, Kapitelverweise treffen Überschriften", async () => {
  const liste = await kapitelListe();
  assert.ok(liste.length >= 10);
  const seiten = new Map([["", await startseite()]]);
  for (const k of liste) seiten.set(k.slug, (await kapitel(k.slug))!);

  for (const [slug, seite] of seiten) {
    for (const [, bild] of seite.html.matchAll(/<img src="\/handbuch\/bilder\/([^"]+)"/g)) {
      assert.ok(existsSync(path.join(HANDBUCH_ORDNER, "bilder", bild)), `${slug}: Bild ${bild} fehlt`);
    }
    for (const [, ziel, anker] of seite.html.matchAll(/href="\/handbuch\/?([a-z0-9-]*)(?:#([^"]+))?"/g)) {
      if (ziel === "pdf" || ziel === "bilder") continue;
      const zielSeite = seiten.get(ziel);
      assert.ok(zielSeite, `${slug}: Verweis auf unbekanntes Kapitel ${ziel}`);
      if (anker) {
        assert.ok(zielSeite.html.includes(`id="${decodeURIComponent(anker)}"`), `${slug}: Anker #${anker} fehlt in ${ziel || "README"}`);
      }
    }
  }
});

// Seiten außerhalb des Handbuchs: Bilder und Verweise ins Handbuch müssen
// auflösen, Ablaufdiagramme dürfen nur bekannte Kästen verbinden.
import { readdirSync, readFileSync } from "node:fs";
import { FUNKTIONEN } from "../src/lib/zeicheneditor.ts";
import * as ABLAEUFE from "../src/lib/ablaeufe.ts";

function seitenQuellen(ordner: string): string[] {
  return readdirSync(ordner, { withFileTypes: true }).flatMap((e) => {
    const p = path.join(ordner, e.name);
    if (e.isDirectory()) return e.name === "handbuch" ? [] : seitenQuellen(p);
    return e.name.endsWith(".tsx") || e.name.endsWith(".ts") ? [p] : [];
  });
}

test("Webseiten verweisen nur auf vorhandene Bilder und Handbuchanker", async () => {
  const liste = await kapitelListe();
  const html = new Map<string, string>([["", (await startseite()).html]]);
  for (const k of liste) html.set(k.slug, (await kapitel(k.slug))!.html);
  const pruefeLink = (wo: string, link: string) => {
    const m = link.match(/^\/handbuch\/?([a-z0-9-]*)(?:#(.+))?$/);
    if (!m || m[1] === "pdf") return;
    const ziel = html.get(m[1]);
    assert.ok(ziel !== undefined, `${wo}: unbekanntes Kapitel ${m[1]}`);
    if (m[2]) assert.ok(ziel.includes(`id="${m[2]}"`), `${wo}: Anker #${m[2]} fehlt in ${m[1]}`);
  };

  const bilder = new Set(FUNKTIONEN.map((f) => f.bild));
  for (const f of FUNKTIONEN) pruefeLink(`Zeicheneditor/${f.id}`, f.link);

  const quellen = [...seitenQuellen(path.join(import.meta.dirname, "../src/app")), path.join(import.meta.dirname, "../src/lib/zeicheneditor.ts")];
  for (const datei of quellen) {
    const text = readFileSync(datei, "utf8");
    for (const [, b] of text.matchAll(/(?:bild: |\/handbuch\/bilder\/)"?([a-z0-9-]+)(?:"|\.png)/g)) bilder.add(b);
    for (const [, link] of text.matchAll(/(?:href|link)[=:] ?"(\/handbuch[^"]*)"/g)) pruefeLink(path.basename(datei), link);
  }
  for (const b of bilder) assert.ok(existsSync(path.join(HANDBUCH_ORDNER, "bilder", `${b}.png`)), `Bild ${b}.png fehlt`);
});

test("Ablaufdiagramme verbinden nur vorhandene Kästen", () => {
  for (const [name, d] of Object.entries(ABLAEUFE)) {
    const ids = new Set(d.knoten.map((k) => k.id));
    assert.equal(ids.size, d.knoten.length, `${name}: doppelte Kasten-ID`);
    for (const k of d.kanten) {
      assert.ok(ids.has(k.von) && ids.has(k.nach), `${name}: ${k.von} → ${k.nach}`);
    }
    for (const k of d.knoten) {
      assert.ok(k.x >= 0 && k.y >= 0 && k.x + (k.w ?? 190) <= d.breite && k.y + (k.h ?? 64) <= d.hoehe, `${name}: ${k.id} liegt außerhalb`);
    }
  }
});
