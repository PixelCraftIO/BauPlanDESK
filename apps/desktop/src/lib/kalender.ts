/** Fachliche Kalendertage gelten unabhängig von der Rechnerzeitzone in Deutschland. */
export function heuteDeutsch(zeitpunkt = new Date()): string {
  return new Intl.DateTimeFormat("sv-SE", {
    timeZone: "Europe/Berlin",
    year: "numeric", month: "2-digit", day: "2-digit",
  }).format(zeitpunkt);
}
