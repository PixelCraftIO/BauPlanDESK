// Keep PDF previews and the IFC viewer offline: fonts, character maps and the
// web-ifc WebAssembly module ship with the app.
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
mkdirSync(`${root}public/web-ifc`, { recursive: true });
cpSync(`${root}node_modules/web-ifc/web-ifc.wasm`, `${root}public/web-ifc/web-ifc.wasm`);
