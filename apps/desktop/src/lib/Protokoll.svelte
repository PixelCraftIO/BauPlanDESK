<script lang="ts">
  import { AKTIONEN, api, fehlertext, type Protokolleintrag } from "./api";

  let { projektId }: { projektId: string } = $props();

  let eintraege = $state<Protokolleintrag[]>([]);
  let fehler = $state<string | null>(null);
  // Technische Abläufe (Ordnerausgabe) bleiben im Protokoll, gehören aber
  // nicht in die fachliche Chronik; fehlgeschlagene bleiben sichtbar.
  const TECHNISCH = new Set(["ausgabeauftrag_geaendert"]);
  let technische = $state(false);
  const technisch = (e: Protokolleintrag) =>
    TECHNISCH.has(e.aktion) && !e.details.includes("fehlgeschlagen");
  const sichtbar = $derived(
    technische ? eintraege : eintraege.filter((e) => !technisch(e)),
  );
  const ausgeblendet = $derived(eintraege.length - sichtbar.length);

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
{#if ausgeblendet || technische}<label class="technisch"
    ><input type="checkbox" bind:checked={technische} />Technische Einträge anzeigen{#if !technische}<span
        class="meta"> ({ausgeblendet} ausgeblendet: Ordnerausgaben)</span
      >{/if}</label
  >{/if}

<table class="ergebnistabelle">
  <thead><tr><th>Zeit</th><th>Benutzer</th><th>Aktion</th><th>Details</th></tr></thead>
  <tbody>
    {#each sichtbar as e (e.id)}
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

<style>
  .technisch {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0.4rem 0;
  }
</style>
