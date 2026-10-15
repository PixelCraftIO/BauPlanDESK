<script lang="ts">
  import { onMount, untrack } from "svelte";
  import {
    api,
    fehlertext,
    type DocumentVersion,
    type Dokumentkategorie,
  } from "./api";
  import KategorieAuswahl from "./KategorieAuswahl.svelte";
  import type { Dokumentdaten } from "./generated/plattform";
  import { FachSitzung } from "./FachSitzung.svelte";
  import { editorAnmelden } from "./editoren";
  let {
    dokument,
    benutzerId,
    darf,
    onschliessen,
    onstatus,
    ongeaendert,
  }: {
    dokument: DocumentVersion;
    benutzerId: string;
    darf: boolean;
    onschliessen: () => void;
    onstatus: (dirty: boolean, busy: boolean) => void;
    ongeaendert: () => Promise<void>;
  } = $props();
  let kategorien = $state<Dokumentkategorie[]>([]),
    kategorie = $state(""),
    originalKategorie = $state(""),
    kategorieRevision = $state(0);
  let kategorieBereit = $state(false),
    kategorieBusy = $state(false),
    kategorieFehler = $state(""),
    kategorieMeldung = $state("");
  const kategorieDirty = $derived(
    kategorieBereit && kategorie !== originalKategorie,
  );
  async function kategorieLaden() {
    kategorieBusy = true;
    try {
      const [k, z] = await Promise.all([
        api.kategorienkatalog(),
        api.dokumentkategorien(dokument.project_id),
      ]);
      kategorien = k.kategorien;
      const stand = z.find((d) => d.dokument_id === dokument.document_id);
      kategorie = stand?.kategorie ?? "";
      originalKategorie = kategorie;
      kategorieRevision = stand?.revision ?? 0;
      kategorieBereit = true;
      kategorieFehler = "";
    } catch (e) {
      kategorieFehler = fehlertext(e);
    } finally {
      kategorieBusy = false;
    }
  }
  async function speichern() {
    if (kategorieBusy || s.laeuft) return false;
    if (kategorieDirty) {
      kategorieBusy = true;
      kategorieFehler = "";
      kategorieMeldung = "";
      try {
        const stand = await api.dokumentkategorieSpeichern(
          dokument.document_id,
          { revision: kategorieRevision, kategorie: kategorie || null },
        );
        kategorieRevision = stand.revision;
        originalKategorie = stand.kategorie ?? "";
        kategorie = originalKategorie;
        kategorieMeldung = "Kategorie gespeichert.";
        await ongeaendert();
      } catch (e) {
        kategorieFehler = fehlertext(e);
        return false;
      } finally {
        kategorieBusy = false;
      }
    }
    return !s.hatAenderungen() || (await s.speichern());
  }
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
    await Promise.all([s.laden(), kategorieLaden()]);
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
    onstatus(s.hatAenderungen() || kategorieDirty, s.laeuft || kategorieBusy);
  });
  editorAnmelden("dokumente", {
    hatAenderungen: () => s.hatAenderungen() || kategorieDirty,
    istBeschaeftigt: () => s.laeuft || kategorieBusy,
    speichern,
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
      await speichern();
    }}
  >
    <fieldset disabled={!darf || s.laeuft || kategorieBusy || !kategorieBereit}>
      <legend>Fachliche Kategorie</legend>
      <KategorieAuswahl {kategorien} bind:wert={kategorie} />
      <p class="meta">
        Gilt für das Dokument und seine Folgeversionen. Änderungen erzeugen
        keine neue Dateiversion.
      </p>
    </fieldset>
    {#if kategorieFehler}<p role="alert" class="fehlertext">
        {kategorieFehler}
      </p>
      <button
        type="button"
        class="sekundaer"
        disabled={kategorieBusy}
        onclick={kategorieLaden}>Kategorie neu laden</button
      >{/if}
    {#if kategorieMeldung}<p role="status">{kategorieMeldung}</p>{/if}
    <fieldset disabled={!darf || s.laeuft || kategorieBusy}>
      <legend>Dokumentangaben</legend>
      <div class="raster">
        <label
          >Dokumentart<input
            bind:value={s.eingabe.dokumentart}
            required={s.hatAenderungen()}
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
      <button
        disabled={!darf ||
          s.laeuft ||
          kategorieBusy ||
          (!s.hatAenderungen() && !kategorieDirty)}>Speichern</button
      ><button
        type="button"
        class="sekundaer"
        disabled={s.laeuft || kategorieBusy}
        onclick={onschliessen}>Schließen</button
      >
    </div>
  </form>{/if}
