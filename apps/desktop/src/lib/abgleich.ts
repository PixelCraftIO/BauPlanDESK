/**
 * Abgleich eigener Eingaben mit der aktuellen Fassung nach einem
 * Versionskonflikt (Optimistic Locking). Reiner Oberflächenzustand: Es wird
 * nichts gespeichert, der Core prüft beim erneuten Speichern wieder die Version.
 *
 * Felder, die nur am anderen Arbeitsplatz geändert wurden, übernehmen die
 * aktuelle Fassung. Eigene Änderungen bleiben stehen. Haben beide dasselbe Feld
 * unterschiedlich geändert, bleibt die eigene Eingabe stehen und das Feld wird
 * als Abweichung gemeldet, damit niemand fremde Änderungen unbemerkt
 * überschreibt.
 */
export interface Abweichung {
  /** Punktgetrennter Pfad, z. B. `anschrift.ort`. */
  pfad: string;
  /** Wert der aktuellen Fassung. */
  aktuell: unknown;
}

const gleich = (a: unknown, b: unknown) =>
  JSON.stringify(a ?? null) === JSON.stringify(b ?? null);
const istObjekt = (v: unknown): v is Record<string, unknown> =>
  !!v && typeof v === "object" && !Array.isArray(v);

export function abgleichen<T>(
  basis: T,
  eigene: T,
  aktuell: T,
): { werte: T; abweichungen: Abweichung[] } {
  const abweichungen: Abweichung[] = [];
  function zweig(b: unknown, e: unknown, a: unknown, pfad: string): unknown {
    if (istObjekt(b) && istObjekt(e) && istObjekt(a)) {
      const neu: Record<string, unknown> = {};
      for (const k of new Set([...Object.keys(a), ...Object.keys(e)]))
        neu[k] = zweig(b[k], e[k], a[k], pfad ? `${pfad}.${k}` : k);
      return neu;
    }
    if (gleich(e, b)) return structuredClone(a);
    if (!gleich(a, b) && !gleich(a, e))
      abweichungen.push({ pfad, aktuell: structuredClone(a) });
    return e;
  }
  const werte = zweig(basis, eigene, aktuell, "") as T;
  return { werte, abweichungen };
}

/** Setzt den Wert eines Pfads aus `abgleichen` (z. B. „aktuellen Wert übernehmen“). */
export function pfadSetzen(ziel: unknown, pfad: string, wert: unknown) {
  const teile = pfad.split(".");
  let o = ziel as Record<string, unknown>;
  for (const t of teile.slice(0, -1)) o = o[t] as Record<string, unknown>;
  o[teile.at(-1)!] = structuredClone(wert);
}

/** Kurze Anzeige eines Werts der aktuellen Fassung. */
export function wertText(v: unknown): string {
  if (v === null || v === undefined || v === "") return "leer";
  if (typeof v === "string") return `„${v}“`;
  if (typeof v === "number") return v.toLocaleString("de-DE");
  if (typeof v === "boolean") return v ? "ja" : "nein";
  return JSON.stringify(v);
}
