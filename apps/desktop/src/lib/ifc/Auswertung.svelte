<script lang="ts">
  /** Auswertung des Modells: Gebäudegrundfläche, BGF/BRI je Geschoss,
   * Netto-Raumflächen nach Nutzung, Höhen und rechnerische GRZ/GFZ. */
  import { api, fehlertext, type GrundstueckKontext } from "../api";
  import type { Modellauswertung, Planpolygon } from "../generated/plattform";
  import { ifcApi } from "./api";
  import { zahl, flaeche, laenge } from "./format";
  let { projektId, akteId, stand = 0, onvorschau }: {
    projektId: string;
    akteId: string;
    /** Wechselt nach Nacharbeit oder Reimport; die Auswertung läuft dann neu. */
    stand?: number;
    /** Gebäudegrundfläche im Grundriss zeigen. */
    onvorschau?: (p: Planpolygon[]) => void;
  } = $props();
  let grundstuecke = $state<GrundstueckKontext[]>([]);
  let grundstueck = $state(""), laeuft = $state(false), fehler = $state("");
  let v = $state<Modellauswertung | null>(null);
  $effect(() => { void api.grundstueckskontext(projektId).then(g => (grundstuecke = (g ?? []).filter(x => x.bezugsflaeche_m2 != null))).catch(() => (grundstuecke = [])); });
  async function auswerten() {
    laeuft = true; fehler = "";
    try {
      v = await ifcApi.auswerten(projektId, akteId, grundstueck || null);
      onvorschau?.(v.grundflaeche);
    } catch (e) { fehler = fehlertext(e); v = null; onvorschau?.([]); } finally { laeuft = false; }
  }
  $effect(() => { void stand; void akteId; void auswerten(); });
  const m3 = (x: number) => `${zahl(x)} m³`;
  const opt = (x: number | null | undefined) => (x == null ? "—" : laenge(x));
</script>

<section class="auswertung" aria-label="Auswertung">
  {#if grundstuecke.length > 1}
    <label>Baugrundstück<select aria-label="Baugrundstück für GRZ und GFZ" bind:value={grundstueck} onchange={() => void auswerten()}>
      <option value="">Baugrundstück wählen</option>
      {#each grundstuecke as g (g.id)}<option value={g.id}>{g.bezeichnung} · {flaeche(g.bezugsflaeche_m2 ?? 0)}</option>{/each}
    </select></label>
  {/if}
  {#if laeuft && !v}<p class="meta" role="status">Modell wird ausgewertet …</p>{/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if v}
    <dl class="werte" aria-label="Kennwerte">
      <dt>Gebäudegrundfläche</dt><dd>{flaeche(v.grundflaeche_m2)}</dd>
      <dt>BGF gesamt</dt><dd>{flaeche(v.bgf_m2)}</dd>
      <dt>BGF oberirdisch</dt><dd>{flaeche(v.bgf_oberirdisch_m2)}</dd>
      <dt>BRI</dt><dd>{m3(v.bri_m3)}</dd>
      <dt>NRF</dt><dd>{flaeche(v.nrf_m2)}</dd>
      {#if v.grundstueck}<dt>Bezugsfläche</dt><dd>{flaeche(v.grundstueck.flaeche_m2)} <span class="meta">{v.grundstueck.bezeichnung}</span></dd>{/if}
      <dt>GRZ (rechnerisch)</dt><dd>{v.grz == null ? "—" : zahl(v.grz, 3)}</dd>
      <dt>GFZ (rechnerisch)</dt><dd>{v.gfz == null ? "—" : zahl(v.gfz, 3)}</dd>
    </dl>
    <h4>Geschosse</h4>
    <table class="liste" aria-label="Geschosse"><thead><tr><th>Geschoss</th><th class="zahl">BGF</th><th class="zahl">Höhe</th><th class="zahl">BRI</th><th class="zahl">NRF</th></tr></thead>
      <tbody>{#each v.geschosse as g (g.global_id)}<tr><td>{g.name}{g.oberirdisch ? "" : " (unter EG)"}</td><td class="zahl">{zahl(g.bgf_m2)}</td><td class="zahl">{zahl(g.hoehe_m)}</td><td class="zahl">{zahl(g.bri_m3)}</td><td class="zahl" title={g.nrf_menge_m2 != null ? `Mengen der Datei: ${flaeche(g.nrf_menge_m2)}` : undefined}>{zahl(g.nrf_m2)}</td></tr>{/each}</tbody>
    </table>
    {#if v.nutzungen.length}
      <h4>Raumflächen nach Nutzung</h4>
      <table class="liste" aria-label="Nutzungen"><tbody>{#each v.nutzungen as n (n.nutzung)}<tr><td>{n.nutzung}</td><td class="zahl">{n.raeume} {n.raeume === 1 ? "Raum" : "Räume"}</td><td class="zahl">{flaeche(n.flaeche_m2)}</td></tr>{/each}</tbody></table>
    {/if}
    <h4>Höhen über EG</h4>
    <dl class="werte" aria-label="Höhen">
      <dt>Bezug (EG)</dt><dd>{laenge(v.hoehen.bezug_m)}{v.hoehen.bezug_nhn_m != null ? ` · ${zahl(v.hoehen.bezug_nhn_m)} m ü. NHN` : ""}</dd>
      <dt>{v.hoehen.flachdach ? "Oberkante Flachdach" : "First"}</dt><dd>{opt(v.hoehen.first_m)}</dd>
      {#if !v.hoehen.flachdach}<dt>Traufe (Näherung)</dt><dd>{opt(v.hoehen.traufe_m)}</dd>{/if}
      <dt>Höchster Punkt</dt><dd>{opt(v.hoehen.oberkante_m)}</dd>
    </dl>
    {#if v.hinweise.length}<h4>Hinweise</h4><ul class="hinweise">{#each v.hinweise as h}<li>{h}</li>{/each}</ul>{/if}
    <p class="meta">Flächen aus der Geometrie des Modells mit Nacharbeit. Für den Bauantrag die Werte über „Übernahme“ in Gebäude und Zeichnungen übernehmen; dort rechnen die Fachmodule GRZ, GFZ und Abstandsflächen.</p>
  {/if}
</section>

<style>
  .auswertung label{display:flex;flex-direction:column;font-size:.8rem;margin-bottom:6px}
  .werte{display:grid;grid-template-columns:auto 1fr;gap:2px 8px;margin:0 0 8px}
  .werte dt{color:var(--muted)}
  .werte dd{margin:0}
  .liste{width:100%;border-collapse:collapse;font-size:.85rem;margin-bottom:8px}
  .liste td,.liste th{border-bottom:1px solid var(--border-innen);padding:2px 4px;text-align:left;vertical-align:top}
  .zahl{text-align:right;white-space:nowrap}
  h4{margin:8px 0 4px;font-size:.85rem}
  .hinweise{margin:0 0 8px;padding-left:18px;font-size:.85rem}
</style>
