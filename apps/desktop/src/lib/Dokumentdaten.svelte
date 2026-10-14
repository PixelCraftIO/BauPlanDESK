<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { DocumentVersion } from "./api";
  import type { Dokumentdaten } from "./generated/plattform";
  import { FachSitzung } from "./FachSitzung.svelte";
  import { editorAnmelden } from "./editoren";
  let {
    dokument,
    benutzerId,
    darf,
    onschliessen,
    onstatus,
  }: {
    dokument: DocumentVersion;
    benutzerId: string;
    darf: boolean;
    onschliessen: () => void;
    onstatus: (dirty: boolean, busy: boolean) => void;
  } = $props();
  const s = untrack(
    () =>
      new FachSitzung<Dokumentdaten>(
        dokument.project_id,
        "dokumentdaten",
        benutzerId,
        () => ({
          dokument: {
            dokument_id: dokument.document_id,
            version: dokument.version,
          },
          dokumentart: "",
          plannummer: "",
          revisionsindex: "",
          massstab: "",
          verfasser: "",
          datum: null,
        }),
        (d) => d.plannummer || d.dokumentart,
        dokument.document_id,
      ),
  );
  onMount(async () => {
    await s.laden();
    if (s.gerettet) return;
    const f = s.liste.find((f) => f.objekt === dokument.document_id);
    if (f) {
      s.bearbeiten(f);
      if (s.eingabe)
        s.eingabe.dokument = {
          dokument_id: dokument.document_id,
          version: dokument.version,
        };
    } else {
      s.neu();
      s.id = dokument.document_id;
      s.original = structuredClone(s.leer());
    }
  });
  $effect(() => {
    s.entwurfSichern();
    onstatus(s.hatAenderungen(), s.laeuft);
  });
  editorAnmelden("dokumente", {
    hatAenderungen: () => s.hatAenderungen(),
    istBeschaeftigt: () => s.laeuft,
    speichern: () => s.speichern(),
  });
</script>

<p class="meta">
  {dokument.file_name} · Version {dokument.version}. Angaben sind diesem
  Dokumentstand zugeordnet und besitzen eine eigene Änderungshistorie.
</p>
{#if s.fehler}<p class="fehlertext" role="alert">{s.fehler}</p>{/if}
{#if s.meldung}<p role="status">{s.meldung}</p>{/if}
{#if s.gerettet}<div class="hinweisbox">
    Ein nicht gespeicherter Metadatenentwurf ist vorhanden.<button
      class="link"
      onclick={() => s.wiederherstellen()}>Entwurf wiederherstellen</button
    >
  </div>{/if}
{#if s.eingabe}<form
    onsubmit={async (e) => {
      e.preventDefault();
      await s.speichern();
    }}
  >
    <fieldset disabled={!darf || s.laeuft}>
      <legend>Dokumentangaben</legend>
      <div class="raster">
        <label
          >Dokumentart<input
            bind:value={s.eingabe.dokumentart}
            required
            maxlength="120"
          /></label
        ><label
          >Plannummer<input
            bind:value={s.eingabe.plannummer}
            maxlength="500"
          /></label
        ><label
          >Revisionsindex<input
            bind:value={s.eingabe.revisionsindex}
            maxlength="500"
          /></label
        ><label
          >Maßstab<input
            bind:value={s.eingabe.massstab}
            maxlength="500"
          /></label
        ><label
          >Verfasser<input
            bind:value={s.eingabe.verfasser}
            maxlength="500"
          /></label
        ><label
          >Dokumentdatum<input
            type="date"
            value={s.eingabe.datum ?? ""}
            onchange={(e) => {
              if (s.eingabe) s.eingabe.datum = e.currentTarget.value || null;
            }}
          /></label
        >
      </div>
    </fieldset>
    <div class="knoepfe">
      <button disabled={!darf || s.laeuft || !s.hatAenderungen()}
        >Speichern</button
      ><button
        type="button"
        class="sekundaer"
        disabled={s.laeuft}
        onclick={onschliessen}>Schließen</button
      >
    </div>
  </form>{/if}
