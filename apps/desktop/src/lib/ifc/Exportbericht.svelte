<script lang="ts">
  /** Ergebnis eines IFC-Exports: abgelegte Datei, Zählungen, Selbstkontrolle. */
  import type { Exportergebnis } from "../generated/plattform";
  import Bauantragsanlage from "./Bauantragsanlage.svelte";
  let { ergebnis, projektId, darf = false, onoeffnen }: { ergebnis: Exportergebnis; projektId: string; darf?: boolean; onoeffnen?: () => void } = $props();
  const b = $derived(ergebnis.bericht);
</script>

<div class="bericht" role="status" aria-label="Exportergebnis">
  <p>In den Projektdateien abgelegt: <strong>{b.dateiname}</strong> · V{ergebnis.dokument.version} ({b.schema}, {b.elemente} Elemente{b.geaendert ? `, ${b.geaendert} geändert` : ""}{b.neu && b.geaendert + b.entfernt ? `, ${b.neu} neu` : ""}{b.entfernt ? `, ${b.entfernt} entfernt` : ""}).</p>
  {#if b.abweichungen.length}
    <p class="fehlertext">Selbstkontrolle: Die wieder eingelesene Datei weicht ab.</p>
    <ul class="liste">{#each b.abweichungen as a}<li>{a}</li>{/each}</ul>
  {:else}<p class="meta">Selbstkontrolle bestanden: Die Datei wurde wieder eingelesen und stimmt mit dem Modell überein.</p>{/if}
  {#if b.hinweise.length}<ul class="liste hinweise">{#each b.hinweise as h}<li>{h}</li>{/each}</ul>{/if}
  <div class="knoepfe">
    {#if onoeffnen}<button type="button" class="sekundaer" onclick={onoeffnen}>Als Modell öffnen</button>{/if}
    <Bauantragsanlage {projektId} {darf} dokumentId={ergebnis.dokument.dokument_id} bezeichnung={b.dateiname.replace(/\.ifc$/i, "")}/>
  </div>
</div>

<style>
  .bericht p{margin:4px 0}
  .liste{margin:4px 0;padding-left:18px;font-size:.85rem}
  .hinweise{color:var(--warning-text,#6b4a00)}
  .knoepfe{display:flex;gap:6px;flex-wrap:wrap;align-items:flex-start}
</style>
