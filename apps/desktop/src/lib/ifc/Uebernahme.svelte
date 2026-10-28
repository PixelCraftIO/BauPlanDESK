<script lang="ts">
  /** Übernahme des Modells in Gebäude, Zeichnungen je Geschoss und Raumbücher. */
  import { fehlertext } from "../api";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import type { Gebaeude, Modellakte, Modelluebernahmeergebnis } from "../generated/plattform";
  import { ifcApi } from "./api";
  let { projektId, akte, darf, onakte, onzeichnung }: {
    projektId: string;
    akte: Datensatz<Modellakte>;
    darf: boolean;
    onakte: (a: Datensatz<Modellakte>) => void;
    onzeichnung?: (id: string, titel: string) => void;
  } = $props();
  let gebaeude = $state<Datensatz<Gebaeude>[]>([]);
  let ziel = $state(""), raumbuch = $state(true), laeuft = $state(false), fehler = $state("");
  let ergebnis = $state<Modelluebernahmeergebnis | null>(null);
  const stand = $derived(akte.daten.uebernahme ?? null);
  $effect(() => {
    void plattformApi.liste<Gebaeude>(projektId, "gebaeude").then(g => {
      gebaeude = g;
      if (!ziel) ziel = stand?.gebaeude_id ?? "";
    }).catch(e => (fehler = fehlertext(e)));
  });
  const name = (id: string) => gebaeude.find(g => g.objekt === id)?.daten.bezeichnung ?? "Gebäude";
  const veraltet = $derived(!!stand && (stand.dokument.dokument_id !== akte.daten.dokument.dokument_id || stand.dokument.version !== akte.daten.dokument.version));
  async function uebernehmen() {
    laeuft = true; fehler = "";
    try {
      ergebnis = await ifcApi.uebernehmen(projektId, { akte_id: akte.objekt, revision: akte.revision, gebaeude_id: ziel || null, raumbuch });
      onakte(ergebnis.akte as Datensatz<Modellakte>);
      gebaeude = await plattformApi.liste<Gebaeude>(projektId, "gebaeude");
      ziel = ergebnis.gebaeude.objekt;
    } catch (e) { fehler = fehlertext(e); } finally { laeuft = false; }
  }
</script>

<section class="uebernahme" aria-label="Übernahme ins Projekt">
  <p class="meta">Legt aus dem Modell ein Gebäude mit Geschossen an, je Geschoss eine eigene Zeichnung mit dem IFC-Grundriss als Hintergrund und Markierungen für Räume, Außenkante und Gebäudegrundfläche, auf Wunsch je Geschoss ein Raumbuch. Wiederholen aktualisiert die übernommenen Markierungen; selbst gezeichnete bleiben erhalten.</p>
  {#if stand}<p class="meta" role="status">Zuletzt übernommen in „{name(stand.gebaeude_id)}“ ({stand.geschosse.length} Geschosse){veraltet ? " · aus einer älteren Modellversion" : ""}.</p>{/if}
  <fieldset disabled={!darf || laeuft}>
    <label>Gebäude<select aria-label="Zielgebäude" bind:value={ziel}>
      <option value="">Neues Gebäude aus dem Modell</option>
      {#each gebaeude as g (g.objekt)}<option value={g.objekt}>{g.daten.bezeichnung}</option>{/each}
    </select></label>
    <label class="haken"><input type="checkbox" bind:checked={raumbuch}/>Raumbuch je Geschoss füllen</label>
    <button type="button" onclick={() => void uebernehmen()}>{stand ? "Übernahme aktualisieren" : "In Projekt übernehmen"}</button>
  </fieldset>
  {#if laeuft}<p class="meta" role="status">Übernahme läuft …</p>{/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if ergebnis}
    <div role="status" aria-label="Übernahmeergebnis">
      <p>Übernommen in „{(ergebnis.gebaeude.daten as Gebaeude).bezeichnung}“: {ergebnis.zeichnungen.length} Zeichnung(en){ergebnis.raumbuecher.length ? `, ${ergebnis.raumbuecher.length} Raumbuch/Raumbücher` : ""}.</p>
      <ul class="zeichnungen">
        {#each ergebnis.zeichnungen as z (z.objekt)}
          {@const titel = (z.daten as { bezeichnung: string }).bezeichnung}
          <li><button type="button" class="link" onclick={() => onzeichnung?.(z.objekt, titel)}>{titel}</button></li>
        {/each}
      </ul>
      {#if ergebnis.hinweise.length}<ul class="hinweise">{#each ergebnis.hinweise as h}<li>{h}</li>{/each}</ul>{/if}
    </div>
  {/if}
</section>

<style>
  .uebernahme fieldset{border:0;padding:0;margin:0 0 8px;display:flex;flex-direction:column;gap:6px}
  .uebernahme label{display:flex;flex-direction:column;font-size:.85rem}
  .uebernahme .haken{flex-direction:row;gap:6px;align-items:center}
  .zeichnungen{margin:4px 0;padding-left:18px}
  .hinweise{margin:4px 0;padding-left:18px;color:var(--warning-text,#6b4a00)}
</style>
