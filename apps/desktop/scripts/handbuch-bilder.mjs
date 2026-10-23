import { readFileSync } from "node:fs";

/** Nur mitgelieferte Bilder; eingebettet bleibt die HTML-Ausgabe vollständig offline. */
export function handbuchBildData(datei) {
  if (!/^[a-z0-9-]+\.png$/.test(datei)) throw new Error(`Ungültige Handbuchabbildung: ${datei}`);
  const bild = readFileSync(new URL(`../public/handbuch/${datei}`, import.meta.url));
  return `data:image/png;base64,${bild.toString("base64")}`;
}
