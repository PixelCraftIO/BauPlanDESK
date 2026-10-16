<script lang="ts">
  import { onMount } from "svelte";
  import { fehlertext, mindestens, type Benutzer } from "./api";
  import { druckApi } from "./druckApi";
  import type { Druckgestaltung } from "./generated/plattform";
  let { ich }: { ich: Benutzer | null } = $props();
  let g = $state<Druckgestaltung | null>(null),
    busy = $state(false),
    fehler = $state(""),
    meldung = $state("");
  let original = $state("");
  const darf = $derived(
    !!ich?.aktiv && mindestens(ich.rolle, "projektleitung"),
  );
  const dirty = $derived(!!g && JSON.stringify(g) !== original);
  onMount(async () => {
    try {
      g = await druckApi.gestaltung();
      original = JSON.stringify(g);
    } catch (e) {
      fehler = fehlertext(e);
    }
  });
  export function istBeschaeftigt() {
    return busy || dirty;
  }
  async function speichern() {
    if (!g) return;
    busy = true;
    fehler = "";
    try {
      g = await druckApi.gestaltungSpeichern(g);
      original = JSON.stringify(g);
      meldung = "Bürostandard für künftige Druckausgaben gespeichert.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
</script>

<details>
  <summary>Briefbogen und Druckvorlagen</summary>
  <p class="meta">
    Gemeinsamer Bürostandard für neue Druckausgaben aller Projekte. Bereits
    gespeicherte PDF-Dateien bleiben unverändert.
  </p>
  {#if fehler}<p role="alert" class="fehlertext">{fehler}</p>{/if}
  {#if meldung}<p role="status">{meldung}</p>{/if}
  {#if g}
    <fieldset disabled={!darf || busy}>
      <legend>Bürostandard</legend>
      <label
        >Kopfzeile<textarea rows="3" maxlength="1000" bind:value={g.kopfzeile}
        ></textarea></label
      >
      <label
        >Fußzeile<textarea rows="3" maxlength="1000" bind:value={g.fusszeile}
        ></textarea></label
      >
      <p class="meta">
        Platzhalter: {"{Projekt}"}, {"{Datum}"}, {"{Seite}"}, {"{Seiten}"}.
        Jeweils maximal drei Druckzeilen; Paket-Seitenzahlen werden automatisch
        ergänzt. Gilt für erzeugte Berichte, Anschreiben und
        Inhaltsverzeichnisse.
      </p>
      <label
        >Vorlage für das Anschreiben<textarea
          rows="9"
          maxlength="16000"
          bind:value={g.anschreiben}
        ></textarea></label
      >
      <button disabled={!dirty} onclick={speichern}
        >Druckvorlage speichern</button
      >
      <button
        class="sekundaer"
        disabled={!dirty}
        onclick={() => {
          g = JSON.parse(original);
          fehler = "";
        }}>Änderungen verwerfen</button
      >
    </fieldset>
    {#if !darf}<p class="meta">
        Den Bürostandard ändert die Projektleitung. Texte einer einzelnen
        Druckausgabe können im Projekt angepasst werden.
      </p>{/if}
  {/if}
</details>

<style>
  label {
    display: block;
    margin-block: 1rem;
  }
  textarea {
    display: block;
    width: 100%;
    box-sizing: border-box;
  }
  button {
    margin-right: 1rem;
  }
</style>
