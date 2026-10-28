/**
 * Vergleichsform eines Eingabestands für die Änderungserkennung: Der Core
 * lässt leere Listen und fehlende Angaben beim Speichern weg, die Masken
 * belegen sie beim Anzeigen mit `[]` oder `null`. Beides ist inhaltlich
 * gleich und darf keine „ungespeicherte Änderung“ melden. Schlüssel werden
 * sortiert, damit nachträglich ergänzte Felder die Reihenfolge nicht ändern.
 */
export function vergleichsform(wert: unknown): string {
  return JSON.stringify(wert, (_schluessel, v: unknown) => {
    if (!v || typeof v !== "object" || Array.isArray(v)) return v;
    const sortiert: Record<string, unknown> = {};
    for (const k of Object.keys(v).sort()) {
      const x = (v as Record<string, unknown>)[k];
      if (x === null || x === undefined) continue;
      if (Array.isArray(x) && x.length === 0) continue;
      sortiert[k] = x;
    }
    return sortiert;
  });
}
