<script lang="ts">
  import { onMount } from "svelte";
  import { api, fehlertext, type DocumentVersion, type Feldwert, type FormularAnalyse } from "./api";

  let {
    dokumentId,
    onfertig,
    onabbrechen,
  }: {
    dokumentId: string;
    onfertig: (ergebnis: DocumentVersion) => void;
    onabbrechen: () => void;
  } = $props();

  let analyse = $state<FormularAnalyse | null>(null);
  let werte = $state<Record<string, Feldwert>>({});
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);
  let nurLeere = $state(false);

  onMount(async () => {
    try {
      const a = await api.formularAnalysieren(dokumentId);
      const start: Record<string, Feldwert> = {};
      for (const f of a.felder) {
        if (f.art === "sonstiges") continue;
        start[f.name] = a.vorschlag[f.name] ?? f.wert ?? (f.art === "ankreuzfeld" ? false : "");
      }
      werte = start;
      analyse = a;
    } catch (e) {
      fehler = fehlertext(e);
    }
  });

  const sichtbar = $derived(
    (analyse?.felder ?? []).filter(
      (f) => f.art !== "sonstiges" && (!nurLeere || werte[f.name] === "" || werte[f.name] === false),
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
      onfertig(await api.formularAusfuellen(dokumentId, geaendert));
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
    {analyse.dokument.file_name} · {analyse.felder.length} Felder
    {#if analyse.vorlage}· erkannt als <strong>{analyse.vorlage.titel}</strong> ({analyse.vorlage.version}){/if}
    · {Object.keys(analyse.vorschlag).length} Werte aus den Projektdaten übernommen
  </p>
  <label class="zeile"><input type="checkbox" bind:checked={nurLeere} /> Nur leere Felder anzeigen</label>

  <form class="spalte" onsubmit={absenden}>
    <div class="formularfelder">
      {#each sichtbar as f (f.name)}
        <label class:vorschlag={f.name in analyse.vorschlag} title={f.name}>
          <span>{f.beschriftung ?? f.name}{f.name in analyse.vorschlag ? " · aus Projektdaten" : ""}</span>
          {#if f.art === "ankreuzfeld"}
            <input type="checkbox" bind:checked={werte[f.name] as boolean} disabled={f.schreibgeschuetzt} />
          {:else if f.art === "optionsfeld" || f.art === "auswahl"}
            <select bind:value={werte[f.name]} disabled={f.schreibgeschuetzt}>
              <option value="">–</option>
              {#each f.optionen as o}<option value={o}>{o}</option>{/each}
            </select>
          {:else}
            <input bind:value={werte[f.name]} maxlength={f.max_laenge ?? undefined} disabled={f.schreibgeschuetzt} />
          {/if}
        </label>
      {:else}
        <p class="meta">Alle Felder sind ausgefüllt.</p>
      {/each}
    </div>
    <p class="meta">Das leere Formular bleibt erhalten; das Ergebnis wird als neues Dokument „… (ausgefüllt).pdf“ abgelegt.</p>
    <div class="knoepfe">
      <button type="button" class="sekundaer" onclick={onabbrechen}>Abbrechen</button>
      <button type="submit" disabled={laeuft}>{laeuft ? "Speichere …" : "Ausfüllen und ablegen"}</button>
    </div>
  </form>
{:else if !fehler}
  <p class="meta">Formular wird gelesen …</p>
{/if}
