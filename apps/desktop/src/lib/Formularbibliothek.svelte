<script lang="ts">
  import { onMount } from "svelte";
  import { open, ask, save } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    mindestens,
    type Benutzer,
    type Formularbibliothek as Bibliothek,
    type BibliotheksVorlage,
    type BibliothekVorschau,
    type Project,
    type DocumentVersion,
  } from "./api";
  import Dialog from "./Dialog.svelte";
  import PdfVorschau from "./PdfVorschau.svelte";
  import FormularAusfuellen from "./FormularAusfuellen.svelte";
  import PdfVorlagenEditor from "./PdfVorlagenEditor.svelte";
  let {
    ich,
    onprojektAenderung,
  }: { ich: Benutzer; onprojektAenderung?: (id: string) => void } = $props();
  let katalog = $state<Bibliothek | null>(null),
    projekte = $state<Project[]>([]),
    projektId = $state("");
  let vorschauSeite = $state(1);
  let auswahl = $state<BibliotheksVorlage | null>(null),
    vorschau = $state<BibliothekVorschau | null>(null);
  let fehler = $state<string | null>(null),
    hinweis = $state<string | null>(null),
    laeuft = $state(false);
  let formular = $state<DocumentVersion | null>(null),
    editor = $state<DocumentVersion | null>(null),
    dirty = $state(false),
    pdfBusy = $state(false);
  const darfAblegen = $derived(mindestens(ich.rolle, "bearbeitung"));
  export function istBeschaeftigt() {
    return laeuft || !!formular || !!editor;
  }
  async function laden() {
    laeuft = true;
    fehler = null;
    try {
      [katalog, projekte] = await Promise.all([
        api.formularbibliothek(),
        api.projekte(),
      ]);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  onMount(laden);
  async function waehlen() {
    const pfad = await open({
      directory: true,
      multiple: false,
      title: "Formularordner auswählen",
      defaultPath: katalog?.root,
    });
    if (typeof pfad !== "string") return;
    if (
      !(await ask(
        `Diesen Ordner als Formularbibliothek verbinden?\n${pfad}\n\nBisherige Vorlagen bleiben im bisherigen Ordner. Es werden keine Dateien verschoben.`,
        { title: "Formularordner", kind: "info" },
      ))
    )
      return;
    laeuft = true;
    fehler = null;
    try {
      await api.formularordnerWaehlen(pfad);
      auswahl = null;
      vorschau = null;
      await laden();
      hinweis = "Formularordner verbunden.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function importieren() {
    const pfad = await open({
      multiple: false,
      title: "PDF-Vorlage importieren",
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (typeof pfad !== "string") return;
    laeuft = true;
    fehler = null;
    try {
      await api.bibliothekImportieren(pfad);
      await laden();
      hinweis = "PDF-Vorlage importiert. Die Originaldatei bleibt erhalten.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function ansehen(v: BibliotheksVorlage) {
    laeuft = true;
    fehler = null;
    auswahl = v;
    vorschauSeite = 1;
    vorschau = null;
    try {
      vorschau = await api.bibliothekVorschau(v);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function uebernehmen() {
    if (!auswahl || !projektId || !vorschau) return;
    laeuft = true;
    fehler = null;
    try {
      const d = await api.bibliothekUebernehmen(auswahl, projektId);
      onprojektAenderung?.(d.project_id);
      dirty = false;
      if (vorschau.felder.length) formular = d;
      else editor = d;
      hinweis =
        "Vorlage als eigenständiges Dokument in das gewählte Projekt übernommen.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function schliessenErlaubt() {
    return (
      !pdfBusy &&
      (!dirty ||
        (await ask("Ungespeicherte PDF-Eingaben verwerfen?", {
          title: "PDF-Formular",
          kind: "warning",
        })))
    );
  }
  async function abbrechen() {
    if (await schliessenErlaubt()) {
      formular = null;
      editor = null;
      dirty = false;
    }
  }
  async function fertig(d: DocumentVersion) {
    onprojektAenderung?.(d.project_id);
    formular = null;
    dirty = false;
    laeuft = true;
    hinweis = "Ausgefüllte PDF im gewählten Projekt gespeichert.";
    try {
      const ziel = await save({
        title: "Ausgefüllte PDF exportieren",
        defaultPath: d.file_name,
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
      if (ziel) await api.dokumentExportieren(d.document_id, ziel, d.version);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

<h1>Formularbibliothek</h1>
<p class="meta">
  Projektunabhängige PDF-Vorlagen für alle Bauvorhaben. Beim Ausfüllen wird eine
  Kopie in der gewählten Projektakte angelegt.
</p>
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
{#if hinweis}<p role="status">{hinweis}</p>{/if}
{#if katalog}<dl class="daten">
    <dt>Formularordner</dt>
    <dd>{katalog.root}</dd>
  </dl>{/if}
<div class="knoepfe links">
  <button
    class="sekundaer"
    disabled={laeuft || !mindestens(ich.rolle, "administrator")}
    onclick={waehlen}>Formularordner auswählen …</button
  >
  <button disabled={laeuft || !darfAblegen} onclick={importieren}
    >PDF-Vorlage importieren …</button
  >
  <button
    class="sekundaer"
    disabled={laeuft}
    onclick={async () => {
      auswahl = null;
      vorschau = null;
      await laden();
    }}>Aktualisieren</button
  >
</div>
<p class="meta">
  Zum Beispiel <code>{"C:\\BauProjektIO\\Formulare"}</code> oder eine eingebundene
  Serverfreigabe. Für eine gemeinsame Bibliothek verbinden alle Arbeitsplätze denselben
  Ordner. Neue Speicherorte verschieben keine bestehenden Vorlagen.
</p>
{#if katalog && !katalog.vorlagen.length}<p>
    Noch keine PDF-Vorlagen vorhanden. Importieren Sie eine PDF oder legen Sie
    sie direkt im Formularordner ab.
  </p>{/if}
{#if katalog?.vorlagen.length}<table>
    <thead><tr><th>PDF-Vorlage</th><th>Größe</th><th></th></tr></thead><tbody>
      {#each katalog.vorlagen as v (v.name)}<tr
          ><td>{v.name}</td><td>{Math.ceil(v.size / 1024)} KB</td><td
            ><button class="link" disabled={laeuft} onclick={() => ansehen(v)}
              >Vorlage ansehen</button
            ></td
          ></tr
        >{/each}
    </tbody>
  </table>{/if}
{#if auswahl && vorschau}
  <h2>{auswahl.name}</h2>
  <p class="meta">
    {vorschau.felder.length} Eingabefelder · {vorschau.seiten.length} Seiten. Ohne
    Eingabefelder können Sie nach dem Übernehmen eigene Felder platzieren.
  </p>
  <div class="knoepfe links">
    <label
      >Projekt<select aria-label="Projekt" bind:value={projektId}
        ><option value="">Projekt auswählen</option>{#each projekte as p}<option
            value={p.id}>{p.name}</option
          >{/each}</select
      ></label
    ><button
      disabled={laeuft || !projektId || !darfAblegen}
      onclick={uebernehmen}>Im Projekt verwenden …</button
    >
  </div>
  <label
    >Vorschauseite<select aria-label="Vorschauseite" bind:value={vorschauSeite}
      >{#each vorschau.seiten as p}<option value={p.nummer}>{p.nummer}</option
        >{/each}</select
    ></label
  >
  {#key vorschau.pdf_base64}<PdfVorschau
      pdfBase64={vorschau.pdf_base64}
      seite={vorschauSeite}
      onfehler={(m) => (fehler = m)}
    />{/key}
{/if}
{#if formular}<Dialog
    titel="Formular ausfüllen"
    breit
    onrequestclose={schliessenErlaubt}
    onclose={() => (formular = null)}
    ><FormularAusfuellen
      dokumentId={formular.document_id}
      onfertig={fertig}
      onabbrechen={abbrechen}
      onstatus={(d, b) => {
        dirty = d;
        pdfBusy = b;
      }}
    /></Dialog
  >{/if}
{#if editor}<Dialog
    titel="Eigene PDF-Felder und Vorlage"
    breit
    onrequestclose={schliessenErlaubt}
    onclose={() => (editor = null)}
    ><PdfVorlagenEditor
      dokumentId={editor.document_id}
      onfertig={(d) => {
        onprojektAenderung?.(d.project_id);
        editor = null;
        dirty = false;
        formular = d;
      }}
      onabbrechen={abbrechen}
      onstatus={(d, b) => {
        dirty = d;
        pdfBusy = b;
      }}
    /></Dialog
  >{/if}

<style>
  table {
    width: 100%;
  }
  td:first-child {
    overflow-wrap: anywhere;
  }
</style>
