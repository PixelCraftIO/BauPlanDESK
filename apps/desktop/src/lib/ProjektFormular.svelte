<script lang="ts">
  import { LAENDER, type Projektwerte as Werte } from "./api";

  let {
    start,
    absenden,
    knopf,
    onabbrechen,
  }: {
    start: Werte;
    absenden: (w: Werte) => Promise<void>;
    knopf: string;
    onabbrechen: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let werte = $state<Werte>({ ...start });
  let laeuft = $state(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    laeuft = true;
    try {
      await absenden({
        ...werte,
        name: werte.name.trim(),
        grundstuecksflaeche_m2: werte.grundstuecksflaeche_m2 || null,
      });
    } finally {
      laeuft = false;
    }
  }
</script>

<form class="spalte" onsubmit={submit}>
  <label>
    Projektname
    <!-- svelte-ignore a11y_autofocus -->
    <input bind:value={werte.name} required autofocus />
  </label>
  <label>
    Bundesland
    <select bind:value={werte.bundesland}>
      {#each LAENDER as l}<option value={l.code}>{l.name}</option>{/each}
    </select>
  </label>
  <label>
    Grundstücksfläche (m²)
    <input type="number" min="0.01" step="0.01" bind:value={werte.grundstuecksflaeche_m2} />
  </label>
  <div class="knoepfe">
    <button type="button" class="sekundaer" onclick={onabbrechen}>Abbrechen</button>
    <button type="submit" disabled={laeuft}>{knopf}</button>
  </div>
</form>
