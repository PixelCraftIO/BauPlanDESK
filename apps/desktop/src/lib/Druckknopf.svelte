<script lang="ts">
  import { fehlertext } from "./api";
  import Dialog from "./Dialog.svelte";
  import Druckausgabe from "./Druckausgabe.svelte";
  import type { Druckquelle } from "./druckApi";
  let {
    projektId,
    quelle,
    vorbereiten,
    disabled = false,
    text = "Drucken / PDF",
  }: {
    projektId: string;
    quelle?: Druckquelle;
    vorbereiten?: () => Promise<Druckquelle | null>;
    disabled?: boolean;
    text?: string;
  } = $props();
  let offen = $state(false),
    busy = $state(false);
  let startquelle = $state<Druckquelle>();
  let fehler = $state("");
  async function oeffnen() {
    if (busy) return;
    busy = true;
    fehler = "";
    try {
      startquelle = vorbereiten ? ((await vorbereiten()) ?? undefined) : quelle;
      if (vorbereiten && !startquelle) return;
      offen = true;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
</script>

<button class="sekundaer" disabled={disabled || busy} onclick={oeffnen}
  >{text}</button
>
{#if fehler}<span role="alert" class="fehlertext">{fehler}</span>{/if}
{#if offen}<Dialog
    titel="Druckausgabe"
    breit
    schliessenErlaubt={!busy}
    onclose={() => (offen = false)}
    ><Druckausgabe
      {projektId}
      vorauswahl={startquelle}
      onbusy={(b) => (busy = b)}
    /><button class="sekundaer" disabled={busy} onclick={() => (offen = false)}
      >Schließen</button
    ></Dialog
  >{/if}
