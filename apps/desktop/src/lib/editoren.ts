import { getContext, setContext, onDestroy, tick } from "svelte";
import { SvelteMap } from "svelte/reactivity";
import type { Ansicht } from "./projektSitzung";
export interface FachEditor {
  hatAenderungen(): boolean;
  istBeschaeftigt(): boolean;
  speichern(): Promise<boolean>;
  hinweis?: string;
}
const SCHLUESSEL = Symbol("projekt-editoren");
export class EditorRegister {
  private editoren = new SvelteMap<
    symbol,
    { id: Ansicht; editor: FachEditor }
  >();
  anmelden(id: Ansicht, editor: FachEditor) {
    const token = Symbol(id);
    this.editoren.set(token, { id, editor });
    return () => {
      this.editoren.delete(token);
    };
  }
  hatAenderungen() {
    return [...this.editoren.values()].some(({ editor: e }) =>
      e.hatAenderungen(),
    );
  }
  istBeschaeftigt() {
    return [...this.editoren.values()].some(({ editor: e }) =>
      e.istBeschaeftigt(),
    );
  }
  async speichern(): Promise<{ bereich: Ansicht; hinweis?: string } | null> {
    for (const { id: bereich, editor: e } of this.editoren.values()) {
      if (e.hatAenderungen()) {
        if (!(await e.speichern())) return { bereich, hinweis: e.hinweis };
        await tick();
      }
    }
    return null;
  }
}
export function projektEditoren() {
  const register = new EditorRegister();
  setContext(SCHLUESSEL, register);
  return register;
}
export function editorAnmelden(id: Ansicht, editor: FachEditor) {
  const register = getContext<EditorRegister | undefined>(SCHLUESSEL);
  if (register) onDestroy(register.anmelden(id, editor));
}
