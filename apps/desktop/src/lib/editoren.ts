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
  /** Ungespeicherte Änderungen in einem bestimmten Bereich dieser Akte. */
  hatAenderungenIn(id: Ansicht) {
    return [...this.editoren.values()].some(
      (e) => e.id === id && e.editor.hatAenderungen(),
    );
  }
  istBeschaeftigt() {
    return [...this.editoren.values()].some(({ editor: e }) =>
      e.istBeschaeftigt(),
    );
  }
  async speichernIn(id: Ansicht): Promise<boolean> {
    for (const e of this.editoren.values()) {
      if (e.id !== id || !e.editor.hatAenderungen()) continue;
      if (e.editor.istBeschaeftigt() || !(await e.editor.speichern()))
        return false;
      await tick();
    }
    return !this.hatAenderungenIn(id);
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
/** Register der aktuellen Projektakte, sofern vorhanden. */
export function editorRegister() {
  return getContext<EditorRegister | undefined>(SCHLUESSEL);
}
export function editorAnmelden(id: Ansicht, editor: FachEditor) {
  const register = getContext<EditorRegister | undefined>(SCHLUESSEL);
  if (register) onDestroy(register.anmelden(id, editor));
}
