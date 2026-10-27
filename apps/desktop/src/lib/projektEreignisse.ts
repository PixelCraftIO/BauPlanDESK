/**
 * Projektweite Hinweise zwischen den Bereichen einer Projektakte.
 *
 * Bereiche bleiben ausgeblendet geladen (Projektakte.svelte). Ändert ein Bereich
 * Dokumente, Fachobjekte oder Nachweise, laden die anderen Bereiche ihre
 * Auswahllisten derselben Akte neu. Das Ereignis trägt nur die Projekt-ID und
 * den Datenbereich; maßgebend bleibt der neu geladene Stand aus dem Core.
 */
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { leseAbfragenVerwerfen } from "./leseAbfrage";
export type Datenbereich = "dokumente" | "fachobjekte" | "nachweise";
export interface Projektereignis {
  projektId: string;
  bereich: Datenbereich;
  /** Auslösender Bereich, damit er seine eigene Meldung überspringen kann. */
  quelle?: string;
  /** Fenster, in dem das Ereignis entstand (Brücke zwischen Haupt- und Editorfenster). */
  fenster?: string;
}

const ziel = new EventTarget();
const FENSTER_EREIGNIS = "projekt-ereignis";
let eigenesFenster = "";
try { eigenesFenster = getCurrentWindow().label; } catch {}

function verteilen(e: Projektereignis) {
  leseAbfragenVerwerfen();
  ziel.dispatchEvent(new CustomEvent<Projektereignis>("projekt", { detail: e }));
}

export function melden(e: Projektereignis) {
  const ereignis = { ...e, fenster: eigenesFenster };
  verteilen(ereignis);
  // Andere Fenster derselben Anwendung (Editorfenster) sehen den Stand ebenfalls neu.
  void emit(FENSTER_EREIGNIS, ereignis).catch(() => {});
}

// Ereignisse aus anderen Fenstern lokal weiterreichen; eigene kommen nicht zurück.
void listen<Projektereignis>(FENSTER_EREIGNIS, (ev) => {
  if (ev.payload?.fenster && ev.payload.fenster === eigenesFenster) return;
  verteilen(ev.payload);
}).catch(() => {});

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
