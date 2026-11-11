<script lang="ts">
  import { projektNavigation } from "./projektNavigation.svelte";
  import { untrack } from "svelte";
  import { editorAnmelden } from "./editoren";

  import { ask } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    type BibliotheksVorlage,
    type DocumentVersion,
    type ProjektFormular,
    type Werksvorlage,
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
    onbibliothek,
    aktiv = true,
  }: {
    aktiv?: boolean;
    onbibliothek?: () => void;
    projekt: Project;
    darf: boolean;
    ongespeichert: () => Promise<void>;
    ondateien: () => void;
  } = $props();
  const navigation = projektNavigation();
  $effect(() => {
    if (aktiv) untrack(() => void laden());
    return () => {
      ladeGeneration++;
      laeuft = false;
    };
  });
  let ladeGeneration = 0;
  let vorlagen = $state<BibliotheksVorlage[]>([]),
    gewaehlt = $state<BibliotheksVorlage | null>(null),
    suche = $state(""),
    laeuft = $state(false),
    dirty = $state(false),
    pdfBusy = $state(false),
    fehler = $state<string | null>(null),
    gespeichert = $state<DocumentVersion | null>(null);
  let ausgefuellt = $state<ProjektFormular[]>([]),
    werksvorlagen = $state<Werksvorlage[]>([]),
    bearbeiten = $state<ProjektFormular | null>(null);
  /** Mitgelieferte Vordrucke für das Bundesland des Projekts, die noch fehlen. */
  const fehlendeWerksvorlagen = $derived(
    werksvorlagen.filter(
      (w) =>
        !w.vorhanden &&
        w.land.toLowerCase() === String(projekt.bundesland).toLowerCase(),
    ),
  );
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
    return laeuft || !!gewaehlt || !!bearbeiten;
  }
  async function laden() {
    const generation = ++ladeGeneration;
    laeuft = true;
    try {
      // Der Abgleich der ausgefüllten Formulare rechnet die Projektquellen
      // durch und hält die Vorlagenliste nicht auf.
      void api
        .projektFormulare(projekt.id)
        .then((a) => {
          if (generation === ladeGeneration) ausgefuellt = a;
        })
        .catch(() => {
          if (generation === ladeGeneration) ausgefuellt = [];
        });
      const [b, k] = await Promise.all([
        api.formularbibliothek(),
        api.kategorienkatalog(),
      ]);
      const z = await Promise.all(
        [...new Set(b.vorlagen.map((v) => v.sha256))].map(
          async (hash) => [hash, await api.vorlagenkategorie(hash)] as const,
        ),
      );
      if (generation !== ladeGeneration) return;
      fehler = null;
      vorlagen = b.vorlagen;
      werksvorlagen = b.werksvorlagen ?? [];
      kategorien = k.kategorien;
      zuordnungen = Object.fromEntries(z);
    } catch (e) {
      if (generation === ladeGeneration) fehler = fehlertext(e);
    } finally {
      if (generation === ladeGeneration) laeuft = false;
    }
  }

  async function werksvorlagenUebernehmen() {
    laeuft = true;
    try {
      await api.werksvorlagenUebernehmen();
    } catch (e) {
      fehler = fehlertext(e);
    }
    laeuft = false;
    await laden();
  }

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
    bearbeiten = null;
    dirty = false;
    void laden();
    try {
      await ongespeichert();
    } catch (e) {
      fehler = `Formular gespeichert. Die Projektansicht konnte nicht aktualisiert werden: ${fehlertext(e)}`;
    }
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
{#if navigation?.unterlagenauftrag}<aside class="hinweisbox">
    <strong>Unterlage für: {navigation.unterlagenauftrag.titel}</strong>
    <p>
      Vorlage ausfüllen. Nach dem Speichern die konkrete Version im
      Ausgangsbereich zuordnen.
    </p>
    <button
      type="button"
      class="link"
      onclick={() => {
        if (navigation?.unterlagenauftrag)
          navigation.springen(navigation.unterlagenauftrag.ziel);
      }}>Zurück zur Anforderung →</button
    >
  </aside>{/if}
{#if onbibliothek}<button
    type="button"
    class="sekundaer"
    disabled={laeuft || !!gewaehlt}
    onclick={onbibliothek}
    >Vorlagen hinzufügen / Eingabefelder einrichten …</button
  >{/if}
{#if ausgefuellt.length}<h3>Ausgefüllte Formulare dieses Projekts</h3>
  <table class="ergebnistabelle ausgefuellt">
    <thead
      ><tr
        ><th>Formular</th><th>Stand</th><th>Abgleich mit den Projektdaten</th
        ><th></th></tr
      ></thead
    ><tbody
      >{#each ausgefuellt as a (a.dokument_id)}<tr
          ><td
            >{a.datei}<span class="meta"> · aus „{a.vorlage}“</span></td
          ><td
            >Version {a.version} · {new Date(a.erstellt).toLocaleDateString(
              "de-DE",
            )}</td
          ><td
            >{#if a.abweichungen === null}<span class="meta"
                >nicht prüfbar</span
              >{:else if a.abweichungen.length}<span
                class="warnung"
                title={a.abweichungen.join(", ")}
                >{a.abweichungen.length}
                {a.abweichungen.length === 1 ? "Feld weicht" : "Felder weichen"} ab</span
              >{:else}aktuell{/if}</td
          ><td
            ><button
              class="link"
              disabled={!darf || laeuft}
              onclick={() => {
                dirty = false;
                gespeichert = null;
                bearbeiten = a;
              }}
              >{a.abweichungen?.length
                ? "Aktualisieren …"
                : "Bearbeiten …"}</button
            ></td
          ></tr
        >{/each}</tbody
    >
  </table>
  <h3>Neues Formular aus einer Vorlage</h3>{/if}
{#if fehlendeWerksvorlagen.length}<aside class="hinweisbox werksvorlagen">
    <strong
      >Für dieses Bundesland liegen {fehlendeWerksvorlagen.length} amtliche
      {fehlendeWerksvorlagen.length === 1 ? "Vordruck" : "Vordrucke"} bei</strong
    >
    <p>
      {fehlendeWerksvorlagen.map((w) => w.titel).join(" · ")}. Die Felder sind
      bereits mit den Projektdaten verknüpft.
    </p>
    <button disabled={!darf || laeuft} onclick={werksvorlagenUebernehmen}
      >Mitgelieferte Vordrucke übernehmen</button
    >
  </aside>{/if}
<p class="meta">
  Vorlage auswählen, ausfüllen und im Projekt „{projekt.name}“ speichern.
</p>
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
{#if gespeichert}<div class="hinweisbox" role="status">
    <strong>{gespeichert.file_name}</strong> wurde im Projekt gespeichert. {gespeichert.projektordner_pfad ??
      ""}
    {gespeichert.ablage_hinweis ?? ""}
    {#if navigation?.unterlagenauftrag}<button
        type="button"
        onclick={() => {
          if (navigation?.unterlagenauftrag && gespeichert) {
            navigation.unterlagenauftrag.dokument = {
              dokument_id: gespeichert.document_id,
              version: gespeichert.version,
            };
            navigation.springen(navigation.unterlagenauftrag.ziel);
          }
        }}>Diese Version zur Zuordnung übernehmen →</button
      >{/if}
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
    ><option value="">Alle Kategorien</option><option value=":ohne"
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
{#if bearbeiten}<Dialog
    titel={`Formular bearbeiten · ${bearbeiten.datei}`}
    breit
    onrequestclose={schliessenErlaubt}
    onclose={() => (bearbeiten = null)}
    ><FormularAusfuellen
      dokumentId={bearbeiten.dokument_id}
      onfertig={fertig}
      onabbrechen={async () => {
        if (await schliessenErlaubt()) {
          bearbeiten = null;
          dirty = false;
        }
      }}
      onstatus={(d, b) => {
        dirty = d;
        pdfBusy = b;
      }}
    /></Dialog
  >{/if}
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
