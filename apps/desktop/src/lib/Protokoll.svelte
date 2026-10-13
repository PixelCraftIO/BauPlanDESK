<script lang="ts">
  import { AKTIONEN, api, fehlertext, type Protokolleintrag } from "./api";

  let { projektId }: { projektId: string } = $props();

  let eintraege = $state<Protokolleintrag[]>([]);
  let fehler = $state<string | null>(null);

  $effect(() => {
    api
      .protokoll(projektId)
      .then((e) => {
        eintraege = e;
        fehler = null;
      })
      .catch((e) => (fehler = fehlertext(e)));
  });
</script>

{#if fehler}<p class="fehlertext">{fehler}</p>{/if}

<table class="ergebnistabelle">
  <thead><tr><th>Zeit</th><th>Benutzer</th><th>Aktion</th><th>Details</th></tr></thead>
  <tbody>
    {#each eintraege as e (e.id)}
      <tr>
        <td>{new Date(e.zeit).toLocaleString("de-DE")}</td>
        <td>{e.benutzer_name}</td>
        <td>{AKTIONEN[e.aktion] ?? e.aktion}</td>
        <td>{e.details}</td>
      </tr>
    {:else}
      <tr><td colspan="4" class="meta">Noch keine Einträge.</td></tr>
    {/each}
  </tbody>
</table>
