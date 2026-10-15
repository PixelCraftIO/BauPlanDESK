<script lang="ts" generics="T">
  import { plattformApi, type Datensatz } from "./plattformApi";
  import { fehlertext } from "./api";
  import { onMount, type Snippet } from "svelte";
  import { editorAnmelden } from "./editoren";
  import type { Ansicht } from "./projektSitzung";
  import type { FachSitzung } from "./FachSitzung.svelte";
  let {
    titel,
    s,
    bereich,
    darf,
    children,
  }: {
    titel: string;
    s: FachSitzung<T>;
    bereich: Ansicht;
    darf: boolean;
    children: Snippet<[T]>;
  } = $props();
  let verlauf = $state<Datensatz<T>[]>([]);
  async function historie(id: string) {
    try {
      verlauf = await plattformApi.verlauf<T>(s.projektId, s.art, id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  onMount(() => s.laden());
  $effect(() => s.entwurfSichern());
  // Komponenten besitzen eine feste Route innerhalb ihrer Projektakte.
  // svelte-ignore state_referenced_locally
  editorAnmelden(bereich, {
    hatAenderungen: () => s.hatAenderungen(),
    istBeschaeftigt: () => s.istBeschaeftigt(),
    speichern: () => s.speichern(),
  });
</script>

<h2>{titel}</h2>
{#if s.fehler}<p class="fehlertext" role="alert">{s.fehler}</p>{/if}
{#if s.meldung}<p role="status">{s.meldung}</p>{/if}
{#if s.gerettet}<div class="hinweisbox">
    Ein lokaler Entwurf wurde nach der letzten Sitzung gefunden. <button
      class="link"
      onclick={() => s.wiederherstellen()}>Entwurf wiederherstellen</button
    ><button class="link" onclick={() => s.verwerfen()}
      >Entwurf verwerfen</button
    >
  </div>{/if}
<div class="knoepfe links">
  <button
    disabled={!darf || s.laeuft || s.hatAenderungen() || s.gerettet}
    onclick={() => s.neu()}>Neu anlegen</button
  ><button class="sekundaer" disabled={s.laeuft} onclick={() => s.laden()}
    >Liste aktualisieren</button
  >
</div>
<table class="ergebnistabelle">
  <thead><tr><th>Bezeichnung</th><th>Stand</th><th></th></tr></thead><tbody
    >{#each s.liste as f (f.objekt)}<tr
        ><td>{s.titel(f.daten)}</td><td>{f.revision}</td><td
          ><button
            class="link"
            disabled={s.laeuft || s.hatAenderungen() || s.gerettet}
            onclick={() => s.bearbeiten(f)}>Öffnen</button
          ><button class="link" onclick={() => historie(f.objekt)}
            >Verlauf</button
          ></td
        ></tr
      >{:else}<tr
        ><td colspan="3"
          >{s.laeuft ? "Wird geladen …" : "Noch keine Einträge vorhanden."}</td
        ></tr
      >{/each}</tbody
  >
</table>
{#if s.eingabe}<form
    onsubmit={async (e) => {
      e.preventDefault();
      await s.speichern();
    }}
  >
    <fieldset disabled={!darf || s.laeuft}>
      <legend
        >{s.revision === 0 ? "Neuer Eintrag" : "Eintrag bearbeiten"}</legend
      >{@render children(s.eingabe)}
    </fieldset>
    <div class="knoepfe links">
      <button
        disabled={!darf || s.laeuft || !(s.hatAenderungen() || s.gerettet)}
        >Speichern</button
      ><button type="button" class="sekundaer" onclick={() => s.verwerfen()}
        >{s.hatAenderungen() ? "Entwurf verwerfen" : "Schließen"}</button
      >
    </div>
  </form>{/if}

{#if verlauf.length}<details open>
    <summary>Gespeicherte Fachstände</summary>
    <table class="ergebnistabelle">
      <thead><tr><th>Stand</th><th>Bezeichnung</th></tr></thead><tbody
        >{#each verlauf as f (f.revision)}<tr
            ><td>{f.revision}</td><td>{s.titel(f.daten)}</td></tr
          >{/each}</tbody
      >
    </table>
    <button class="link" onclick={() => (verlauf = [])}
      >Verlauf schließen</button
    >
  </details>{/if}
