<script lang="ts">
  /** IFC-Datei dem Bauantrag (XBau) als Anlage „BIM-Gebäudemodell“ beifügen. */
  import { fehlertext } from "../api";
  import { ifcApi } from "./api";
  let { projektId, dokumentId, bezeichnung = "", darf, titel = "Dem Bauantrag beifügen" }: {
    projektId: string; dokumentId: string; bezeichnung?: string; darf: boolean; titel?: string;
  } = $props();
  let laeuft = $state(false), meldung = $state(""), fehler = $state("");
  async function beifuegen() {
    laeuft = true; meldung = ""; fehler = "";
    try {
      const r = await ifcApi.bauantragBeifuegen(projektId, dokumentId, bezeichnung);
      meldung = !r.neu ? "Liegt dem Bauantrag schon bei." : `Dem Bauantrag beigefügt${r.anlageart ? " als „BIM-Gebäudemodell“ (2010)" : "; Anlagenart im Bauantrag wählen"} · ${r.anlagen} Anlage(n).`;
    } catch (e) { fehler = fehlertext(e); } finally { laeuft = false; }
  }
</script>

<span class="anlage">
  <button type="button" class="sekundaer" disabled={!darf || laeuft} onclick={() => void beifuegen()}>{titel}</button>
  {#if meldung}<span class="meta" role="status">{meldung}</span>{/if}
  {#if fehler}<span class="fehlertext" role="alert">{fehler}</span>{/if}
</span>

<style>
  .anlage{display:inline-flex;flex-direction:column;gap:2px;max-width:100%}
</style>
