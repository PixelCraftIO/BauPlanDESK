<script lang="ts">
  import type { Frist } from "../generated/plattform";
  import type { DocumentVersion } from "../api";
  import Dokumentfeld from "./Dokumentfeld.svelte";
  import { heute } from "./api";
  let {
    wert = $bindable(null),
    dateien,
  }: { wert?: Frist | null; dateien: DocumentVersion[] } = $props();
</script>

{#if wert}<div class="raster">
    <label
      >Fristdatum<input type="date" required bind:value={wert.datum} /></label
    ><label>Fristauslöser<input required bind:value={wert.ausloeser} /></label
    ><label
      >Grund / Verlängerung<input required bind:value={wert.grund} /></label
    ><label
      >Erinnerung Tage vorher<input
        type="number"
        min="0"
        max="365"
        bind:value={wert.erinnerung_tage}
      /></label
    ><Dokumentfeld {dateien} bind:wert={wert.beleg} label="Fristbeleg" />
  </div>
  <button type="button" class="link" onclick={() => (wert = null)}
    >Frist entfernen (Änderung bleibt im Verlauf)</button
  >{:else}<button
    type="button"
    class="sekundaer"
    onclick={() =>
      (wert = {
        datum: heute(),
        ausloeser: "",
        beleg: null,
        erinnerung_tage: 7,
        grund: "",
      })}>Frist / Wiedervorlage erfassen</button
  >{/if}
