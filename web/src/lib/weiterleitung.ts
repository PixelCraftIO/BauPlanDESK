// Ziel nach der Anmeldung: nur Pfade dieser Seite, kein offener Redirect.
// Browser lösen „/\evil.com“ oder „/<Tab>/evil.com“ wie „//evil.com“ auf;
// daher wird das Ziel gegen einen Platzhalter-Ursprung aufgelöst und nur
// akzeptiert, wenn es dort bleibt.
const BASIS = "https://bauplanio.invalid";

export function sicheresZiel(weiter: string | string[] | undefined): string {
  if (typeof weiter !== "string" || !weiter.startsWith("/")) return "/";
  // Steuerzeichen und Backslashes haben in einem internen Pfad nichts zu suchen.
  if (/[\\\u0000-\u001f\u007f]/.test(weiter)) return "/";
  let url: URL;
  try {
    url = new URL(weiter, BASIS);
  } catch {
    return "/";
  }
  if (url.origin !== BASIS) return "/";
  return url.pathname + url.search + url.hash;
}
