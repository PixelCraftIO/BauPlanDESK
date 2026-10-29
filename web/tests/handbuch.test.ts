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
