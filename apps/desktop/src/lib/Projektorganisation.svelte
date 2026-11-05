<script lang="ts">
  import { abonnieren } from "./projektEreignisse";
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
  // „Nicht benötigt“ im Projektbaum speichert ebenfalls hier: ohne eigene
  // Änderungen den neuen Stand übernehmen, damit kein Versionskonflikt entsteht.
  // Hat das Formular eigene Änderungen und hat sich am Server nur diese
  // Kennzeichnung geändert, wird sie samt Revision übernommen; die eigenen
  // Eingaben bleiben stehen und lassen sich ohne Konflikt speichern.
  const ohneKennzeichen = (o: Projektorganisation | null) =>
    o && JSON.stringify({ ...o, nicht_benoetigt: undefined });
  $effect(() =>
    abonnieren(projektId, ["fachobjekte"], async () => {
      if (!s.bereit || s.laeuft) return;
      if (!s.hatAenderungen()) {
        await s.laden();
        const f = s.liste.find((f) => f.objekt === projektId);
        if (f && f.revision !== s.revision) s.bearbeiten(f);
        return;
      }
      const vorher = ohneKennzeichen(s.original);
      await s.laden(); // lässt Eingabe und Original bei offenen Änderungen stehen
      const f = s.liste.find((f) => f.objekt === projektId);
      if (!f || f.revision === s.revision || !s.eingabe || !s.original) return;
      if (ohneKennzeichen(f.daten) !== vorher) return;
      const kennzeichen = f.daten.nicht_benoetigt;
      s.revision = f.revision;
      s.original = { ...s.original, nicht_benoetigt: kennzeichen };
      s.eingabe = { ...s.eingabe, nicht_benoetigt: kennzeichen };
    }),
  );
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
