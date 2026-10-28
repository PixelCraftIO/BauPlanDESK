import type { Abgleichsfrage, Zeilenzuordnung } from "../generated/plattform";
/** Rückfrage vor einer Übernahme aus dem Aufmaß, wenn schon passende Zeilen von
 * Hand erfasst sind. Die Kandidaten und die Vorbelegung liefert der Core; die
 * Maske gibt nur die Entscheidung zurück (`null` = abgebrochen). */
export class Abgleich {
  daten = $state<{ fragen: Abgleichsfrage[]; wahl: Record<string, string>; antwort: (z: Zeilenzuordnung[] | null) => void } | null>(null);
  fragen(fragen: Abgleichsfrage[]): Promise<Zeilenzuordnung[] | null> {
    this.abbrechen();
    if (!fragen.length) return Promise.resolve([]);
    return new Promise((resolve) => {
      this.daten = { fragen, wahl: Object.fromEntries(fragen.map((f) => [f.markierung_id, f.vorschlag ?? ""])), antwort: resolve };
    });
  }
  bestaetigen() {
    const d = this.daten;
    this.daten = null;
    d?.antwort(d.fragen.map((f) => ({ markierung_id: f.markierung_id, ...(d.wahl[f.markierung_id] ? { zeile: d.wahl[f.markierung_id] } : {}) })));
  }
  abbrechen() {
    const d = this.daten;
    this.daten = null;
    d?.antwort(null);
  }
}
