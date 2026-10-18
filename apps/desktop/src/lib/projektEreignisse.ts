/**
 * Projektweite Hinweise zwischen den Bereichen einer Projektakte.
 *
 * Bereiche bleiben ausgeblendet geladen (Projektakte.svelte). Ändert ein Bereich
 * Dokumente, Fachobjekte oder Nachweise, laden die anderen Bereiche ihre
 * Auswahllisten derselben Akte neu. Das Ereignis trägt nur die Projekt-ID und
 * den Datenbereich; maßgebend bleibt der neu geladene Stand aus dem Core.
 */
export type Datenbereich = "dokumente" | "fachobjekte" | "nachweise";
export interface Projektereignis {
  projektId: string;
  bereich: Datenbereich;
  /** Auslösender Bereich, damit er seine eigene Meldung überspringen kann. */
  quelle?: string;
}

const ziel = new EventTarget();

export function melden(e: Projektereignis) {
  ziel.dispatchEvent(new CustomEvent<Projektereignis>("projekt", { detail: e }));
}

/** Meldet nach erfolgreichem Abschluss; Fehler bleiben beim Aufrufer. */
export async function nachErfolg<T>(
  aufruf: Promise<T>,
  e: Projektereignis,
): Promise<T> {
  const ergebnis = await aufruf;
  melden(e);
  return ergebnis;
}

/** Abonniert Ereignisse einer Akte; gibt die Abmeldung zurück (für `$effect`). */
export function abonnieren(
  projektId: string,
  bereiche: Datenbereich[],
  fn: (e: Projektereignis) => void,
): () => void {
  const hoerer = (ev: Event) => {
    const e = (ev as CustomEvent<Projektereignis>).detail;
    if (e.projektId === projektId && bereiche.includes(e.bereich)) fn(e);
  };
  ziel.addEventListener("projekt", hoerer);
  return () => ziel.removeEventListener("projekt", hoerer);
}

/** Büroangaben sind projektübergreifend (Entwurfsverfasser im Bauantrag). */
export function bueroGeaendert() {
  ziel.dispatchEvent(new Event("buero"));
}
export function bueroAbonnieren(fn: () => void): () => void {
  ziel.addEventListener("buero", fn);
  return () => ziel.removeEventListener("buero", fn);
}
