<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { editorAnmelden } from "./editoren";
  import { save } from "@tauri-apps/plugin-dialog";
  import { fehlertext } from "./api";
  import { druckApi, type Druckquelle, type Druckvorschau } from "./druckApi";
  import type {
    Druckauftrag,
    Druckkatalog,
    Druckeintrag,
  } from "./generated/plattform";
  import PdfVorschau from "./PdfVorschau.svelte";
  import { druckPdf } from "./druckPdf";
  let {
    projektId,
    vorauswahl,
    darfAblegen = false,
    onbusy,
  }: {
    projektId: string;
    vorauswahl?: Druckquelle;
    darfAblegen?: boolean;
    onbusy?: (b: boolean) => void;
  } = $props();
  let katalog = $state<Druckkatalog | null>(null),
    auswahl = $state<number[]>([]);
  let titel = $state("Projektunterlagen"),
    absender = $state(""),
    empfaenger = $state(""),
    anschreiben = $state(
      "Sehr geehrte Damen und Herren,\n\nanbei erhalten Sie die zusammengestellten Unterlagen zum Bauvorhaben.\n\nMit freundlichen Grüßen",
    );
  let mitAnschreiben = $state(untrack(() => !vorauswahl)),
    mitInhalt = $state(untrack(() => !vorauswahl)),
    seiten = $state<Record<number, string>>({});
  let busy = $state(false),
    fehler = $state(""),
    meldung = $state(""),
    pdf = $state<Druckvorschau | null>(null),
    seite = $state(1),
    fortschritt = $state("");
  let signatur = $state("");
  let ablageVorgang = crypto.randomUUID();
  $effect(() => {
    onbusy?.(busy);
  });
  function quelle(index: number): Druckquelle {
    const q = katalog!.eintraege[index].quelle;
    if (q.art !== "dokument") return q;
    const s = (seiten[index] ?? "").trim();
    if (!s) return { ...q, seiten: [] };
    const result: number[] = [];
    for (const teil of s.split(",")) {
      const match = /^\s*(\d+)\s*(?:-\s*(\d+)\s*)?$/.exec(teil);
      if (!match) throw new Error("Seiten bitte als 1, 3-5 eingeben.");
      const start = Number(match[1]),
        ende = Number(match[2] ?? match[1]);
      if (
        start < 1 ||
        ende < start ||
        ende > 300 ||
        result.length + ende - start + 1 > 300
      )
        throw new Error("Seitenauswahl muss zwischen 1 und 300 liegen.");
      for (let n = start; n <= ende; n++) {
        if (result.includes(n))
          throw new Error("Keine doppelten Seiten auswählen.");
        result.push(n);
      }
    }
    return { ...q, seiten: result };
  }
  function auftrag(): Druckauftrag {
    return {
      titel,
      absender,
      empfaenger,
      anschreiben,
      mit_anschreiben: mitAnschreiben,
      inhaltsverzeichnis: mitInhalt,
      projekt_version: katalog!.projekt_version,
      quellen: auswahl.map(quelle),
    };
  }
  const geaendert = $derived.by(() => {
    try {
      return !!pdf && JSON.stringify(auftrag()) !== signatur;
    } catch {
      return true;
    }
  });
  function gleich(a: Druckquelle, b: Druckquelle) {
    return (
      a.art === b.art &&
      (a.art === "dokument" && b.art === "dokument"
        ? a.dokument_id === b.dokument_id && a.version === b.version
        : a.art === "nachweis" && b.art === "nachweis"
          ? a.id === b.id
          : a.art === "raumbuch" && b.art === "raumbuch"
            ? a.id === b.id && a.revision === b.revision
            : true)
    );
  }
  async function laden() {
    busy = true;
    fehler = "";
    pdf = null;
    try {
      katalog = await druckApi.katalog(projektId);
      absender = katalog.absender;
      auswahl = [];
      seiten = {};
      if (vorauswahl) {
        if (
          vorauswahl.art === "dokument" &&
          !katalog.eintraege.some((e) => gleich(e.quelle, vorauswahl!))
        )
          katalog.eintraege.push({
            titel: `Ausgewählte Dokumentversion ${vorauswahl.version}`,
            quelle: vorauswahl,
            verfuegbar: true,
            hinweis: "Konkrete archivierte Version",
          });
        const index = katalog.eintraege.findIndex((e) =>
          gleich(e.quelle, vorauswahl!),
        );
        if (index >= 0 && katalog.eintraege[index].verfuegbar) {
          auswahl = [index];
          titel = katalog.eintraege[index].titel;
        } else
          fehler =
            "Die gewählte Unterlage ist nicht mehr im aktuellen Ausgabekatalog vorhanden.";
      }
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  editorAnmelden("druck", {
    hatAenderungen: () => false,
    istBeschaeftigt: () => busy,
    speichern: async () => !busy,
  });
  onMount(laden);
  function waehlen(i: number) {
    auswahl = auswahl.includes(i)
      ? auswahl.filter((n) => n !== i)
      : [...auswahl, i];
  }
  function verschieben(i: number, d: number) {
    const n = i + d;
    if (n < 0 || n >= auswahl.length) return;
    const neu = [...auswahl];
    [neu[i], neu[n]] = [neu[n], neu[i]];
    auswahl = neu;
  }
  async function erstellen() {
    busy = true;
    fehler = "";
    meldung = "";
    pdf = null;
    try {
      const a = auftrag();
      const p = await druckApi.erstellen(projektId, a);
      pdf = p;
      ablageVorgang = crypto.randomUUID();
      signatur = JSON.stringify(a);
      seite = 1;
      meldung = `${p.seiten} Seiten zur Ausgabe bereit.`;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  async function ablegen() {
    if (!pdf || geaendert) return;
    busy = true;
    fehler = "";
    try {
      const d = await druckApi.ablegen(
        projektId,
        titel,
        pdf.pdf_base64,
        ablageVorgang,
      );
      meldung = `Im Projekt abgelegt: ${d.file_name}. In der Verfahrensakte können Sie diese Version dem Abgabestand zuordnen.`;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  async function exportieren() {
    if (!pdf || geaendert) return;
    busy = true;
    fehler = "";
    try {
      const ziel = await save({
        title: "Druckausgabe als PDF speichern",
        defaultPath: "Druckausgabe.pdf",
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
      if (ziel) {
        await druckApi.exportieren(pdf.pdf_base64, ziel);
        meldung = "PDF gespeichert.";
      }
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  async function drucken() {
    if (!pdf || geaendert) return;
    busy = true;
    fehler = "";
    try {
      await druckPdf(
        pdf.pdf_base64,
        (n, gesamt) =>
          (fortschritt = `Druckseite ${n} von ${gesamt} vorbereiten …`),
      );
      meldung = "Druckdialog geöffnet. Ausgabe im Druckdialog bestätigen.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
      fortschritt = "";
    }
  }
  const istPdf = (e: Druckeintrag) =>
    e.quelle.art === "dokument" && /\.pdf · Version \d+$/.test(e.titel);
</script>

<h2>Drucken und PDF-Ausgabe</h2>
<p class="meta">
  Gespeicherte Formulare, Dateien und Berechnungsstände auswählen. Die
  Reihenfolge der Auswahl bestimmt die Reihenfolge im Paket. Ungespeicherte
  Eingaben zuerst in ihrem Fachbereich speichern; GRZ, GFZ, Geschossigkeit, BMZ
  und Abstandsflächen als Nachweisstand sichern.
</p>
<button class="sekundaer" disabled={busy} onclick={laden}
  >Auswahl aktualisieren</button
>
{#if fehler}<p role="alert" class="fehlertext">{fehler}</p>{/if}
{#if meldung}<p role="status">{meldung}</p>{/if}
{#if katalog}
  <fieldset disabled={busy}>
    <legend>Unterlagen zusammenstellen</legend>
    <div class="aktionen">
      <button
        class="sekundaer"
        onclick={() =>
          (auswahl = katalog!.eintraege.flatMap((e, i) =>
            e.verfuegbar ? [i] : [],
          ))}>Alle verfügbaren auswählen</button
      ><button class="sekundaer" onclick={() => (auswahl = [])}
        >Auswahl leeren</button
      >
    </div>
    <div class="druckliste">
      <table class="ergebnistabelle">
        <thead
          ><tr
            ><th>Auswahl</th><th>Unterlage / Berechnung</th><th>Hinweis</th></tr
          ></thead
        ><tbody>
          {#each katalog.eintraege as e, i}<tr
              ><td
                ><input
                  type="checkbox"
                  aria-label={e.titel}
                  checked={auswahl.includes(i)}
                  disabled={!e.verfuegbar}
                  onchange={() => waehlen(i)}
                /></td
              ><td>{e.titel}</td><td>{e.hinweis ?? ""}</td></tr
            >{/each}
        </tbody>
      </table>
    </div>
    {#if auswahl.length}<h3>Reihenfolge und einzelne Seiten</h3>
      <ol>
        {#each auswahl as index, i}<li>
            <span>{katalog.eintraege[index].titel}</span><button
              class="link"
              aria-label={`${i + 1}. Unterlage nach oben`}
              disabled={i === 0}
              onclick={() => verschieben(i, -1)}>↑</button
            ><button
              class="link"
              aria-label={`${i + 1}. Unterlage nach unten`}
              disabled={i === auswahl.length - 1}
              onclick={() => verschieben(i, 1)}>↓</button
            >{#if istPdf(katalog.eintraege[index])}<label
                >Seiten (leer = alle)<input
                  placeholder="z. B. 1, 3-5"
                  bind:value={seiten[index]}
                /></label
              >{/if}
          </li>{/each}
      </ol>{/if}
    <label>Bezeichnung<input maxlength="500" bind:value={titel} /></label>
    <div class="aktionen">
      <label
        ><input type="checkbox" bind:checked={mitAnschreiben} /> Anschreiben voranstellen</label
      ><label
        ><input type="checkbox" bind:checked={mitInhalt} /> Inhaltsverzeichnis mit
        Seitenzahlen</label
      >
    </div>
    {#if mitAnschreiben}<div class="raster">
        <label
          >Absender<textarea rows="3" maxlength="4000" bind:value={absender}
          ></textarea></label
        ><label
          >Empfänger<textarea rows="3" maxlength="4000" bind:value={empfaenger}
          ></textarea></label
        >
      </div>
      <label
        >Anschreiben<textarea
          rows="7"
          maxlength="16000"
          bind:value={anschreiben}
        ></textarea></label
      >{/if}
    <button disabled={!auswahl.length || !titel.trim()} onclick={erstellen}
      >Druckausgabe erstellen ({auswahl.length})</button
    >
  </fieldset>
{/if}
{#if pdf}<section aria-label="Druckvorschau">
    <h3>Druckvorschau · {pdf.seiten} Seiten</h3>
    {#if geaendert}<p role="status">
        Auswahl oder Texte geändert. Druckausgabe erneut erstellen.
      </p>{/if}
    <div class="aktionen">
      {#if darfAblegen}<button
          class="sekundaer"
          disabled={busy || geaendert}
          onclick={ablegen}>Als Projektunterlage ablegen</button
        >{/if}
      <button disabled={busy || geaendert} onclick={drucken}>Drucken …</button
      ><button
        class="sekundaer"
        disabled={busy || geaendert}
        onclick={exportieren}>PDF speichern …</button
      ><label
        >Seite<select bind:value={seite}
          >{#each Array(pdf.seiten) as _, i}<option value={i + 1}
              >{i + 1}</option
            >{/each}</select
        ></label
      >
    </div>
    {#if fortschritt}<p role="status">{fortschritt}</p>{/if}<PdfVorschau
      pdfBase64={pdf.pdf_base64}
      {seite}
      onfehler={(e) => (fehler = e)}
    />
  </section>{/if}

<style>
  .druckliste {
    max-height: 24rem;
    overflow: auto;
    margin-block: 1rem;
  }
  .aktionen {
    display: flex;
    gap: 1rem;
    align-items: center;
    flex-wrap: wrap;
    margin-block: 1rem;
  }
  ol {
    padding-left: 1.5rem;
  }
  li {
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border);
  }
  li > label {
    max-width: 18rem;
    display: block;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
  }
  section {
    margin-top: 1.5rem;
  }
  label {
    display: block;
    margin-block: 0.6rem;
  }
</style>
