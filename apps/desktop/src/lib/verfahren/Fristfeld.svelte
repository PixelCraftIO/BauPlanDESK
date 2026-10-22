<script lang="ts">
  import type { Frist } from "../generated/plattform";
  import type { DocumentVersion } from "../api";
  import Dokumentfeld from "./Dokumentfeld.svelte";
  import { heute } from "./api";
  let {
    // Optionale Fristen können im Core-JSON fehlen. Kein bindbarer Fallback:
    // Svelte verbietet sonst bind:wert={undefined} beim Öffnen solcher Akten.
    wert = $bindable(),
    dateien,
  }: { wert?: Frist | null; dateien: DocumentVersion[] } = $props();
</script>

{#if wert}<div class="raster">
    <label
      >Fristdatum<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Fristdatum" type="date" required bind:value={wert.datum} /></label
    ><label>Fristauslöser<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Fristauslöser" required bind:value={wert.ausloeser} /></label
    ><label
      >Grund / Verlängerung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Grund / Verlängerung" required bind:value={wert.grund} /></label
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
