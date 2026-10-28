<script lang="ts">
  /** Export: Datei der Modellakte mit Nacharbeit neu schreiben, eigenes Modell aus Zeichnungen. */
  import { fehlertext } from "../api";
  import type { Datensatz } from "../plattformApi";
  import type { Exportergebnis, Modellakte } from "../generated/plattform";
  import { ifcApi } from "./api";
  import Exportbericht from "./Exportbericht.svelte";
  import Entwurf from "./Entwurf.svelte";
  let { projektId, akte, darf, onoeffnen }: {
    projektId: string;
    akte: Datensatz<Modellakte>;
    darf: boolean;
    onoeffnen?: (e: Exportergebnis) => void;
  } = $props();
  let laeuft = $state(false), fehler = $state("");
  let ergebnis = $state<Exportergebnis | null>(null);
  const zahl = $derived(akte.daten.nacharbeit?.length ?? 0);
  async function schreiben() {
    laeuft = true; fehler = ""; ergebnis = null;
    try { ergebnis = await ifcApi.exportieren(projektId, { art: "nacharbeit", akte_id: akte.objekt }); }
    catch (e) { fehler = fehlertext(e); } finally { laeuft = false; }
  }
</script>

<section class="export" aria-label="Export">
  <h4>IFC-Datei mit Nacharbeit</h4>
  <p class="meta">Schreibt die Datei der Modellakte mit {zahl === 1 ? "der Änderung" : `den ${zahl} Änderungen`} der Nacharbeit neu, im Schema der Quelle: Klassen, Namen, Eigenschaften und Geschosszuordnung ersetzt, gebildete Räume mit Basismengen ergänzt, ausgeblendete Elemente entfernt. Alles andere bleibt zeichengleich. Die Datei wird zur Kontrolle wieder eingelesen und in den Projektdateien abgelegt.</p>
  <button type="button" disabled={!darf || laeuft || !zahl} onclick={() => void schreiben()}>IFC-Datei schreiben</button>
  {#if !zahl}<p class="meta">Noch keine Nacharbeit; die Datei ist unverändert.</p>{/if}
  {#if laeuft}<p class="meta" role="status">Datei wird geschrieben …</p>{/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if ergebnis}<Exportbericht {ergebnis} onoeffnen={onoeffnen ? () => onoeffnen!(ergebnis!) : undefined}/>{/if}
  <h4>Eigenes IFC4-Modell aus Zeichnungen</h4>
  <Entwurf {projektId} {darf} {onoeffnen}/>
</section>

<style>
  h4{margin:8px 0 4px;font-size:.85rem}
</style>
