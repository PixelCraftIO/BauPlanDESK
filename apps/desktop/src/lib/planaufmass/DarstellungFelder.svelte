<script lang="ts">
  /** Stift, Linienart, Farbe und Schraffur; leere Felder erben (Element → Ebene → Standard). */
  import type { Darstellung } from "../generated/plattform";
  import { STIFTE, LINIENARTEN, SCHRAFFUREN, BAUZUSTAENDE, leer } from "./darstellung";
  let { wert, onaendern, erbe = "Standard", schraffur = true, gemischt = false, disabled = false }:
    { wert: Darstellung | null | undefined; onaendern: (d: Darstellung | undefined) => void; erbe?: string; schraffur?: boolean; gemischt?: boolean; disabled?: boolean } = $props();
  function setzen<K extends keyof Darstellung>(k: K, v: Darstellung[K] | undefined) {
    const d: Darstellung = { ...(wert ?? {}) };
    if (v === undefined || v === null) delete d[k]; else d[k] = v;
    onaendern(leer(d) ? undefined : d);
  }
  const ohne = $derived(gemischt ? "– unterschiedlich / erben –" : `wie ${erbe}`);
</script>

<fieldset class="darstellung" {disabled}>
  <legend>Darstellung</legend>
  <label>Stift (mm)<select value={wert?.stift_mm ?? ""} onchange={e => setzen("stift_mm", e.currentTarget.value ? Number(e.currentTarget.value) : undefined)}>
    <option value="">{ohne}</option>{#each STIFTE as s}<option value={s}>{s.toLocaleString("de-DE")}</option>{/each}</select></label>
  <label>Linienart<select value={wert?.linienart ?? ""} onchange={e => setzen("linienart", (e.currentTarget.value || undefined) as Darstellung["linienart"])}>
    <option value="">{ohne}</option>{#each LINIENARTEN as [v, t]}<option value={v}>{t}</option>{/each}</select></label>
  <label>Farbe<span class="farbe"><input type="color" value={wert?.farbe ?? "#2b3138"} onchange={e => setzen("farbe", e.currentTarget.value)} aria-label="Farbe wählen"/>
    {#if wert?.farbe}<button type="button" class="link" onclick={() => setzen("farbe", undefined)}>{ohne}</button>{:else}<span class="meta">{ohne}</span>{/if}</span></label>
  <label>Bauzustand<select value={wert?.bauzustand ?? ""} onchange={e => setzen("bauzustand", (e.currentTarget.value || undefined) as Darstellung["bauzustand"])} title="Farbe nach BauVorlV: Bestand grau, Abbruch gelb, Neubau rot (eigene Farbe hat Vorrang)">
    <option value="">{ohne}</option>{#each BAUZUSTAENDE as [v, t]}<option value={v}>{t}</option>{/each}</select></label>
  {#if schraffur}<label>Schraffur<select value={wert?.schraffur ?? ""} onchange={e => setzen("schraffur", (e.currentTarget.value || undefined) as Darstellung["schraffur"])}>
    <option value="">{ohne}</option>{#each SCHRAFFUREN as [v, t]}<option value={v}>{t}</option>{/each}</select></label>{/if}
</fieldset>

<style>
  .darstellung{display:grid;grid-template-columns:repeat(auto-fit,minmax(8rem,1fr));gap:.35rem .6rem;margin:.4rem 0;padding:.4rem .5rem}
  .darstellung legend{font-size:.85rem;font-weight:600}
  .farbe{display:flex;gap:.4rem;align-items:center}
  .farbe input{width:2.4rem;height:1.6rem;padding:0}
</style>
