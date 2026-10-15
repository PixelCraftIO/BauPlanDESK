<script lang="ts">
  import { editorAnmelden } from "./editoren";
  import { onMount } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    type BibliotheksVorlage,
    type DocumentVersion,
    type Project,
  } from "./api";
  import {
    kategorieText,
    kategoriePasst,
    kategorienOptionen,
  } from "./kategorien";
  import type { Dokumentkategorie, KategorieZuordnung } from "./api";
  import Dialog from "./Dialog.svelte";
  import FormularAusfuellen from "./FormularAusfuellen.svelte";
  let {
    projekt,
    darf,
    ongespeichert,
    ondateien,
  }: {
    projekt: Project;
    darf: boolean;
    ongespeichert: () => Promise<void>;
    ondateien: () => void;
  } = $props();
  let vorlagen = $state<BibliotheksVorlage[]>([]),
    gewaehlt = $state<BibliotheksVorlage | null>(null),
    suche = $state(""),
    laeuft = $state(false),
    dirty = $state(false),
    pdfBusy = $state(false),
    fehler = $state<string | null>(null),
    gespeichert = $state<DocumentVersion | null>(null);
  let kategorien = $state<Dokumentkategorie[]>([]),
    zuordnungen = $state<Record<string, KategorieZuordnung>>({}),
    kategorieFilter = $state("");
  const gefiltert = $derived(
    vorlagen.filter(
      (v) =>
        kategoriePasst(
          kategorien,
          zuordnungen[v.sha256]?.kategorie,
          kategorieFilter,
        ) &&
        v.name.toLocaleLowerCase("de").includes(suche.toLocaleLowerCase("de")),
    ),
  );
  export function istBeschaeftigt() {
    return laeuft || !!gewaehlt;
  }
  async function laden() {
    laeuft = true;
    try {
      const [b, k] = await Promise.all([
        api.formularbibliothek(),
        api.kategorienkatalog(),
      ]);
      const z = await Promise.all(
        [...new Set(b.vorlagen.map((v) => v.sha256))].map(
          async (hash) => [hash, await api.vorlagenkategorie(hash)] as const,
        ),
      );
      vorlagen = b.vorlagen;
      kategorien = k.kategorien;
      zuordnungen = Object.fromEntries(z);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  onMount(laden);
  async function schliessenErlaubt() {
    return (
      !pdfBusy &&
      (!dirty ||
        (await ask(
          "Formulareingaben verwerfen? Im Projekt wurde noch keine Datei angelegt.",
          {
            title: "Formular schließen",
            kind: "warning",
            okLabel: "Verwerfen",
            cancelLabel: "Weiter ausfüllen",
          },
        )))
    );
  }
  async function abbrechen() {
    if (await schliessenErlaubt()) {
      gewaehlt = null;
      dirty = false;
    }
  }
  async function fertig(d: DocumentVersion) {
    gespeichert = d;
    gewaehlt = null;
    dirty = false;
    await ongespeichert();
  }
  editorAnmelden("formulare", {
    hatAenderungen: () => false,
    istBeschaeftigt: istBeschaeftigt,
    speichern: async () => true,
    hinweis:
      "Einreichungsauswahl zuerst als Paket festhalten oder ausdrücklich verwerfen.",
  });
</script>

<h2>Formulare</h2>
<p class="meta">
  Vorlage auswählen, ausfüllen und im Projekt „{projekt.name}“ speichern.
</p>
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
{#if gespeichert}<div class="hinweisbox" role="status">
    <strong>{gespeichert.file_name}</strong> wurde im Projekt gespeichert. {gespeichert.projektordner_pfad ??
      ""}
    {gespeichert.ablage_hinweis ?? ""}
    <button class="link" onclick={ondateien}>Zu den Projektdateien</button>
  </div>{/if}
<div class="formular-auswahlkopf">
  <label
    >Vorlage suchen<input
      type="search"
      bind:value={suche}
      placeholder="Name der PDF-Vorlage"
    /></label
  ><button class="sekundaer" disabled={laeuft} onclick={laden}
    >Bibliothek aktualisieren</button
  >
</div>
<label
  >Kategorie filtern<select
    aria-label="Kategorie filtern"
    bind:value={kategorieFilter}
    ><option value="">Alle Kategorien</option><option value="__ohne"
      >Ohne Kategorie</option
    >{#each kategorienOptionen(kategorien) as k}<option value={k.id}
        >{k.bezeichnung}</option
      >{/each}</select
  ></label
>
<table class="ergebnistabelle">
  <thead><tr><th>Formularvorlage</th><th>Kategorie</th><th></th></tr></thead
  ><tbody
    >{#each gefiltert as v (v.name)}<tr
        ><td
          >{v.name}{#if v.felder === 0}<span class="meta">
              · Noch keine Eingabefelder</span
            >{/if}</td
        ><td>{kategorieText(kategorien, zuordnungen[v.sha256]?.kategorie)}</td
        ><td
          ><button
            disabled={!darf || laeuft || v.felder === 0 || v.felder === null}
            onclick={() => {
              dirty = false;
              gespeichert = null;
              gewaehlt = v;
            }}>Ausfüllen</button
          ></td
        ></tr
      >{/each}</tbody
  >
</table>
{#if !vorlagen.length && !laeuft}<p>
    Noch keine Vorlagen vorhanden. Vorlagen werden zentral unter
    „Formularbibliothek“ verwaltet.
  </p>{/if}
{#if gewaehlt}<Dialog
    titel={`Formular ausfüllen · ${projekt.name}`}
    breit
    onrequestclose={schliessenErlaubt}
    onclose={() => (gewaehlt = null)}
    ><FormularAusfuellen
      quelle={{ projektId: projekt.id, vorlage: gewaehlt }}
      onfertig={fertig}
      onabbrechen={abbrechen}
      onstatus={(d, b) => {
        dirty = d;
        pdfBusy = b;
      }}
    /></Dialog
  >{/if}

<style>
  .formular-auswahlkopf {
    display: flex;
    align-items: end;
    gap: 1rem;
    margin: 1.2rem 0;
  }
  .formular-auswahlkopf label {
    flex: 1;
    max-width: 36rem;
  }
  .hinweisbox {
    margin: 1rem 0;
  }
</style>
