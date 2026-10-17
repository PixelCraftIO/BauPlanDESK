<script lang="ts">
  import { api, fehlertext, leereAnschrift, mindestens, type Benutzer, type Buero } from "./api";
  import AnschriftFelder from "./AnschriftFelder.svelte";

  let { ich, onschliessen }: { ich: Benutzer; onschliessen: () => void } = $props();

  const darf = $derived(ich.aktiv && mindestens(ich.rolle, "projektleitung"));
  let buero = $state<Buero>({ name: "", anschrift: leereAnschrift(), kennung: "" });
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);

  $effect(() => {
    api.buero().then((b) => (buero = b)).catch((e) => (fehler = fehlertext(e)));
  });

  async function speichern(e: SubmitEvent) {
    e.preventDefault();
    laeuft = true;
    fehler = null;
    try {
      await api.bueroSpeichern(buero);
      onschliessen();
    } catch (err) {
      fehler = fehlertext(err);
    } finally {
      laeuft = false;
    }
  }
</script>

<form class="spalte" onsubmit={speichern}>
  <p class="meta">Das Büro ist Absender jedes XBau-Bauantrags und wird als Entwurfsverfasser eingetragen.</p>
  <label>Name des Büros
    <span class="pflicht" aria-hidden="true"> *</span><input aria-label="Name des Büros" bind:value={buero.name} disabled={!darf} required />
  </label>
  <AnschriftFelder bind:anschrift={buero.anschrift} gesperrt={!darf} />
  <label>Absenderkennung
    <input bind:value={buero.kennung} disabled={!darf} placeholder="z. B. vom Antragsportal vergeben" />
  </label>
  {#if !darf}<p class="meta">Büroangaben ändern kann die Projektleitung.</p>{/if}
  {#if fehler}<p class="fehlertext">{fehler}</p>{/if}
  <div class="knoepfe">
    <button type="button" class="sekundaer" onclick={onschliessen}>{darf ? "Abbrechen" : "Schließen"}</button>
    {#if darf}<button type="submit" disabled={laeuft}>Speichern</button>{/if}
  </div>
</form>
