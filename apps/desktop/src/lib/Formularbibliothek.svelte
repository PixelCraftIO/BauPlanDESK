<script lang="ts">
  import { onMount } from "svelte";
  import { open, ask } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    mindestens,
    type Benutzer,
    type Formularbibliothek as Bibliothek,
    type BibliotheksVorlage,
    type BibliothekVorschau,
  } from "./api";
  import KategorieAuswahl from "./KategorieAuswahl.svelte";
  import { kategorieText } from "./kategorien";
  import type { Dokumentkategorie, KategorieZuordnung } from "./api";
  import Dialog from "./Dialog.svelte";
  import PdfVorschau from "./PdfVorschau.svelte";
  import PdfVorlagenEditor from "./PdfVorlagenEditor.svelte";
  let { ich }: { ich: Benutzer } = $props();
  let katalog = $state<Bibliothek | null>(null),
    auswahl = $state<BibliotheksVorlage | null>(null),
    vorschau = $state<BibliothekVorschau | null>(null),
    editor = $state<BibliotheksVorlage | null>(null),
    fehler = $state<string | null>(null),
    hinweis = $state<string | null>(null),
    laeuft = $state(false),
    dirty = $state(false),
    pdfBusy = $state(false),
    seite = $state(1);
  let kategorien = $state<Dokumentkategorie[]>([]),
    zuordnungen = $state<Record<string, KategorieZuordnung>>({});
  let kategorieVorlage = $state<BibliotheksVorlage | null>(null),
    kategorieWert = $state("");
  async function kategorieSpeichern(e: SubmitEvent) {
    e.preventDefault();
    if (!kategorieVorlage) return;
    laeuft = true;
    fehler = null;
    const hash = kategorieVorlage.sha256;
    try {
      const v = await api.vorlagenkategorieSpeichern(hash, {
        revision: zuordnungen[hash]?.revision ?? 0,
        kategorie: kategorieWert || null,
      });
      zuordnungen = { ...zuordnungen, [hash]: v };
      kategorieVorlage = null;
      hinweis =
        "Standardkategorie der Vorlage gespeichert. Bereits erstellte Projektdokumente bleiben unverändert.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  const darf = $derived(mindestens(ich.rolle, "bearbeitung"));
  export function istBeschaeftigt() {
    return laeuft || !!editor || !!kategorieVorlage;
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
      katalog = b;
      kategorien = k.kategorien;
      zuordnungen = Object.fromEntries(z);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  onMount(laden);
  async function importieren() {
    const pfad = await open({
      multiple: false,
      title: "PDF-Vorlage importieren",
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (typeof pfad !== "string") return;
    laeuft = true;
    try {
      await api.bibliothekImportieren(pfad);
      await laden();
      hinweis =
        "Vorlage in der Bibliothek gespeichert. Sie kann jetzt im Projekt unter Formulare ausgefüllt werden.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  const fehlendeWerksvorlagen = $derived(
    (katalog?.werksvorlagen ?? []).filter((w) => !w.vorhanden),
  );
  async function werksvorlagenUebernehmen() {
    laeuft = true;
    fehler = null;
    try {
      const neu = await api.werksvorlagenUebernehmen();
      await laden();
      hinweis = `${neu.length} mitgelieferte ${neu.length === 1 ? "Vorlage" : "Vorlagen"} übernommen. Die Felder sind bereits mit den Projektdaten verknüpft.`;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function ansehen(v: BibliotheksVorlage) {
    laeuft = true;
    auswahl = v;
    vorschau = null;
    seite = 1;
    try {
      vorschau = await api.bibliothekVorschau(v);
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
        (await ask("Neue Felddefinitionen verwerfen?", {
          title: "Vorlageneditor",
          kind: "warning",
        })))
    );
  }
</script>

<h1>Formularbibliothek</h1>
<p class="meta">
  Zentrale Vorlagen verwalten und Eingabefelder definieren. Ausgefüllt werden
  Formulare direkt in der jeweiligen Projektakte unter „Formulare“.
</p>
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}{#if hinweis}<p
    role="status"
  >
    {hinweis}
  </p>{/if}
<div class="knoepfe links">
  <button disabled={!darf || laeuft} onclick={importieren}
    >PDF-Vorlage importieren …</button
  ><button class="sekundaer" disabled={laeuft} onclick={laden}
    >Aktualisieren</button
  >
</div>
{#if fehlendeWerksvorlagen.length}<aside class="hinweisbox werksvorlagen">
    <strong>Mitgelieferte amtliche Vordrucke</strong>
    <p>
      Diese Vordrucke liegen der Anwendung bei und sind bereits mit den
      Projektdaten verknüpft. In der Bibliothek stehen sie noch nicht:
    </p>
    <ul>
      {#each fehlendeWerksvorlagen as w (w.name)}<li>{w.titel}</li>{/each}
    </ul>
    <button disabled={!darf || laeuft} onclick={werksvorlagenUebernehmen}
      >Mitgelieferte Vordrucke übernehmen</button
    >
  </aside>{/if}
{#if katalog}<p class="meta">
    Formularordner: {katalog.root} · Speicherort unter Einstellungen ändern.
  </p>
  <table class="ergebnistabelle">
    <thead><tr><th>Vorlage</th><th>Standardkategorie</th><th></th></tr></thead
    ><tbody
      >{#each katalog.vorlagen as v}<tr
          ><td
            >{v.name}<span class="meta">
              · {v.felder ?? "?"} Eingabefelder</span
            ></td
          ><td
            >{kategorieText(kategorien, zuordnungen[v.sha256]?.kategorie)}
            <button
              class="link"
              disabled={!darf || laeuft}
              aria-label={`Kategorie ändern: ${v.name}`}
              onclick={() => {
                kategorieVorlage = v;
                kategorieWert = zuordnungen[v.sha256]?.kategorie ?? "";
              }}>Ändern</button
            ></td
          ><td class="aktionen"
            ><button class="link" disabled={laeuft} onclick={() => ansehen(v)}
              >Vorschau</button
            ><button
              class="link"
              disabled={!darf || laeuft}
              onclick={() => {
                dirty = false;
                editor = v;
              }}>Felder hinzufügen …</button
            ></td
          ></tr
        >{/each}</tbody
    >
  </table>
  {#if !katalog.vorlagen.length}<p>Noch keine Vorlagen vorhanden.</p>{/if}{/if}
{#if vorschau && auswahl}<h2>{auswahl.name}</h2>
  <label
    >Seite<select bind:value={seite}
      >{#each vorschau.seiten as s}<option value={s.nummer}>{s.nummer}</option
        >{/each}</select
    ></label
  >{#key vorschau.pdf_base64}<PdfVorschau
      pdfBase64={vorschau.pdf_base64}
      {seite}
      onfehler={(m) => (fehler = m)}
    />{/key}{/if}
{#if editor}<Dialog
    titel="Eingabefelder der Vorlage"
    breit
    onrequestclose={schliessenErlaubt}
    onclose={() => (editor = null)}
    ><PdfVorlagenEditor
      bibliothekVorlage={editor}
      onbibliothekFertig={async () => {
        editor = null;
        dirty = false;
        hinweis =
          "Neue Vorlage mit Eingabefeldern gespeichert. Das Original bleibt erhalten.";
        await laden();
      }}
      onabbrechen={async () => {
        if (await schliessenErlaubt()) editor = null;
      }}
      onstatus={(d, b) => {
        dirty = d;
        pdfBusy = b;
      }}
    /></Dialog
  >{/if}

{#if kategorieVorlage}<Dialog
    titel="Standardkategorie der Vorlage"
    schliessenErlaubt={!laeuft}
    onclose={() => (kategorieVorlage = null)}
  >
    <form onsubmit={kategorieSpeichern}>
      <p>
        {kategorieVorlage.name}. Neue Projektdokumente übernehmen diese
        Zuordnung. Inhaltsgleiche Vorlagen teilen dieselbe Standardkategorie.
      </p>
      {#if fehler}<p role="alert" class="fehlertext">{fehler}</p>{/if}
      <KategorieAuswahl
        {kategorien}
        bind:wert={kategorieWert}
        disabled={laeuft}
      />
      <div class="knoepfe">
        <button disabled={laeuft}>Standardkategorie speichern</button><button
          type="button"
          class="sekundaer"
          disabled={laeuft}
          onclick={() => (kategorieVorlage = null)}>Abbrechen</button
        >
      </div>
    </form>
  </Dialog>{/if}
