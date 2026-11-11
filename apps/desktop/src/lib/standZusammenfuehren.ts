/**
 * Führt einen ungespeicherten Entwurf mit einem inzwischen neu gespeicherten
 * Stand zusammen (Dreiwegevergleich gegen den Stand, auf dem der Entwurf
 * beruht). Felder, die nur im gespeicherten Stand geändert wurden, werden
 * übernommen; eigene Eingaben bleiben. Wurde dasselbe Feld an beiden Stellen
 * unterschiedlich geändert, gewinnt die eigene Eingabe und der Pfad wird als
 * Konflikt gemeldet. Ohne Laufzeit-Importe (node-Test).
 */
const gleich = (a: unknown, b: unknown) => JSON.stringify(a ?? null) === JSON.stringify(b ?? null);
const objekt = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const kennung = (v: unknown) => (objekt(v) && (typeof v.id === "string" || typeof v.id === "number") ? String(v.id) : null);
/** Listen lassen sich elementweise zusammenführen, wenn alle drei dieselben Elemente in derselben Reihenfolge führen. */
function gleicheElemente(a: unknown[], b: unknown[], c: unknown[]) {
  if (a.length !== b.length || a.length !== c.length) return false;
  return a.every((x, i) => {
    const k = kennung(x);
    return k !== null && k === kennung(b[i]) && k === kennung(c[i]);
  });
}
function mischen(basis: unknown, eigen: unknown, neu: unknown, pfad: string, konflikte: string[]): unknown {
  if (gleich(eigen, basis)) return neu;
  if (gleich(neu, basis) || gleich(eigen, neu)) return eigen;
  if (objekt(basis) && objekt(eigen) && objekt(neu)) {
    const aus: Record<string, unknown> = {};
    for (const k of new Set([...Object.keys(eigen), ...Object.keys(neu)])) {
      const wert = mischen(basis[k], eigen[k], neu[k], pfad ? `${pfad}.${k}` : k, konflikte);
      if (wert !== undefined) aus[k] = wert;
    }
    return aus;
  }
  if (Array.isArray(basis) && Array.isArray(eigen) && Array.isArray(neu) && gleicheElemente(basis, eigen, neu))
    return eigen.map((e, i) => mischen(basis[i], e, neu[i], `${pfad}[${i}]`, konflikte));
  konflikte.push(pfad || "(gesamt)");
  return eigen;
}
export function standZusammenfuehren<T>(basis: T | null, eigen: T, neu: T): { daten: T; konflikte: string[] } {
  const konflikte: string[] = [];
  const daten = mischen(basis, eigen, neu, "", konflikte) as T;
  return { daten: JSON.parse(JSON.stringify(daten)) as T, konflikte };
}
