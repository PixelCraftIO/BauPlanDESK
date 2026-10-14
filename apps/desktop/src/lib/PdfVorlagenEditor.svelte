<script lang="ts">
  import { onMount } from "svelte";
  import {
    api,
    fehlertext,
    type EigenesPdfFeld,
    type PdfVorschauDaten,
    type DocumentVersion,
  } from "./api";
  import PdfVorschau from "./PdfVorschau.svelte";
  let {
    dokumentId,
    onfertig,
    onabbrechen,
    onstatus,
  }: {
    dokumentId: string;
    onfertig: (d: DocumentVersion) => void;
    onabbrechen: () => void;
    onstatus?: (dirty: boolean, busy: boolean) => void;
  } = $props();
  let daten = $state<PdfVorschauDaten | null>(null),
    felder = $state<EigenesPdfFeld[]>([]),
    seite = $state(1),
    auswahl = $state(0),
    fehler = $state<string | null>(null),
    laeuft = $state(false);
  const aktiv = $derived(felder[auswahl]);
  const page = $derived(daten?.seiten.find((p) => p.nummer === seite));
  $effect(() => onstatus?.(felder.length > 0, laeuft));
  onMount(async () => {
    try {
      daten = await api.pdfVorschau(dokumentId);
    } catch (e) {
      fehler = fehlertext(e);
    }
  });
  function hinzufuegen(art: "text" | "ankreuzfeld") {
    let n = 1;
    while (
      felder.some((f) => f.name === `Platzhalter${n}`) ||
      daten?.felder.some((f) => f.name === `Platzhalter${n}`)
    )
      n++;
    felder.push({
      name: `Platzhalter${n}`,
      beschriftung: "",
      art,
      seite,
      x_mm: 20,
      y_mm: 30 + felder.filter((f) => f.seite === seite).length * 12,
      breite_mm: art === "text" ? 60 : 5,
      hoehe_mm: art === "text" ? 8 : 5,
      schriftgroesse_pt: 10,
    });
    auswahl = felder.length - 1;
  }
  function position(x: number, y: number) {
    if (!aktiv || !page?.platzierbar) return;
    aktiv.seite = seite;
    aktiv.x_mm =
      Math.round(
        Math.max(0, Math.min(x, page.breite_mm - aktiv.breite_mm)) * 10,
      ) / 10;
    aktiv.y_mm =
      Math.round(
        Math.max(0, Math.min(y, page.hoehe_mm - aktiv.hoehe_mm)) * 10,
      ) / 10;
  }
  async function speichern(e: SubmitEvent) {
    e.preventDefault();
    if (!daten) return;
    laeuft = true;
    fehler = null;
    try {
      onfertig(
        await api.formularVorlageErstellen(
          dokumentId,
          daten.dokument.version,
          $state.snapshot(felder),
        ),
      );
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
{#if daten}
  <p class="meta">
    {daten.dokument.file_name} · {daten.felder.length} bestehende Felder. Neue Felder
    bleiben in einer eigenen PDF-Vorlage gespeichert; das Original wird erhalten.
  </p>
  <form class="pdf-arbeitsplatz" onsubmit={speichern}>
    <div class="pdf-vorschau-spalte">
      <div class="knoepfe links">
        <label
          >Seite<select bind:value={seite}
            >{#each daten.seiten as p}<option value={p.nummer}
                >{p.nummer}</option
              >{/each}</select
          ></label
        ><span class="meta"
          >{page?.breite_mm.toFixed(1)} × {page?.hoehe_mm.toFixed(1)} mm</span
        >
      </div>
      {#key daten.pdf_base64}<PdfVorschau
          unrotiert
          pdfBase64={daten.pdf_base64}
          {seite}
          {felder}
          onposition={aktiv && page?.platzierbar ? position : undefined}
          onfehler={(m) => (fehler = m)}
        />{/key}
    </div>
    <div class="pdf-eigene-felder">
      <h3>Eigene Eingabefelder</h3>
      <p class="meta">
        Textfeld oder Checkbox hinzufügen, benennen und auf der Vorschau
        platzieren. Koordinaten in mm ab links oben.
      </p>
      {#if !page?.platzierbar}<p class="hinweisbox">
          Diese Seite hat eine gespeicherte Rotation oder einen besonderen
          Maßstab. Für neue Felder die PDF vorher entsprechend ausrichten.
        </p>{/if}
      <div class="knoepfe links">
        <button
          type="button"
          class="sekundaer"
          disabled={!page?.platzierbar || laeuft}
          onclick={() => hinzufuegen("text")}>Textfeld hinzufügen</button
        ><button
          type="button"
          class="sekundaer"
          disabled={!page?.platzierbar || laeuft}
          onclick={() => hinzufuegen("ankreuzfeld")}>Checkbox hinzufügen</button
        >
      </div>
      <div class="pdf-feldliste">
        {#each felder as f, i}<button
            type="button"
            class="sekundaer"
            class:aktiv={auswahl === i}
            onclick={() => {
              auswahl = i;
              seite = f.seite;
            }}>{f.name || "Unbenannt"} · Seite {f.seite}</button
          >{/each}
      </div>
      {#if aktiv}<fieldset disabled={laeuft}>
          <legend>Feld bearbeiten</legend>
          <div class="raster">
            <label
              >Feldname / Platzhalter<input
                bind:value={aktiv.name}
                required
                maxlength="120"
                placeholder="Vorname oder <Platzhalter1>"
              /></label
            >
            <label
              >Beschriftung in der Maske<input
                bind:value={aktiv.beschriftung}
                maxlength="500"
                placeholder="Vorname"
              /></label
            >
            <label
              >X (mm)<input
                type="number"
                min="0"
                max={page?.breite_mm}
                step="0.1"
                bind:value={aktiv.x_mm}
                required
              /></label
            ><label
              >Y (mm)<input
                type="number"
                min="0"
                max={page?.hoehe_mm}
                step="0.1"
                bind:value={aktiv.y_mm}
                required
              /></label
            >
            <label
              >Breite (mm)<input
                type="number"
                min="2"
                step="0.1"
                bind:value={aktiv.breite_mm}
                required
              /></label
            ><label
              >Höhe (mm)<input
                type="number"
                min="2"
                step="0.1"
                bind:value={aktiv.hoehe_mm}
                required
              /></label
            >
            {#if aktiv.art === "text"}<label
                >Schriftgröße (pt)<input
                  type="number"
                  min="6"
                  max="24"
                  step="1"
                  bind:value={aktiv.schriftgroesse_pt}
                  required
                /></label
              >{/if}
          </div>
          <button
            type="button"
            class="sekundaer"
            onclick={() => {
              felder.splice(auswahl, 1);
              auswahl = Math.max(0, auswahl - 1);
            }}>Feld entfernen</button
          >
        </fieldset>{/if}
      <p class="meta">
        Gleiche Namen sind nicht zulässig. Optional kann ein Feld z. B.
        „bauherr.vorname“ heißen, um verfügbare Projektdaten vorzuschlagen.
        Eigene Textfelder sind einzeilig.
      </p>
      <div class="knoepfe">
        <button
          type="button"
          class="sekundaer"
          disabled={laeuft}
          onclick={onabbrechen}>Abbrechen</button
        ><button type="submit" disabled={!felder.length || laeuft}
          >{laeuft ? "Wird erstellt …" : "Vorlage erstellen"}</button
        >
      </div>
    </div>
  </form>
{:else if !fehler}<p class="meta">PDF-Vorlage wird geladen …</p>{/if}
