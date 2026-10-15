<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api, fehlertext, type Benutzer } from "./api";
  import type { Projektorganisation } from "./generated/plattform";
  import { FachSitzung } from "./FachSitzung.svelte";
  import { editorAnmelden } from "./editoren";
  let {
    projektId,
    benutzerId,
    darf,
  }: { projektId: string; benutzerId: string; darf: boolean } = $props();
  const s = untrack(
    () =>
      new FachSitzung<Projektorganisation>(
        projektId,
        "projektorganisation",
        benutzerId,
        () => ({
          kennzeichen: "",
          phase: "vorbereitung",
          verantwortlich: null,
          beschreibung: "",
        }),
        (o) => o.kennzeichen,
      ),
  );
  let benutzer = $state<Benutzer[]>([]);
  onMount(async () => {
    try {
      benutzer = await api.benutzerAuflisten();
      await s.laden();
      if (!s.bereit || s.gerettet) return;
      const f = s.liste.find((f) => f.objekt === projektId);
      if (f) s.bearbeiten(f);
      else {
        s.neu();
        s.id = projektId;
        s.original = s.leer();
      }
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
  $effect(() => s.entwurfSichern());
  editorAnmelden("uebersicht", {
    hatAenderungen: () => s.hatAenderungen(),
    istBeschaeftigt: () => s.laeuft,
    speichern: () => s.speichern(),
  });
</script>

<h3>Projektorganisation</h3>
{#if s.fehler}<p class="fehlertext" role="alert">
    {s.fehler}
  </p>{/if}{#if s.meldung}<p role="status">{s.meldung}</p>{/if}
{#if s.gerettet}<p class="hinweisbox">
    Ein ungespeicherter Organisationsentwurf wurde gefunden. <button
      class="link"
      onclick={() => s.wiederherstellen()}>Entwurf wiederherstellen</button
    ><button
      class="link"
      onclick={() => {
        s.verwerfen();
        const f = s.liste.find((f) => f.objekt === projektId);
        if (f) s.bearbeiten(f);
        else {
          s.neu();
          s.id = projektId;
          s.original = s.leer();
        }
      }}>Entwurf verwerfen</button
    >
  </p>{/if}
{#if s.eingabe}<form
    onsubmit={async (e) => {
      e.preventDefault();
      await s.speichern();
    }}
  >
    <fieldset disabled={!darf || s.laeuft}>
      <legend>Interne Angaben</legend>
      <div class="raster">
        <label
          >Projektkennzeichen<input
            bind:value={s.eingabe.kennzeichen}
            maxlength="120"
          /></label
        ><label
          >Projektphase<select
            aria-label="Projektphase"
            bind:value={s.eingabe.phase}
            ><option value="vorbereitung">Vorbereitung</option><option
              value="vorplanung">Vorplanung</option
            ><option value="entwurf">Entwurf</option><option value="genehmigung"
              >Genehmigung</option
            ><option value="ausfuehrung">Ausführung</option><option
              value="abgeschlossen">Abgeschlossen</option
            ></select
          ></label
        ><label
          >Projektverantwortlich<select
            aria-label="Projektverantwortlich"
            bind:value={s.eingabe.verantwortlich}
            ><option value={null}>Noch nicht zugeordnet</option
            >{#each benutzer.filter((b) => b.aktiv) as b}<option value={b.id}
                >{b.anzeigename}</option
              >{/each}</select
          ></label
        >
      </div>
      <label
        >Kurzbeschreibung<textarea
          bind:value={s.eingabe.beschreibung}
          maxlength="10000"
        ></textarea></label
      >
    </fieldset>
    <button disabled={!darf || s.laeuft || !s.hatAenderungen()}
      >Projektorganisation speichern</button
    >
  </form>{/if}
