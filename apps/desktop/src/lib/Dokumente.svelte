<script lang="ts">
  import { untrack } from "svelte";
  import { ask, open, save } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    istKonflikt,
    istPlan,
    mindestens,
    rollenName,
    type Benutzer,
    type CmdError,
    type DocumentLock,
    type DocumentVersion,
  } from "./api";
  import Arbeitskopien from "./Arbeitskopien.svelte";
  import Dialog from "./Dialog.svelte";
  import FormularAusfuellen from "./FormularAusfuellen.svelte";
  import Historie from "./Historie.svelte";

  let {
    projektId,
    benutzer,
    importAusloeser,
    onplan,
    onfehler,
  }: {
    projektId: string;
    benutzer: Benutzer;
    importAusloeser: number;
    onplan: (dokumentId: string) => void;
    onfehler: (m: string) => void;
  } = $props();

  let arbeitsEditor = $state<Arbeitskopien>();
  let suche = $state("");
  let versionen = $state<DocumentVersion[]>([]);
  let laeuft = $state(false);
  let formular = $state<DocumentVersion | null>(null);
  let hinweis = $state<string | null>(null);
  let sperren = $state<Map<string, DocumentLock>>(new Map());
  let historie = $state<DocumentVersion | null>(null);
  let pdfDirty = $state(false),
    pdfBusy = $state(false),
    ordnerLaeuft = $state(false),
    ordnerHistorie = $state(false);
  async function pdfSchliessenErlaubt() {
    return (
      !pdfBusy &&
      (!pdfDirty ||
        (await ask("PDF-Eingaben wurden noch nicht gespeichert. Verwerfen?", {
          title: "PDF-Formular",
          kind: "warning",
          okLabel: "Verwerfen",
          cancelLabel: "Weiter bearbeiten",
        })))
    );
  }
  async function pdfAbbrechen() {
    if (await pdfSchliessenErlaubt()) {
      formular = null;
      pdfDirty = false;
    }
  }
  async function inBibliothek(d: DocumentVersion) {
    if (
      !(await ask(
        "Diese PDF als gemeinsame Vorlage in der Formularbibliothek ablegen? Enthaltene Eingaben werden mitkopiert. Verwenden Sie eine leere Vorlage ohne persönliche Daten.",
        { title: "Formularbibliothek", kind: "warning" },
      ))
    )
      return;
    laeuft = true;
    try {
      const v = await api.bibliothekVeroeffentlichen(d.document_id, d.version);
      hinweis = `In der Formularbibliothek abgelegt: ${v.name}`;
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      laeuft = false;
    }
  }
  async function projektordnerExportieren() {
    const root = await api.ablageStatus().catch(() => null);
    const ziel = await open({
      directory: true,
      multiple: false,
      title: "Projektordner ausgeben",
      defaultPath: root?.root ?? undefined,
    });
    if (typeof ziel !== "string") return;
    ordnerLaeuft = true;
    try {
      const r = await api.projektordnerExportieren(
        projektId,
        ziel,
        ordnerHistorie,
      );
      hinweis = `${r.dateien} Dateien (${r.neu} neu) ausgegeben: ${r.ordner}`;
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      ordnerLaeuft = false;
    }
  }

  const darfAblegen = $derived(
    benutzer.aktiv && mindestens(benutzer.rolle, "bearbeitung"),
  );
  const darfFremdeAufheben = $derived(
    benutzer.aktiv && mindestens(benutzer.rolle, "projektleitung"),
  );
  /** Gesperrt von jemand anderem: Ablegen ist dann nicht möglich. */
  const fremdGesperrt = (id: string) => {
    const s = sperren.get(id);
    return !!s && s.benutzer_id !== benutzer.id;
  };

  const istPdf = (d: DocumentVersion) =>
    d.file_name.toLowerCase().endsWith(".pdf");

  async function externOeffnen(doc: DocumentVersion) {
    laeuft = true;
    try {
      await api.dokumentExternOeffnen(doc.document_id, doc.version);
      hinweis = `${doc.file_name} als Arbeitskopie im zugeordneten Programm geöffnet.`;
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      laeuft = false;
      await arbeitsEditor?.aktualisieren();
    }
  }
  async function exportieren(doc: DocumentVersion) {
    const ziel = await save({
      title: "Dokument exportieren",
      defaultPath: doc.file_name,
      filters: istPdf(doc) ? [{ name: "PDF", extensions: ["pdf"] }] : [],
    });
    if (!ziel) return;
    try {
      await api.dokumentExportieren(doc.document_id, ziel);
      hinweis = `${doc.file_name} wurde exportiert.`;
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  async function formularFertig(ergebnis: DocumentVersion) {
    formular = null;
    pdfDirty = false;
    await laden();
    hinweis = `${ergebnis.file_name} wurde abgelegt.`;
    hinweis =
      ergebnis.ablage_hinweis ??
      `${ergebnis.file_name} wurde im Projekt gespeichert. ${ergebnis.projektordner_pfad ?? ""}`;
  }

  // Je Dokument nur die neueste Version anzeigen, ältere als Historie zählen.
  const dokumente = $derived.by(() => {
    const neueste = new Map<string, { doc: DocumentVersion; anzahl: number }>();
    for (const v of versionen) {
      const bisher = neueste.get(v.document_id);
      if (!bisher || v.version > bisher.doc.version) {
        neueste.set(v.document_id, {
          doc: v,
          anzahl: (bisher?.anzahl ?? 0) + 1,
        });
      } else {
        bisher.anzahl++;
      }
    }
    return [...neueste.values()]
      .filter(({ doc }) =>
        doc.file_name
          .toLocaleLowerCase("de")
          .includes(suche.toLocaleLowerCase("de")),
      )
      .sort((a, b) => a.doc.file_name.localeCompare(b.doc.file_name, "de"));
  });

  export async function aktualisieren() {
    await laden();
  }
  async function laden() {
    try {
      const [v, s] = await Promise.all([
        api.dokumente(projektId),
        api.sperrenAuflisten(projektId),
      ]);
      versionen = v;
      sperren = new Map(s.map((x) => [x.document_id, x]));
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  /** Verständliche Meldung für die beiden typischen Mehrbenutzer-Fälle. */
  function ablegeFehler(e: unknown, dateiname: string) {
    if (istKonflikt(e)) {
      onfehler(
        `Inzwischen hat jemand eine neuere Version von „${dateiname}“ abgelegt. Ihre Datei wurde nicht gespeichert, damit nichts überschrieben wird. Sehen Sie sich die neue Version an und legen Sie Ihre Änderungen danach erneut ab.`,
      );
    } else if ((e as CmdError)?.art === "gesperrt") {
      onfehler(`„${dateiname}“ ist gesperrt. ${fehlertext(e)}`);
    } else {
      onfehler(fehlertext(e));
    }
  }

  async function sperrenUmschalten(doc: DocumentVersion) {
    const s = sperren.get(doc.document_id);
    try {
      if (!s) {
        await api.sperren(doc.document_id);
        hinweis = `${doc.file_name} ist für Sie gesperrt. Andere können es ansehen, aber keine neue Version ablegen.`;
      } else {
        if (
          s.benutzer_id !== benutzer.id &&
          !(await ask(
            `${doc.file_name} ist von ${s.benutzer_name} gesperrt. Sperre trotzdem aufheben?`,
            {
              title: "Fremde Sperre aufheben",
              kind: "warning",
              okLabel: "Aufheben",
              cancelLabel: "Abbrechen",
            },
          ))
        )
          return;
        await api.entsperren(doc.document_id);
        hinweis = `Sperre für ${doc.file_name} aufgehoben.`;
      }
    } catch (e) {
      onfehler(fehlertext(e));
    }
    await laden();
  }

  export async function importieren(dokumentId: string | null = null) {
    if (!darfAblegen) {
      onfehler(
        `Dokumente ablegen dürfen nur Benutzer mit der Rolle „Bearbeitung“ oder höher (Ihre Rolle: „${rollenName(benutzer.rolle)}“).`,
      );
      return;
    }
    const auswahl = await open({
      multiple: dokumentId === null,
      title: dokumentId ? "Neue Version auswählen" : "Dokumente importieren",
      filters: [
        {
          name: "Projektdateien",
          extensions: [
            "pdf",
            "dxf",
            "dwg",
            "xml",
            "png",
            "jpg",
            "jpeg",
            "xlsx",
            "xls",
            "docx",
            "doc",
            "odt",
            "ods",
            "csv",
            "txt",
          ],
        },
        { name: "Alle Dateien", extensions: ["*"] },
      ],
    });
    if (!auswahl) return;
    const pfade = Array.isArray(auswahl) ? auswahl : [auswahl];
    // Neue Version: auf der Version aufbauen, die hier angezeigt wird.
    // Hat jemand anderes inzwischen abgelegt, meldet core-lib einen Konflikt.
    const bisher = dokumentId
      ? dokumente.find((d) => d.doc.document_id === dokumentId)?.doc
      : undefined;
    laeuft = true;
    try {
      for (const pfad of pfade) {
        const v = await api.dokumentImportieren(
          projektId,
          pfad,
          dokumentId,
          bisher?.version ?? null,
        );
        hinweis =
          v.ablage_hinweis ??
          `${v.file_name} wurde als Version ${v.version} abgelegt.`;
      }
    } catch (e) {
      ablegeFehler(e, bisher?.file_name ?? "Dokument");
    } finally {
      laeuft = false;
      await laden();
    }
  }

  $effect(() => {
    void projektId;
    laden();
  });

  // Menü "Datei → Dokument importieren" zählt diesen Wert hoch.
  let letzterAusloeser = untrack(() => importAusloeser);
  $effect(() => {
    if (importAusloeser > letzterAusloeser) {
      letzterAusloeser = importAusloeser;
      importieren();
    }
  });

  const groesse = (b: number) =>
    b < 1024 * 1024
      ? `${(b / 1024).toFixed(0)} KB`
      : `${(b / 1024 / 1024).toFixed(1)} MB`;
  export function istBeschaeftigt() {
    return (
      arbeitsEditor?.istBeschaeftigt() ||
      laeuft ||
      ordnerLaeuft ||
      !!formular ||
      !!historie
    );
  }
</script>

<div class="knoepfe links">
  <button
    onclick={() => importieren()}
    disabled={laeuft || !darfAblegen}
    title={darfAblegen
      ? ""
      : "Dazu ist mindestens die Rolle „Bearbeitung“ nötig."}
    >{laeuft ? "Importiere …" : "Dateien importieren …"}</button
  >
  <button
    class="sekundaer"
    onclick={projektordnerExportieren}
    disabled={ordnerLaeuft}
    >{ordnerLaeuft
      ? "Ordner wird erstellt …"
      : "Projektordner exportieren …"}</button
  >
  <label class="zeile"
    ><input type="checkbox" bind:checked={ordnerHistorie} /> Mit Dokumenthistorie</label
  >
</div>
<p class="meta">
  Projektordner sind Exportkopien mit „Dokumente“ und „Pläne“. Änderungen an
  diesen Kopien werden nicht automatisch in die Anwendung übernommen.
</p>

<label class="dateisuche"
  >Dateien suchen<input
    type="search"
    bind:value={suche}
    placeholder="Dateiname, z. B. Lageplan oder .xlsx"
  /></label
>

{#if hinweis}
  <p class="meta" role="status">{hinweis}</p>
{/if}

<div class="tabellen-scroll">
  <table class="ergebnistabelle">
    <thead
      ><tr
        ><th>Datei</th><th>Version</th><th>Größe</th><th>Geändert</th><th
          >Sperre</th
        ><th></th></tr
      ></thead
    >
    <tbody>
      {#each dokumente as { doc, anzahl } (doc.document_id)}
        <tr>
          <td>{doc.file_name}</td>
          <td>{doc.version}{anzahl > 1 ? ` (${anzahl} Versionen)` : ""}</td>
          <td>{groesse(doc.size_bytes)}</td>
          <td
            >{new Date(doc.created_at).toLocaleString("de-DE")} · {doc.created_by}</td
          >
          <td>
            {#if sperren.get(doc.document_id)}
              {@const s = sperren.get(doc.document_id)!}
              <span
                class="sperre"
                class:eigene={s.benutzer_id === benutzer.id}
                title={`seit ${new Date(s.seit).toLocaleString("de-DE")}`}
              >
                🔒 {s.benutzer_id === benutzer.id
                  ? "von Ihnen"
                  : s.benutzer_name}
              </span>
            {/if}
          </td>
          <td class="aktionen">
            <button
              class="link"
              disabled={laeuft}
              title="Im zugeordneten Programm öffnen"
              onclick={() => externOeffnen(doc)}>Öffnen</button
            >
            {#if istPdf(doc) && darfAblegen && !fremdGesperrt(doc.document_id)}<button
                class="link"
                onclick={() => {
                  pdfDirty = false;
                  formular = doc;
                }}>Formular bearbeiten</button
              >{/if}
            <details class="dateiaktionen">
              <summary aria-label={`Weitere Aktionen für ${doc.file_name}`}
                >Weitere …</summary
              >
              <div>
                <button class="link" onclick={() => exportieren(doc)}
                  >Exportieren …</button
                >
                {#if istPlan(doc.file_name)}<button
                    class="link"
                    onclick={() => onplan(doc.document_id)}>Plan ansehen</button
                  >{/if}
                {#if darfAblegen && !fremdGesperrt(doc.document_id)}<button
                    class="link"
                    onclick={() => importieren(doc.document_id)}
                    >Neue Version …</button
                  >{/if}
                <button class="link" onclick={() => (historie = doc)}
                  >Historie …</button
                >
                {#if istPdf(doc) && darfAblegen}<button
                    class="link"
                    disabled={laeuft}
                    onclick={() => inBibliothek(doc)}
                    >In Formularbibliothek ablegen …</button
                  >{/if}
                {#if !sperren.get(doc.document_id)}{#if darfAblegen}<button
                      class="link"
                      onclick={() => sperrenUmschalten(doc)}>Sperren</button
                    >{/if}{:else if !fremdGesperrt(doc.document_id) || darfFremdeAufheben}<button
                    class="link"
                    onclick={() => sperrenUmschalten(doc)}>Entsperren</button
                  >{/if}
              </div>
            </details>
          </td>
        </tr>
      {:else}
        <tr
          ><td colspan="6" class="meta"
            >Noch keine Projektdateien. Über „Dateien importieren“ hinzufügen.</td
          ></tr
        >
      {/each}
    </tbody>
  </table>
</div>

{#if historie}
  <Dialog
    titel={`Historie: ${historie.file_name}`}
    onclose={() => (historie = null)}
  >
    <Historie
      dokument={historie}
      darfWiederherstellen={darfAblegen && !fremdGesperrt(historie.document_id)}
      onwiederhergestellt={async (v) => {
        historie = null;
        await laden();
        hinweis = `Version ${v.version} von ${v.file_name} wurde aus einer älteren Version erstellt.`;
      }}
      onfehler={(e) => ablegeFehler(e, historie?.file_name ?? "Dokument")}
      onschliessen={() => (historie = null)}
    />
  </Dialog>
{/if}

{#if formular}
  <Dialog
    titel="Formular ausfüllen"
    schliessenErlaubt={!pdfBusy}
    onrequestclose={pdfSchliessenErlaubt}
    onclose={() => (formular = null)}
  >
    <FormularAusfuellen
      dokumentId={formular.document_id}
      onfertig={formularFertig}
      onabbrechen={pdfAbbrechen}
      onstatus={(dirty, busy) => {
        pdfDirty = dirty;
        pdfBusy = busy;
      }}
    />
  </Dialog>
{/if}

<Arbeitskopien
  bind:this={arbeitsEditor}
  {projektId}
  {darfAblegen}
  onaktualisiert={laden}
  {onfehler}
/>

<style>
  .dateisuche {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    max-width: 40rem;
    margin: 1rem 0;
  }
  .dateisuche input {
    flex: 1;
    min-width: 0;
  }
  .dateiaktionen summary {
    cursor: pointer;
    color: var(--accent);
    font-size: 0.85rem;
  }
  .dateiaktionen > div {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.7rem;
    border: 1px solid var(--border);
    background: var(--panel);
    min-width: 12rem;
  }
</style>
