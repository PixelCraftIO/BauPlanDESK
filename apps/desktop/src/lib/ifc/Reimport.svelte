<script lang="ts">
  /** Reimport: neue Modellversion über die GlobalId abgleichen und die Akte umstellen. */
  import { fehlertext, type DocumentVersion } from "../api";
  import type { Datensatz } from "../plattformApi";
  import type { Modellabgleich, Modellakte } from "../generated/plattform";
  import { ifcApi } from "./api";
  let { projektId, akte, dokumente, darf, onakte }: {
    projektId: string;
    akte: Datensatz<Modellakte>;
    dokumente: DocumentVersion[];
    darf: boolean;
    onakte: (a: Datensatz<Modellakte>) => void;
  } = $props();
  const aktuell = $derived(akte.daten.dokument);
  const andere = $derived(dokumente.filter(d => !(d.document_id === aktuell.dokument_id && d.version === aktuell.version)));
  let wahl = $state(""), abgleich = $state<Modellabgleich | null>(null);
  let ueberholte = $state(true), verwaiste = $state(false), laeuft = $state(false), fehler = $state(""), meldung = $state("");
  $effect(() => {
    // Vorbelegt: neueste Version derselben Datei.
    if (wahl) return;
    const n = andere.filter(d => d.document_id === aktuell.dokument_id && d.version > aktuell.version).sort((a, b) => b.version - a.version)[0];
    if (n) wahl = `${n.document_id}|${n.version}`;
  });
  const bezug = () => { const d = andere.find(x => `${x.document_id}|${x.version}` === wahl); return d ? { dokument_id: d.document_id, version: d.version } : null; };
  async function abgleichen() {
    const b = bezug(); if (!b) return;
    laeuft = true; fehler = ""; meldung = "";
    try { abgleich = await ifcApi.abgleich(projektId, akte.objekt, b); } catch (e) { fehler = fehlertext(e); abgleich = null; } finally { laeuft = false; }
  }
  async function umstellen() {
    const b = bezug(); if (!b) return;
    laeuft = true; fehler = "";
    try {
      const f = await ifcApi.reimport(projektId, { akte_id: akte.objekt, revision: akte.revision, dokument: b, ueberholte_entfernen: ueberholte, verwaiste_entfernen: verwaiste });
      onakte(f as Datensatz<Modellakte>);
      meldung = "Modellakte auf die neue Version umgestellt. Übernahme bei Bedarf aktualisieren.";
      abgleich = null; wahl = "";
    } catch (e) { fehler = fehlertext(e); } finally { laeuft = false; }
  }
  const zahl = (s: string) => abgleich?.nacharbeit.filter(x => x === s).length ?? 0;
</script>

<section class="reimport" aria-label="Neue Modellversion">
  <h4>Neue Modellversion</h4>
  {#if andere.length}
    <div class="zeile">
      <label>IFC-Datei<select aria-label="Neue Modellversion" bind:value={wahl} onchange={() => (abgleich = null)}><option value="">Datei wählen</option>{#each andere as d (`${d.document_id}|${d.version}`)}<option value={`${d.document_id}|${d.version}`}>{d.file_name} · V{d.version}</option>{/each}</select></label>
      <button type="button" class="sekundaer" disabled={!wahl || laeuft} onclick={() => void abgleichen()}>Abgleichen</button>
    </div>
  {:else}<p class="meta">Neue Fassung des Modells als neue Version der Datei unter Projektdateien ablegen; dann hier abgleichen.</p>{/if}
  {#if abgleich}
    <div role="status" aria-label="Abgleich">
      <p>{abgleich.neu.length} neu, {abgleich.entfernt.length} entfernt, {abgleich.geaendert.length} geändert, {abgleich.unveraendert} unverändert{abgleich.alt_schema !== abgleich.neu_schema ? ` · Schema ${abgleich.alt_schema} → ${abgleich.neu_schema}` : ""}.</p>
      {#if abgleich.geschosse.length}<ul>{#each abgleich.geschosse as g}<li>{g}</li>{/each}</ul>{/if}
      {#if abgleich.geaendert.length}<details><summary>Geändert ({abgleich.geaendert.length})</summary><ul>{#each abgleich.geaendert as g (g.global_id)}<li>{g.titel}: {g.aenderungen.join(", ")}</li>{/each}</ul></details>{/if}
      {#if abgleich.neu.length}<details><summary>Neu ({abgleich.neu.length})</summary><ul>{#each abgleich.neu as g (g.global_id)}<li>{g.titel}</li>{/each}</ul></details>{/if}
      {#if abgleich.entfernt.length}<details><summary>Entfernt ({abgleich.entfernt.length})</summary><ul>{#each abgleich.entfernt as g (g.global_id)}<li>{g.titel}</li>{/each}</ul></details>{/if}
      {#if abgleich.nacharbeit.length}<p>Nacharbeit: {zahl("wirksam")} wirkt weiter, {zahl("ueberholt")} überholt, {zahl("verwaist")} verwaist.</p>{/if}
      {#each abgleich.hinweise as h}<p class="hinweis">{h}</p>{/each}
    </div>
    {#if darf}
      <fieldset disabled={laeuft}>
        <label class="haken"><input type="checkbox" bind:checked={ueberholte}/>Überholte Nacharbeit streichen</label>
        <label class="haken"><input type="checkbox" bind:checked={verwaiste}/>Verwaiste Nacharbeit streichen</label>
        <button type="button" onclick={() => void umstellen()}>Auf diese Version umstellen</button>
      </fieldset>
    {/if}
  {/if}
  {#if meldung}<p class="meta" role="status">{meldung}</p>{/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
</section>

<style>
  h4{margin:10px 0 4px;font-size:.85rem}
  .zeile{display:flex;gap:4px;align-items:flex-end;flex-wrap:wrap}
  .zeile label{display:flex;flex-direction:column;font-size:.8rem;flex:1;min-width:0}
  ul{margin:2px 0;padding-left:18px;font-size:.85rem}
  p{margin:4px 0;font-size:.85rem}
  .hinweis{color:var(--warning-text,#6b4a00)}
  fieldset{border:0;padding:0;margin:4px 0;display:flex;flex-direction:column;gap:4px}
  .haken{display:flex;gap:6px;align-items:center;font-size:.85rem}
</style>
