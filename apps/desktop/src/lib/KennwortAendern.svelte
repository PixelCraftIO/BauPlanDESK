<script lang="ts">
  import { api, fehlertext, type Anmeldestatus } from "./api";

  let {
    bisherNoetig,
    mindestlaenge,
    knopf = "Kennwort speichern",
    ongespeichert,
    onabbrechen,
  }: {
    /** Bisheriges Kennwort bzw. Erstkennwort abfragen. */
    bisherNoetig: boolean;
    mindestlaenge: number;
    knopf?: string;
    ongespeichert: (s: Anmeldestatus) => void;
    onabbrechen?: () => void;
  } = $props();

  let bisher = $state("");
  let neu = $state("");
  let wiederholung = $state("");
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);

  async function speichern(ev: SubmitEvent) {
    ev.preventDefault();
    fehler = null;
    if (neu !== wiederholung) {
      fehler = "Die beiden neuen Kennwörter stimmen nicht überein.";
      return;
    }
    laeuft = true;
    try {
      const s = await api.kennwortAendern(bisherNoetig ? bisher : null, neu);
      bisher = neu = wiederholung = "";
      ongespeichert(s);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

<form class="kennwortformular" onsubmit={speichern}>
  {#if bisherNoetig}
    <label
      >Bisheriges Kennwort bzw. Erstkennwort
      <input
        type="password"
        autocomplete="current-password"
        bind:value={bisher}
        required
      /></label
    >
  {/if}
  <label
    >Neues Kennwort
    <input
      type="password"
      autocomplete="new-password"
      bind:value={neu}
      required
    /></label
  >
  <label
    >Neues Kennwort wiederholen
    <input
      type="password"
      autocomplete="new-password"
      bind:value={wiederholung}
      required
    /></label
  >
  <p class="meta">
    Mindestens {mindestlaenge} Zeichen. Ein längerer Satz ist sicherer als Sonderzeichen.
  </p>
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  <div class="knoepfe">
    {#if onabbrechen}<button
        type="button"
        class="sekundaer"
        disabled={laeuft}
        onclick={onabbrechen}>Abbrechen</button
      >{/if}
    <button type="submit" disabled={laeuft}
      >{laeuft ? "Wird gespeichert …" : knopf}</button
    >
  </div>
</form>

<style>
  .kennwortformular {
    display: grid;
    gap: var(--space-2);
  }
  .kennwortformular label {
    display: grid;
    gap: 0.2rem;
  }
</style>
