<script lang="ts">
  import { KUNDENKATEGORIEN, type Kundendaten } from "./api";
  import AnschriftFelder from "./AnschriftFelder.svelte";

  let { daten = $bindable(), gesperrt = false }: { daten: Kundendaten; gesperrt?: boolean } = $props();
</script>

<div class="kundenfelder">
  <div class="umschalter" role="radiogroup" aria-label="Art">
    <label><input type="radio" bind:group={daten.typ} value="person" disabled={gesperrt} /> Person</label>
    <label><input type="radio" bind:group={daten.typ} value="organisation" disabled={gesperrt} /> Firma / Organisation</label>
  </div>
  <div class="anschrift">
    <label>Kategorie
      <select bind:value={daten.kategorie} disabled={gesperrt}>
        {#each KUNDENKATEGORIEN as k}<option value={k.code}>{k.name}</option>{/each}
      </select>
    </label>
    <label>Kundennummer <input bind:value={daten.kundennummer} disabled={gesperrt} size="10" /></label>
  </div>
  {#if daten.typ === "person"}
    <div class="anschrift">
      <label class="breit">Vorname <input bind:value={daten.vorname} disabled={gesperrt} /></label>
      <label class="breit">Familienname <span class="pflicht" aria-hidden="true"> *</span><input aria-label="Familienname" bind:value={daten.familienname} disabled={gesperrt} required /></label>
    </div>
  {:else}
    <div class="anschrift">
      <label class="breit">Firma / Organisation <span class="pflicht" aria-hidden="true"> *</span><input aria-label="Firma / Organisation" bind:value={daten.firma} disabled={gesperrt} required /></label>
      <label class="breit">Ansprechpartner <input bind:value={daten.ansprechpartner} disabled={gesperrt} /></label>
    </div>
  {/if}
  <AnschriftFelder bind:anschrift={daten.anschrift} {gesperrt} />
  <div class="anschrift">
    <label class="breit">Telefon <input type="tel" bind:value={daten.telefon} disabled={gesperrt} /></label>
    <label class="breit">E-Mail <input type="email" bind:value={daten.email} disabled={gesperrt} /></label>
  </div>
  <label class="voll">Notizen <textarea bind:value={daten.notizen} disabled={gesperrt} rows="3"></textarea></label>
</div>
