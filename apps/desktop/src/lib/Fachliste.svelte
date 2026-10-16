<script lang="ts" generics="T">
  import { plattformApi, type Datensatz } from "./plattformApi";
  import { fehlertext } from "./api";
  import { projektNavigation } from "./projektNavigation.svelte";
  import { onMount, untrack, type Snippet } from "svelte";
  import { editorAnmelden } from "./editoren";
  import type { Ansicht } from "./projektSitzung";
  import type { FachSitzung } from "./FachSitzung.svelte";
  let {
    titel,
    s,
    bereich,
    darf,
    children,
    lesendBedienbar = false,
    zusatzBeschaeftigt,
  }: {
    titel: string;
    s: FachSitzung<T>;
    bereich: Ansicht;
    darf: boolean;
    lesendBedienbar?: boolean;
    zusatzBeschaeftigt?: () => boolean;
    children: Snippet<[T]>;
  } = $props();
  const gesperrt = $derived(s.laeuft || (zusatzBeschaeftigt?.() ?? false));
  let verlauf = $state<Datensatz<T>[]>([]);
  async function historie(id: string) {
    try {
      verlauf = await plattformApi.verlauf<T>(s.projektId, s.art, id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  const navigation = projektNavigation();
  let sprung = 0;
  $effect(() => {
    const a = navigation?.auftrag;
    if (
      !a ||
      a.nummer === sprung ||
      a.ziel.bereich !== bereich ||
      !a.ziel.objekt ||
      !s.bereit ||
      gesperrt
    )
      return;
    untrack(() => {
      sprung = a.nummer;
      if (s.hatAenderungen() || s.gerettet) {
        if (s.id !== a.ziel.objekt)
          s.meldung =
            "Ein anderer Entwurf ist noch offen. Bitte zuerst speichern oder verwerfen; anschließend den Eintrag erneut aus der Projektübersicht öffnen.";
        return;
      }
      void s.laden().then((ok) => {
        if (!ok || navigation?.auftrag?.nummer !== a.nummer) return;
        if (s.hatAenderungen() || s.gerettet) return;
        const f = s.liste.find((f) => f.objekt === a.ziel.objekt);
        if (f) {
          s.meldung = null;
          s.bearbeiten(f);
        } else
          s.fehler =
            "Der verknüpfte Eintrag ist nicht mehr in dieser Projektliste vorhanden. Liste aktualisieren.";
      });
    });
  });
  onMount(() => s.laden());
  $effect(() => s.entwurfSichern());
  // Komponenten besitzen eine feste Route innerhalb ihrer Projektakte.
  // svelte-ignore state_referenced_locally
  editorAnmelden(bereich, {
    hatAenderungen: () => s.hatAenderungen(),
    istBeschaeftigt: () =>
      s.istBeschaeftigt() || (zusatzBeschaeftigt?.() ?? false),
    speichern: () =>
      zusatzBeschaeftigt?.() ? Promise.resolve(false) : s.speichern(),
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
    disabled={!darf || gesperrt || s.hatAenderungen() || s.gerettet}
    onclick={() => s.neu()}>Neu anlegen</button
  ><button class="sekundaer" disabled={gesperrt} onclick={() => s.laden()}
    >Liste aktualisieren</button
  >
</div>
<table class="ergebnistabelle">
  <thead><tr><th>Bezeichnung</th><th>Stand</th><th></th></tr></thead><tbody
    >{#each s.liste as f (f.objekt)}<tr
        ><td>{s.titel(f.daten)}</td><td>{f.revision}</td><td
          ><button
            class="link"
            disabled={gesperrt || s.hatAenderungen() || s.gerettet}
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
    class="fachformular"
    class:aktenformular={lesendBedienbar}
    onsubmit={async (e) => {
      e.preventDefault();
      if (!gesperrt) await s.speichern();
    }}
  >
    <fieldset disabled={(!darf && !lesendBedienbar) || gesperrt}>
      <legend
        >{s.revision === 0 ? "Neuer Eintrag" : "Eintrag bearbeiten"}</legend
      >{@render children(s.eingabe)}
    </fieldset>
    <div class="knoepfe links">
      <button
        disabled={!darf || gesperrt || !(s.hatAenderungen() || s.gerettet)}
        >Speichern</button
      ><button
        type="button"
        class="sekundaer"
        disabled={gesperrt}
        onclick={() => s.verwerfen()}
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

<style>
  .fachformular {
    display: block;
  }
  .fachformular > fieldset {
    width: 100%;
    margin-block: 1.5rem;
  }
  .aktenformular > fieldset {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    border: 0;
    margin: 0;
    padding: 0;
  }
  .aktenformular > .knoepfe {
    margin-top: 1rem;
  }
  td > .link + .link {
    margin-left: 0.75rem;
  }
</style>
