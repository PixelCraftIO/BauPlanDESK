<script lang="ts">
  import { onMount } from "svelte";
  import { api, fehlertext, landName, type Project } from "./api";

  let {
    onoeffnen,
    onabbrechen,
  }: { onoeffnen: (id: string) => void; onabbrechen: () => void } = $props();

  let projekte = $state<Project[]>([]);
  let filter = $state("");
  let auswahl = $state<string | null>(null);
  let fehler = $state<string | null>(null);

  const gefiltert = $derived(
    projekte.filter((p) => p.name.toLowerCase().includes(filter.trim().toLowerCase())),
  );

  onMount(async () => {
    try {
      projekte = await api.projekte();
      auswahl = projekte[0]?.id ?? null;
    } catch (e) {
      fehler = fehlertext(e);
    }
  });

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (auswahl) onoeffnen(auswahl);
  }
</script>

<form class="spalte" onsubmit={submit}>
  <!-- svelte-ignore a11y_autofocus -->
  <input placeholder="Suchen …" bind:value={filter} autofocus />
  {#if fehler}<p class="fehlertext">{fehler}</p>{/if}
  <ul class="auswahlliste" role="listbox" aria-label="Projekte">
    {#each gefiltert as p (p.id)}
      <li
        role="option"
        aria-selected={auswahl === p.id}
        class:aktiv={auswahl === p.id}
        onclick={() => (auswahl = p.id)}
        ondblclick={() => onoeffnen(p.id)}
        onkeydown={(e) => e.key === "Enter" && onoeffnen(p.id)}
        tabindex="0"
      >
        <strong>{p.name}</strong>
        <span class="meta">{landName(p.bundesland)} · geändert {new Date(p.updated_at).toLocaleDateString("de-DE")}</span>
      </li>
    {:else}
      <li class="meta">Keine Projekte gefunden.</li>
    {/each}
  </ul>
  <div class="knoepfe">
    <button type="button" class="sekundaer" onclick={onabbrechen}>Abbrechen</button>
    <button type="submit" disabled={!auswahl}>Öffnen</button>
  </div>
</form>
