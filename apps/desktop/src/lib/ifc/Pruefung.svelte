<script lang="ts">
  /** Modellprüfung für den Bauantrag: mitgeliefertes oder eigenes IDS und eingebaute Regeln. */
  import { api, fehlertext, type DocumentVersion } from "../api";
  import type { Befund, Modellauszug, Modellpruefung } from "../generated/plattform";
  import { ifcApi } from "./api";
  import { elementtitel } from "./format";
  let { projektId, akteId, auszug, stand = 0, onauswahl, onmarkieren, onaktion }: {
    projektId: string;
    akteId: string;
    auszug: Modellauszug;
    /** Wechselt nach Nacharbeit; die Prüfung läuft dann neu. */
    stand?: number;
    onauswahl: (gid: string) => void;
    onmarkieren: (gids: string[]) => void;
    onaktion?: (b: Befund) => void;
  } = $props();
  let idsDateien = $state<DocumentVersion[]>([]);
  let ids = $state(""), laeuft = $state(false), fehler = $state("");
  let ergebnis = $state<Modellpruefung | null>(null);
  let offen = $state<Record<number, boolean>>({});
  $effect(() => { void api.dokumente(projektId).then(d => (idsDateien = d.filter(x => /\.ids$/i.test(x.file_name)))).catch(() => {}); });
  async function pruefen() {
    laeuft = true; fehler = "";
    try {
      const d = idsDateien.find(x => `${x.document_id}|${x.version}` === ids);
      ergebnis = await ifcApi.pruefen(projektId, akteId, d ? { dokument_id: d.document_id, version: d.version } : null);
    } catch (e) { fehler = fehlertext(e); ergebnis = null; } finally { laeuft = false; }
  }
  $effect(() => { void stand; void akteId; void pruefen(); });
  const titel = (gid: string) => {
    const e = auszug.elemente.find(x => x.global_id === gid);
    if (e) return elementtitel(e);
    return [...auszug.geschosse, ...auszug.gebaeude, ...auszug.grundstuecke].find(x => x.global_id === gid)?.name || gid;
  };
  const stufentitel = { fehler: "Fehler", warnung: "Warnung", hinweis: "Hinweis" } as const;
  const aktionstitel = { klassifizieren: "Klassifizieren", geschoss_zuordnen: "Geschoss zuordnen", eigenschaft_setzen: "Eigenschaft setzen", raum_bilden: "Raum bilden", benennen: "Benennen" } as const;
</script>

<section class="pruefung" aria-label="Modellprüfung">
  <div class="zeile">
    <label>Regelwerk<select aria-label="Regelwerk der Prüfung" bind:value={ids} onchange={() => void pruefen()}>
      <option value="">Mitgeliefert: Modell für den Bauantrag</option>
      {#each idsDateien as d (`${d.document_id}|${d.version}`)}<option value={`${d.document_id}|${d.version}`}>{d.file_name} · V{d.version}</option>{/each}
    </select></label>
    <button type="button" class="sekundaer" disabled={laeuft} onclick={() => void pruefen()}>Erneut prüfen</button>
  </div>
  {#if laeuft}<p class="meta" role="status">Prüfung läuft …</p>{/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if ergebnis}
    <p class="summe" role="status" aria-label="Prüfergebnis">
      {ergebnis.regelwerk}: <strong>{ergebnis.fehler}</strong> Fehler, <strong>{ergebnis.warnungen}</strong> Warnungen, <strong>{ergebnis.hinweise}</strong> Hinweise
      {#if !ergebnis.befunde.length} · Das Modell erfüllt alle Regeln.{/if}
    </p>
    <ul class="befunde" aria-label="Befunde">
      {#each ergebnis.befunde as b, i (i)}
        <li class={b.stufe}>
          <div class="kopf"><span class="stufe">{stufentitel[b.stufe]}</span> <strong>{b.titel}</strong></div>
          <p>{b.text}</p>
          <div class="knoepfe">
            {#if b.elemente.length}
              <button type="button" class="link" onclick={() => onmarkieren(b.elemente)}>Im Modell zeigen ({b.elemente.length})</button>
              <button type="button" class="link" aria-expanded={!!offen[i]} onclick={() => (offen[i] = !offen[i])}>Elemente</button>
            {/if}
            {#if b.aktion && onaktion}<button type="button" class="sekundaer klein" onclick={() => onaktion(b)}>{aktionstitel[b.aktion]} …</button>{/if}
          </div>
          {#if offen[i]}<ul class="elemente">{#each b.elemente.slice(0, 200) as g (g)}<li><button type="button" class="link" onclick={() => onauswahl(g)}>{titel(g)}</button></li>{/each}{#if b.elemente.length > 200}<li class="meta">… und {b.elemente.length - 200} weitere</li>{/if}</ul>{/if}
        </li>
      {/each}
    </ul>
    {#if ergebnis.nicht_geprueft.length}<h4>Nicht geprüft</h4><ul class="meta">{#each ergebnis.nicht_geprueft as n}<li>{n}</li>{/each}</ul>{/if}
  {/if}
</section>

<style>
  .zeile{display:flex;gap:6px;align-items:flex-end;flex-wrap:wrap;margin-bottom:6px}
  .zeile label{display:flex;flex-direction:column;font-size:.85rem;flex:1;min-width:0}
  .summe{margin:4px 0 8px}
  .befunde{list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:6px}
  .befunde > li{border:1px solid var(--border-innen);border-left-width:4px;padding:4px 6px;background:var(--panel)}
  .befunde > li.fehler{border-left-color:var(--danger,#b3261e)}
  .befunde > li.warnung{border-left-color:var(--warning-text,#9a6700)}
  .befunde > li.hinweis{border-left-color:var(--accent)}
  .befunde p{margin:2px 0;font-size:.85rem}
  .stufe{font-size:.75rem;text-transform:uppercase;letter-spacing:.03em;color:var(--muted)}
  .knoepfe{display:flex;gap:8px;align-items:center;flex-wrap:wrap}
  .klein{min-height:22px;padding:0 8px;font-size:.8rem}
  .elemente{margin:4px 0 0;padding-left:16px;font-size:.85rem}
  h4{margin:8px 0 4px;font-size:.85rem}
</style>
