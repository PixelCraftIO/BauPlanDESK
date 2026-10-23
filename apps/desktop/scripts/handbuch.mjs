import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { handbuchMarkdown, handbuchHtml } from "../src/lib/hilfe/export.ts";

import { handbuchBildData } from "./handbuch-bilder.mjs";

const ziel = new URL("../../../docs/handbuch/", import.meta.url);
const pruefen = process.argv.includes("--check");
if (!pruefen) mkdirSync(ziel, { recursive: true });
let fehler = false;
for (const [name, text] of [["Bedienhandbuch.md", handbuchMarkdown()], ["Bedienhandbuch.html", handbuchHtml(handbuchBildData)]]) {
  const datei = new URL(name, ziel);
  if (pruefen) {
    let vorhanden;
    try { vorhanden = readFileSync(datei, "utf8"); } catch { /* fehlende Ausgabe melden */ }
    if (vorhanden !== text) { console.error(`${name} ist nicht aktuell. npm run handbuch:export ausführen.`); fehler = true; }
  } else { writeFileSync(datei, text); console.log(`${name} erstellt.`); }
}
if (fehler) process.exitCode = 1;
else if (pruefen) console.log("Handbuch-Dateien stimmen mit der integrierten Hilfe überein.");
