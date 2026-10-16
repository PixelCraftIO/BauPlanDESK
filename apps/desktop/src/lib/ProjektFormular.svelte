<script lang="ts">
  import {
    LAENDER,
    kundenname,
    type Kunde,
    type Projektwerte as Werte,
  } from "./api";

  let {
    start,
    kunden = null,
    kundenLadefehler = null,
    absenden,
    knopf,
    onabbrechen,
  }: {
    start: Werte;
    /** Kunden zur Auswahl des Bauherrn (nur beim Anlegen). */
    kunden?: Kunde[] | null;
    kundenLadefehler?: string | null;
    absenden: (w: Werte, bauherr: string | null) => Promise<void>;
    knopf: string;
    onabbrechen: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let werte = $state<Werte>({ ...start });
  let bauherr = $state<string | null>(null);
  let laeuft = $state(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (laeuft) return;
    laeuft = true;
    try {
      await absenden(
        {
          ...werte,
          name: werte.name.trim(),
          grundstuecksflaeche_m2: werte.grundstuecksflaeche_m2 || null,
        },
        bauherr,
      );
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
  <p class="meta">
    Flurstücke mit ihren Flächen und optionalen Grundbuchangaben werden in der
    Projektakte unter Grundstücksinformationen erfasst. Die Baugrundstücksfläche
    wird separat berechnet.
  </p>
  {#if kundenLadefehler}
    <p class="fehlertext" role="alert">Bauherren konnten nicht geladen werden: {kundenLadefehler} Sie können das Projekt anlegen und den Bauherrn später zuordnen.</p>
  {/if}
  {#if kunden}
    <label>
      Bauherr
      <select bind:value={bauherr}>
        <option value={null}>– später zuordnen –</option>
        {#each kunden as k (k.id)}<option value={k.id}
            >{kundenname(k)}{k.kundennummer
              ? ` (${k.kundennummer})`
              : ""}</option
          >{/each}
      </select>
    </label>
  {/if}
  <div class="knoepfe">
    <button type="button" class="sekundaer" onclick={onabbrechen}
      >Abbrechen</button
    >
    <button type="submit" disabled={laeuft}>{knopf}</button>
  </div>
</form>
