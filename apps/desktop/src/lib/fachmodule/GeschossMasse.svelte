<script lang="ts">
  import type { GeschossGeometrie, GebaeudeGeschoss } from "../generated/plattform";
  import GrundlagenZahl from "./GrundlagenZahl.svelte";
  let { masse, onmasse, geschosse = [], id, einzelgeschoss = false }: { masse?: GeschossGeometrie; onmasse: (m: GeschossGeometrie) => void; geschosse?: GebaeudeGeschoss[]; id?: string; einzelgeschoss?: boolean } = $props();
  const felder = [
    ["flaeche_m2", "Geschossfläche nach Außenmaßen (m²)", 0],
    ["fussboden_m", "OK Fertigfußboden zu ±0,00 (m)", undefined],
    ["lichte_hoehe_m", "Lichte Raumhöhe (m)", 0],
    ["geschosshoehe_m", "Geschosshöhe, OKFF bis OKFF (m)", 0],
    ["deckendicke_m", "Deckendicke, UK bis OK Decke (m)", 0],
    ["decke_oberkante_m", "OK Decke zu ±0,00, falls bekannt (m)", undefined],
  ] as const;
</script>
<div class="raster">
  {#each felder.filter(([key]) => !einzelgeschoss || key !== "flaeche_m2") as [key, titel, min]}
    <GrundlagenZahl {titel} {min} wert={masse?.[key]} onwert={(v) => onmasse({ ...masse, [key]: v })} />
  {/each}
  {#if geschosse.length > 1}<label>Unterliegendes Geschoss<select value={masse?.unterliegend_geschoss_id ?? ""} onchange={(e) => onmasse({ ...masse, unterliegend_geschoss_id: e.currentTarget.value || undefined })}>
    <option value="">Noch nicht zugeordnet</option>
    {#each geschosse.filter((s) => s.id !== id) as s}<option value={s.id}>{s.bezeichnung || "Unbenanntes Geschoss"}</option>{/each}
  </select></label>{/if}
</div>
<label class="einheitlich"><input type="checkbox" checked={masse?.lichte_hoehe_einheitlich === true} onchange={(e) => onmasse({ ...masse, lichte_hoehe_einheitlich: e.currentTarget.checked })} />Die lichte Raumhöhe gilt für die gesamte Geschossfläche</label>
<p class="meta">Bei Dachschrägen oder unterschiedlichen Höhen die Höhenzonen im Vollgeschossnachweis einzeln erfassen. Alle Höhen beziehen sich auf denselben Gebäudenullpunkt.</p>
<style>.einheitlich { display: flex; align-items: center; gap: .5rem; margin-block: .7rem; }</style>
