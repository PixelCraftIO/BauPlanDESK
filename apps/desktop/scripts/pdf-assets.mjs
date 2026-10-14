// Keep PDF previews offline: all fonts and character maps ship with the app.
import { cpSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
const root = fileURLToPath(new URL("../", import.meta.url));
for (const dir of ["cmaps", "standard_fonts"]) {
  mkdirSync(`${root}public/pdfjs/${dir}`, { recursive: true });
  cpSync(
    `${root}node_modules/pdfjs-dist/${dir}`,
    `${root}public/pdfjs/${dir}`,
    { recursive: true },
  );
}
