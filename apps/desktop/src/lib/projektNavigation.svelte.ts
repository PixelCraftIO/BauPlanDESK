import { getContext, setContext } from "svelte";
import type { UebersichtsZiel } from "./generated/plattform";
import { api } from "./api";
import { kategorieText } from "./kategorien";
const SCHLUESSEL = Symbol("projekt-navigation");
export class ProjektNavigation {
  projektId = "";
  kategorien = $state<Record<string, string>>({});
  private kategorienAnfrage: Promise<void> | null = null;
  kategorienLaden() {
    if (!this.projektId) return Promise.resolve();
    if (this.kategorienAnfrage) return this.kategorienAnfrage;
    this.kategorienAnfrage = Promise.all([
      api.kategorienkatalog(),
      api.dokumentkategorien(this.projektId),
    ])
      .then(([k, z]) => {
        this.kategorien = Object.fromEntries(
          z.map((d) => [
            d.dokument_id,
            kategorieText(k.kategorien, d.kategorie),
          ]),
        );
      })
      .finally(() => {
        this.kategorienAnfrage = null;
      });
    return this.kategorienAnfrage;
  }
  benutzerId = $state<string | null>(null);
  vollgeschossauftrag = $state<{
    gebaeude: string;
    geschoss: string;
    bezeichnung: string;
  } | null>(null);
  verfahren = $state<string | null>(null);
  rueckweg = $state<UebersichtsZiel | null>(null);
  unterlagenauftrag = $state<{
    ziel: UebersichtsZiel;
    titel: string;
    dokument: { dokument_id: string; version: number } | null;
  } | null>(null);
  druckauswahl = $state<{
    titel: string;
    dokumente: { dokument_id: string; version: number }[];
  } | null>(null);
  auftrag = $state<{ nummer: number; ziel: UebersichtsZiel } | null>(null);
  private nummer = 0;
  springen(ziel: UebersichtsZiel) {
    if (["verfahren", "portale"].includes(ziel.bereich) && ziel.objekt)
      this.verfahren = ziel.objekt;
    this.auftrag = { nummer: ++this.nummer, ziel };
  }
  unterlageErstellen(
    ziel: UebersichtsZiel,
    titel: string,
    bereich = "formulare",
  ) {
    this.unterlagenauftrag = { ziel, titel, dokument: null };
    this.springen({ bereich, objekt: null, register: null, eintrag: null });
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
