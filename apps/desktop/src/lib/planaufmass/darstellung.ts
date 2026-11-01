/**
 * Darstellung im Zeicheneditor: Stift, Linienart, Farbe, Schraffur. Element
 * überschreibt Ebene, Ebene überschreibt Standard. Nur Anzeige und Planausgabe.
 */
import type { Darstellung, Linienart, Schraffur } from "../generated/plattform";

/** Stiftstärken nach DIN ISO 128 (mm). */
export const STIFTE = [0.13, 0.18, 0.25, 0.35, 0.5, 0.7, 1.0, 1.4, 2.0];
export const LINIENARTEN: [Linienart, string][] = [["volllinie", "Volllinie"], ["strichlinie", "Strichlinie"], ["strichpunktlinie", "Strichpunktlinie"], ["punktlinie", "Punktlinie"]];
export const SCHRAFFUREN: [Schraffur, string][] = [["keine", "Keine"], ["voll", "Vollfläche"], ["diagonal", "Diagonal"], ["kreuz", "Kreuz"], ["mauerwerk", "Mauerwerk"], ["beton", "Beton"], ["daemmung", "Dämmung"], ["erdreich", "Erdreich"]];

export type Wirksam = { stift_mm: number | null; linienart: Linienart | null; farbe: string | null; schraffur: Schraffur | null };

/** Fasst Element- und Ebenendarstellung zusammen; `null` = Standard des Editors. */
export function wirksam(element?: Darstellung | null, ebene?: Darstellung | null): Wirksam {
  return {
    stift_mm: element?.stift_mm ?? ebene?.stift_mm ?? null,
    linienart: element?.linienart ?? ebene?.linienart ?? null,
    farbe: element?.farbe ?? ebene?.farbe ?? null,
    schraffur: element?.schraffur ?? ebene?.schraffur ?? null,
  };
}
export const leer = (d?: Darstellung | null) => !d || (d.stift_mm == null && d.linienart == null && d.farbe == null && d.schraffur == null);

/** Strichmuster relativ zur Strichbreite (Bildschirm-Pixel, da die Linien nicht mitskalieren). */
export function strichmuster(art: Linienart | null, breite: number): string | null {
  const b = Math.max(1, breite);
  if (art === "strichlinie") return `${6 * b} ${3 * b}`;
  if (art === "strichpunktlinie") return `${9 * b} ${2.5 * b} ${1 * b} ${2.5 * b}`;
  if (art === "punktlinie") return `${1 * b} ${2.5 * b}`;
  return null;
}
/** Strichbreite am Bildschirm: 1 mm Stift ≈ 3,8 px, mindestens 0,7 px. */
export const strichPx = (stift: number | null) => (stift === null ? null : Math.max(0.7, stift * 3.8));

/** Id des SVG-Musters einer Schraffur in einer Farbe. */
export const musterId = (praefix: string, s: Schraffur, farbe: string) => `${praefix}-${s}-${farbe.replace("#", "")}`;

/** Inline-Stil für Linien und Flächen; leer, wenn alles Standard ist. */
export function stil(w: Wirksam, praefix: string, flaeche = true): string {
  const teile: string[] = [];
  const px = strichPx(w.stift_mm);
  if (w.farbe) teile.push(`stroke:${w.farbe}`);
  if (px !== null) teile.push(`stroke-width:${px}px`);
  const muster = strichmuster(w.linienart, px ?? 1.8);
  if (muster) teile.push(`stroke-dasharray:${muster}`);
  if (flaeche && w.schraffur) {
    const f = w.farbe ?? "#2b3138";
    teile.push(w.schraffur === "keine" ? "fill:none" : w.schraffur === "voll" ? `fill:${f};fill-opacity:.35` : `fill:url(#${musterId(praefix, w.schraffur, f)})`);
  }
  return teile.join(";");
}

/** SVG-Inhalt eines Schraffurmusters (Kachel 8×8 Einheiten). */
export function musterInhalt(s: Schraffur, farbe: string): string {
  const l = (d: string, extra = "") => `<path d="${d}" stroke="${farbe}" stroke-width=".6" fill="none"${extra}/>`;
  switch (s) {
    case "diagonal": return l("M-2 2L2 -2M0 8L8 0M6 10L10 6");
    case "kreuz": return l("M0 8L8 0M0 0L8 8");
    case "mauerwerk": return l("M0 8L8 0M-2 2L2 -2M6 10L10 6M0 4L4 0M4 8L8 4");
    case "beton": return `<circle cx="2" cy="2" r=".7" fill="${farbe}"/><circle cx="6" cy="5" r="1" fill="none" stroke="${farbe}" stroke-width=".4"/><path d="M3 6l1.2 1" stroke="${farbe}" stroke-width=".4"/>`;
    case "daemmung": return l("M0 4C1 0 3 0 4 4S7 8 8 4");
    case "erdreich": return l("M0 8L8 0M0 6h3M5 2h3") ;
    default: return "";
  }
}
