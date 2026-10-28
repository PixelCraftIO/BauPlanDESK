<script lang="ts">
  import { onDestroy } from "svelte";
  import Dialog from "../Dialog.svelte";
  import type { Abgleich } from "./Abgleich.svelte";
  import { werttext } from "./Uebernahme.svelte";
  let { a }: { a: Abgleich } = $props();
  onDestroy(() => a.abbrechen());
</script>
{#if a.daten}<Dialog titel="Bereits erfasste Flächen abgleichen" breit onclose={() => a.abbrechen()}>
  <p>Zu diesen gezeichneten Flächen gibt es schon Zeilen, die von Hand erfasst wurden. Damit keine Fläche doppelt zählt, wählen Sie je Markierung die Zeile, die den gezeichneten Wert übernehmen soll. Art und Grundlage der Zeile bleiben erhalten.</p>
  <div class="tabellen-scroll"><table class="ergebnistabelle" aria-label="Abgleich mit erfassten Zeilen">
    <thead><tr><th>Ziel</th><th>Markierung</th><th>Gezeichnet</th><th>Übernehmen in</th></tr></thead>
    <tbody>{#each a.daten.fragen as f (f.markierung_id)}<tr>
      <th scope="row">{f.ziel}</th><td>{f.markierung}</td><td>{werttext(f.flaeche_m2, "m²")}</td>
      <td><select aria-label={`Zeile für ${f.markierung}`} bind:value={a.daten.wahl[f.markierung_id]}>
        {#each f.kandidaten as k (k.zeile)}<option value={k.zeile}>{k.bezeichnung} · bisher {werttext(k.flaeche_m2, "m²")}</option>{/each}
        <option value="">Als neue Zeile anlegen</option>
      </select></td>
    </tr>{/each}</tbody>
  </table></div>
  <div class="knoepfe"><button type="button" onclick={() => a.bestaetigen()}>Übernehmen</button><button type="button" class="sekundaer" onclick={() => a.abbrechen()}>Abbrechen</button></div>
</Dialog>{/if}
