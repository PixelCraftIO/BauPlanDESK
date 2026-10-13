// Prüft alle installierten npm-Pakete auf erlaubte Lizenzen (kein GPL/AGPL/LGPL).
// Aufruf: npm run licenses
import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join } from "node:path";

const ALLOWED = new Set([
  "MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "0BSD",
  "CC0-1.0", "Unlicense", "BlueOak-1.0.0", "Python-2.0", "CC-BY-4.0", "MPL-2.0",
]);

function* packages(dir) {
  if (!existsSync(dir)) return;
  for (const name of readdirSync(dir)) {
    if (name.startsWith(".")) continue;
    const path = join(dir, name);
    if (name.startsWith("@")) {
      yield* packages(path);
      continue;
    }
    const manifest = join(path, "package.json");
    if (existsSync(manifest)) yield manifest;
    yield* packages(join(path, "node_modules"));
  }
}

// Ein SPDX-Ausdruck ist erlaubt, wenn bei OR mindestens eine, bei AND alle Lizenzen erlaubt sind.
function allowed(expr) {
  const clean = expr.replace(/[()]/g, "").trim();
  if (clean.includes(" OR ")) return clean.split(" OR ").some(allowed);
  if (clean.includes(" AND ")) return clean.split(" AND ").every(allowed);
  return ALLOWED.has(clean);
}

let bad = 0;
let count = 0;
for (const manifest of packages("node_modules")) {
  const pkg = JSON.parse(readFileSync(manifest, "utf8"));
  if (!pkg.name || !pkg.version) continue;
  count++;
  const raw = pkg.license ?? (pkg.licenses ?? []).map((l) => l.type ?? l).join(" OR ");
  const license = typeof raw === "string" ? raw : raw?.type ?? "";
  if (!license || !allowed(license)) {
    console.error(`NICHT ERLAUBT: ${pkg.name}@${pkg.version} – ${license || "keine Angabe"}`);
    bad++;
  }
}
console.log(`${count} Pakete geprüft, ${bad} Verstöße.`);
process.exit(bad ? 1 : 0);
