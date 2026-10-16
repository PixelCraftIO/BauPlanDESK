import { getContext, setContext } from "svelte";
import type { UebersichtsZiel } from "./generated/plattform";
const SCHLUESSEL = Symbol("projekt-navigation");
export class ProjektNavigation {
  auftrag = $state<{ nummer: number; ziel: UebersichtsZiel } | null>(null);
  private nummer = 0;
  springen(ziel: UebersichtsZiel) {
    this.auftrag = { nummer: ++this.nummer, ziel };
  }
}
export function projektNavigationAnlegen() {
  const n = new ProjektNavigation();
  setContext(SCHLUESSEL, n);
  return n;
}
export function projektNavigation() {
  return getContext<ProjektNavigation | undefined>(SCHLUESSEL);
}
