<script lang="ts">
  import { leereAnschrift, type Beteiligter } from "./api";
  import AnschriftFelder from "./AnschriftFelder.svelte";

  let { wer = $bindable(), gesperrt = false }: { wer: Beteiligter; gesperrt?: boolean } = $props();

  function artWechseln(art: "person" | "organisation") {
    if (art === wer.art) return;
    const anschrift = wer.anschrift ?? leereAnschrift();
    wer = art === "person"
      ? { art, id: wer.id, vorname: "", familienname: "", anschrift }
      : { art, id: wer.id, name: "", anschrift };
  }
</script>

<div class="beteiligter">
  <div class="umschalter" role="radiogroup" aria-label="Art">
    <label><input type="radio" checked={wer.art === "person"} disabled={gesperrt} onchange={() => artWechseln("person")} /> Person</label>
    <label><input type="radio" checked={wer.art === "organisation"} disabled={gesperrt} onchange={() => artWechseln("organisation")} /> Firma / Organisation</label>
  </div>
  {#if wer.art === "person"}
    <div class="anschrift">
      <label class="breit">Vorname <input bind:value={wer.vorname} disabled={gesperrt} /></label>
      <label class="breit">Familienname <input bind:value={wer.familienname} disabled={gesperrt} /></label>
    </div>
  {:else}
    <label class="voll">Name <input bind:value={wer.name} disabled={gesperrt} /></label>
  {/if}
  <AnschriftFelder bind:anschrift={wer.anschrift} {gesperrt} />
</div>
