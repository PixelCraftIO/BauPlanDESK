<script lang="ts">
  import { onMount } from "svelte";
  import {
    api,
    fehlertext,
    type DocumentVersion,
    type Feldwert,
    type FormularAnalyse,
    type PdfVorschauDaten,
    type BibliothekQuelle,
  } from "./api";

  import PdfVorschau from "./PdfVorschau.svelte";
  let {
    dokumentId,
    quelle,
    onfertig,
    onabbrechen,
    onstatus,
  }: {
    dokumentId?: string;
    quelle?: BibliothekQuelle;
    onfertig: (ergebnis: DocumentVersion) => void;
    onabbrechen: () => void;
    onstatus?: (dirty: boolean, busy: boolean) => void;
  } = $props();

  let analyse = $state<
    (Omit<FormularAnalyse, "dokument"> & { dokument?: DocumentVersion }) | null
  >(null);
  let werte = $state<Record<string, Feldwert>>({});
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);
  let nurLeere = $state(false);
  let vorschau = $state<Pick<PdfVorschauDaten, "pdf_base64" | "seiten"> | null>(
    null,
  );
  let seite = $state(1);
  const dirty = $derived(
    !!analyse &&
      analyse.felder.some(
        (f) =>
          !f.schreibgeschuetzt &&
          werte[f.name] !== undefined &&
          werte[f.name] !== (f.wert ?? (f.art === "ankreuzfeld" ? false : "")),
      ),
  );
  $effect(() => onstatus?.(dirty, laeuft));

  onMount(async () => {
    try {
      const bibliothek = quelle ? await api.bibliothekFormular(quelle) : null;
      const a: Omit<FormularAnalyse, "dokument"> & {
        dokument?: DocumentVersion;
      } = bibliothek
        ? bibliothek.schema
        : await api.formularAnalysieren(dokumentId!);
      if (bibliothek) vorschau = bibliothek;
      const start: Record<string, Feldwert> = {};
      for (const f of a.felder) {
        if (f.art === "sonstiges") continue;
        start[f.name] =
          a.vorschlag[f.name] ??
          f.wert ??
          (f.art === "ankreuzfeld" ? false : "");
      }
      werte = start;
      analyse = a;
      if (a.dokument)
        void api
          .pdfVorschau(dokumentId!, a.dokument.version)
          .then((p) => (vorschau = p))
          .catch(() => {});
    } catch (e) {
      fehler = fehlertext(e);
    }
  });

  const sichtbar = $derived(
    (analyse?.felder ?? []).filter(
      (f) =>
        f.art !== "sonstiges" &&
        (!nurLeere || werte[f.name] === "" || werte[f.name] === false),
    ),
  );

  async function absenden(event: SubmitEvent) {
    event.preventDefault();
    if (!analyse) return;
    // Nur geänderte bzw. gesetzte Felder schicken; schreibgeschützte nie.
    const geaendert: Record<string, Feldwert> = {};
    for (const f of analyse.felder) {
      const w = werte[f.name];
      if (f.schreibgeschuetzt || w === undefined) continue;
      const alt = f.wert ?? (f.art === "ankreuzfeld" ? false : "");
      if (w !== alt) geaendert[f.name] = w;
    }
    laeuft = true;
    try {
      onfertig(
        quelle
          ? await api.bibliothekFormularSpeichern(quelle, geaendert)
          : await api.formularImProjektSpeichern(
              dokumentId!,
              analyse.dokument!.version,
              geaendert,
            ),
      );
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

{#if fehler}<p class="fehlertext">{fehler}</p>{/if}

{#if analyse}
  <p class="meta">
    {analyse.dokument?.file_name ?? quelle?.vorlage.name} · {analyse.felder
      .length} Felder
    {#if analyse.vorlage}· erkannt als <strong>{analyse.vorlage.titel}</strong>
      ({analyse.vorlage.version}){/if}
    · {Object.keys(analyse.vorschlag).length} Werte aus den Projektdaten übernommen
  </p>
  <label class="zeile"
    ><input type="checkbox" bind:checked={nurLeere} /> Nur leere Felder anzeigen</label
  >

  <div class="pdf-arbeitsplatz">
    <div class="pdf-vorschau-spalte">
      {#if vorschau}<label
          >Seite<select bind:value={seite}
            >{#each vorschau.seiten as p}<option value={p.nummer}
                >{p.nummer}</option
              >{/each}</select
          ></label
        ><PdfVorschau
          pdfBase64={vorschau.pdf_base64}
          {seite}
          onfehler={(m) => (fehler = m)}
        />
        <p class="meta">
          Vorschau der gespeicherten Vorlage. Ihre neuen Angaben werden beim
          Ausfüllen in die PDF übernommen.
        </p>{:else}<p class="meta">
          PDF-Vorschau wird geladen. Die Eingabemaske kann bereits verwendet
          werden.
        </p>{/if}
    </div>
    <form class="spalte pdf-eingabemaske" onsubmit={absenden}>
      <div class="formularfelder">
        {#each sichtbar as f (f.name)}
          <label class:vorschlag={f.name in analyse.vorschlag} title={f.name}>
            <span
              >{f.beschriftung ?? f.name}{f.name in analyse.vorschlag
                ? " · aus Projektdaten"
                : ""}</span
            >
            {#if f.art === "ankreuzfeld"}
              <input
                type="checkbox"
                bind:checked={werte[f.name] as boolean}
                disabled={f.schreibgeschuetzt}
              />
            {:else if f.art === "optionsfeld" || f.art === "auswahl"}
              <select bind:value={werte[f.name]} disabled={f.schreibgeschuetzt}>
                <option value="">–</option>
                {#each f.optionen as o}<option value={o}>{o}</option>{/each}
              </select>
            {:else}
              <input
                bind:value={werte[f.name]}
                maxlength={f.max_laenge ?? undefined}
                disabled={f.schreibgeschuetzt}
              />
            {/if}
          </label>
        {:else}
          <p class="meta">Alle Felder sind ausgefüllt.</p>
        {/each}
      </div>
      <p class="meta">
        {quelle
          ? "Die Bibliotheksvorlage bleibt unverändert. Im Projekt wird nur die ausgefüllte Datei gespeichert."
          : "Die Angaben werden als neue Version derselben Projektdatei gespeichert."}
      </p>
      <div class="knoepfe">
        <button type="button" class="sekundaer" onclick={onabbrechen}
          >Abbrechen</button
        >
        <button
          type="submit"
          disabled={laeuft ||
            !analyse.felder.some(
              (f) => f.art !== "sonstiges" && !f.schreibgeschuetzt,
            )}>{laeuft ? "Speichere …" : "Im Projekt speichern"}</button
        >
      </div>
    </form>
  </div>
{:else if !fehler}
  <p class="meta">Formular wird gelesen …</p>
{/if}

{#if fehler && !analyse}<button class="sekundaer" onclick={onabbrechen}
    >Schließen</button
  >{/if}
