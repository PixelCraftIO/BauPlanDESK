/**
 * Geänderte Bereiche zwischen dem aktuellen Blatt und einer ausgerichteten
 * Vergleichsunterlage, in Seitenpunkten des aktuellen Blatts. Nur im Browser.
 */
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { differenz, bereiche } from "./planvergleich";
import type { Planausrichtung } from "./ausrichtung";

export type Bereich = [number, number, number, number];
interface Blatt { base64: string; seite: number }

async function rendern(b: Blatt, skala: number | null) {
  const { getDocument, GlobalWorkerOptions } = await import("pdfjs-dist");
  GlobalWorkerOptions.workerSrc = workerUrl;
  const pdf = await getDocument({ data: Uint8Array.from(atob(b.base64), c => c.charCodeAt(0)), isEvalSupported: false }).promise;
  try {
    const seite = await pdf.getPage(Math.min(b.seite, pdf.numPages)), v1 = seite.getViewport({ scale: 1 });
    // Etwa 2000 px: genug für Strichänderungen, ohne den Speicher zu sprengen.
    const s = skala ?? Math.min(3, 2000 / Math.max(v1.width, v1.height)), vp = seite.getViewport({ scale: s });
    const c = document.createElement("canvas");
    c.width = Math.round(vp.width); c.height = Math.round(vp.height);
    const ctx = c.getContext("2d", { willReadFrequently: true })!;
    ctx.fillStyle = "white"; ctx.fillRect(0, 0, c.width, c.height);
    await seite.render({ canvasContext: ctx, viewport: vp }).promise;
    return { c, ctx, skala: s };
  } finally { void pdf.destroy(); }
}

export async function planstandDifferenz(aktuell: Blatt, vergleich: Blatt, t: Planausrichtung): Promise<Bereich[]> {
  const a = await rendern(aktuell, null), b = await rendern(vergleich, a.skala);
  // Vergleichsblatt mit seiner Ausrichtung auf die Fläche des aktuellen Blatts legen.
  const c = document.createElement("canvas");
  c.width = a.c.width; c.height = a.c.height;
  const ctx = c.getContext("2d", { willReadFrequently: true })!;
  ctx.fillStyle = "white"; ctx.fillRect(0, 0, c.width, c.height);
  ctx.translate(t.dx * a.skala, t.dy * a.skala); ctx.rotate(t.grad * Math.PI / 180); ctx.scale(t.faktor, t.faktor);
  ctx.drawImage(b.c, 0, 0);
  const { maske } = differenz(a.ctx.getImageData(0, 0, c.width, c.height).data, ctx.getImageData(0, 0, c.width, c.height).data, c.width, c.height, 160, 2);
  return bereiche(maske, c.width, c.height, Math.max(8, Math.round(c.width / 150)), 2, 12)
    .map(([x, y, w, h]) => [(x - 6) / a.skala, (y - 6) / a.skala, (w + 12) / a.skala, (h + 12) / a.skala]);
}

/** Liegt einer der Punkte (oder ihre Umgrenzung) in einem der Bereiche? */
export function imBereich(punkte: [number, number][], bs: Bereich[]): boolean {
  if (!punkte.length) return false;
  const x0 = Math.min(...punkte.map(p => p[0])), x1 = Math.max(...punkte.map(p => p[0])), y0 = Math.min(...punkte.map(p => p[1])), y1 = Math.max(...punkte.map(p => p[1]));
  return bs.some(([x, y, w, h]) => x0 <= x + w && x1 >= x && y0 <= y + h && y1 >= y);
}
