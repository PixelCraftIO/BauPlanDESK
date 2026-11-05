/**
 * Breite der Projektnavigation (Seitenleiste mit dem Projektbaum).
 *
 * Gilt für alle geöffneten Projekte gemeinsam und wird je Arbeitsplatz im
 * Browser-Speicher gemerkt. Die Leiste lässt sich am rechten Rand ziehen,
 * mit den Pfeiltasten verstellen und per Doppelklick zurücksetzen.
 */
export const NAVBREITE_STANDARD = 300;
export const NAVBREITE_MIN = 220;
export const NAVBREITE_MAX = 520;
const SCHLUESSEL = "bauplan.navbreite";

export function navbreiteBegrenzen(px: number): number {
  if (!Number.isFinite(px)) return NAVBREITE_STANDARD;
  return Math.round(Math.min(NAVBREITE_MAX, Math.max(NAVBREITE_MIN, px)));
}

function gemerkt(): number {
  try {
    const wert = Number(localStorage.getItem(SCHLUESSEL));
    return wert ? navbreiteBegrenzen(wert) : NAVBREITE_STANDARD;
  } catch {
    return NAVBREITE_STANDARD;
  }
}

export const navbreite = $state({ px: gemerkt() });

export function navbreiteSetzen(px: number, merken = true) {
  navbreite.px = navbreiteBegrenzen(px);
  if (!merken) return;
  try {
    localStorage.setItem(SCHLUESSEL, String(navbreite.px));
  } catch {
    /* Breite gilt dann nur bis zum Neustart */
  }
}
