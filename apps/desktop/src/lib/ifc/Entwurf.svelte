<script lang="ts">
  /** Eigenes IFC4-Modell aus Gebäude und je Geschoss einer eigenen Zeichnung
   * (Wände, Räume, Außenkante als Decke bzw. Dach). */
  import { untrack } from "svelte";
  import { fehlertext } from "../api";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import type { Exportergebnis, Gebaeude, Planaufmass } from "../generated/plattform";
  import { ifcApi } from "./api";
  import Exportbericht from "./Exportbericht.svelte";
  let { projektId, darf, onoeffnen }: {
    projektId: string;
    darf: boolean;
    /** Abgelegte Datei als Modellakte öffnen. */
    onoeffnen?: (e: Exportergebnis) => void;
  } = $props();
  let gebaeude = $state<Datensatz<Gebaeude>[]>([]);
  let zeichnungen = $state<Datensatz<Planaufmass>[]>([]);
  let haus = $state(""), wahl = $state<Record<string, string>>({});
  let laeuft = $state(false), fehler = $state("");
  let ergebnis = $state<Exportergebnis | null>(null);
  $effect(() => {
    void projektId;
    untrack(() => {
      void Promise.all([plattformApi.liste<Gebaeude>(projektId, "gebaeude"), plattformApi.liste<Planaufmass>(projektId, "planaufmass")]).then(([g, z]) => {
        gebaeude = g;
        zeichnungen = z.filter(x => !x.daten.dokument).sort((a, b) => a.daten.bezeichnung.localeCompare(b.daten.bezeichnung, "de"));
        if (!haus && g.length === 1) haus = g[0].objekt;
        vorschlagen();
      }).catch(e => (fehler = fehlertext(e)));
    });
  });
  const geschosse = $derived(gebaeude.find(g => g.objekt === haus)?.daten.geschosse ?? []);
  // Vorschlag: Zeichnung, deren Name den Geschossnamen trägt (z. B. „Entwurf · EG“).
  function vorschlagen() {
    const gs = gebaeude.find(g => g.objekt === haus)?.daten.geschosse ?? [];
    wahl = Object.fromEntries(gs.map(g => [g.id, zeichnungen.find(z => z.daten.bezeichnung.split(/[·\s]+/).includes(g.bezeichnung))?.objekt ?? ""]));
  }
  const gewaehlt = $derived(geschosse.filter(g => wahl[g.id]).map(g => ({ geschoss_id: g.id, zeichnung_id: wahl[g.id] })));
  async function schreiben() {
    laeuft = true; fehler = ""; ergebnis = null;
    try { ergebnis = await ifcApi.exportieren(projektId, { art: "entwurf", gebaeude_id: haus, geschosse: gewaehlt }); }
    catch (e) { fehler = fehlertext(e); } finally { laeuft = false; }
  }
</script>

<section class="entwurf" aria-label="Eigenes IFC4-Modell">
  <p class="meta">Schreibt aus den eigenen Zeichnungen ein IFC4-Modell: je Geschoss die Wände (Außenwände an der Geschoss-Außenkante), die Räume und eine Decke aus der Außenkante, im obersten Geschoss als Dach. Höhen kommen aus dem Gebäude. Die GlobalIds bleiben bei jedem Export gleich.</p>
  {#if !gebaeude.length}<p class="meta">Noch kein Gebäude im Projekt.</p>
  {:else}
    <fieldset disabled={!darf || laeuft}>
      <label>Gebäude<select aria-label="Gebäude für das Modell" bind:value={haus} onchange={vorschlagen}>
        <option value="">Gebäude wählen</option>
        {#each gebaeude as g (g.objekt)}<option value={g.objekt}>{g.daten.bezeichnung}</option>{/each}
      </select></label>
      {#each geschosse as g (g.id)}
        <label>{g.bezeichnung}<select aria-label={`Zeichnung für ${g.bezeichnung}`} bind:value={wahl[g.id]}>
          <option value="">nicht ins Modell</option>
          {#each zeichnungen as z (z.objekt)}<option value={z.objekt}>{z.daten.bezeichnung}</option>{/each}
        </select></label>
      {/each}
      {#if haus && !zeichnungen.length}<p class="meta">Keine eigene Zeichnung im Projekt. Im Aufmaß „Neue Zeichnung“ anlegen.</p>{/if}
      <button type="button" disabled={!haus || !gewaehlt.length} onclick={() => void schreiben()}>IFC4-Modell schreiben</button>
    </fieldset>
  {/if}
  {#if laeuft}<p class="meta" role="status">Modell wird geschrieben …</p>{/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if ergebnis}<Exportbericht {ergebnis} {projektId} {darf} onoeffnen={onoeffnen ? () => onoeffnen!(ergebnis!) : undefined}/>{/if}
</section>

<style>
  .entwurf fieldset{border:0;padding:0;margin:0 0 8px;display:flex;flex-direction:column;gap:6px}
  .entwurf label{display:flex;flex-direction:column;font-size:.85rem}
  .entwurf fieldset button{align-self:flex-start}
</style>
