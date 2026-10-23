/** Ansichtsabfragen pausieren bei verdeckten Bereichen; Editorzustände bleiben
 * montiert. Ereignisse werden bis zur Sichtbarkeit zusammengefasst. */
import { getContext, setContext, untrack } from "svelte";
import { abonnieren, type Datenbereich } from "./projektEreignisse";
const AKTIV = Symbol("arbeitsbereich-aktiv");
export function bereichAktivSetzen(aktiv: () => boolean) {
  setContext(AKTIV, aktiv);
}
export function bereichAktiv() {
  return getContext<(() => boolean) | undefined>(AKTIV) ?? (() => true);
}
export function ansichtsAbfrage(
  projekt: () => string,
  bereiche: Datenbereich[],
  laden: () => Promise<unknown>,
) {
  const aktiv = bereichAktiv();
  let offen = $state(true),
    laeuft = $state(false);
  function vormerken() {
    offen = true;
  }
  $effect(() => abonnieren(projekt(), bereiche, vormerken));
  $effect(() => {
    if (!aktiv() || !offen || laeuft) return;
    offen = false;
    laeuft = true;
    untrack(() => {
      void laden()
        .catch(() => {
          /* Die Fachansicht meldet ihre Ladefehler. */
        })
        .finally(() => {
          laeuft = false;
        });
    });
  });
  return vormerken;
}
