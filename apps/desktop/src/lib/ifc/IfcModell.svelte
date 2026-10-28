<script lang="ts">
  /** IFC-Modell als Tab im Aufmaß: Modellakte wählen oder anlegen, Struktur,
   * 3D-Ansicht, Grundriss-Schnitt, Eigenschaften und Modellbericht. */
  import { untrack } from "svelte";
  import { api, fehlertext, istIfc, type DocumentVersion, type Project } from "../api";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import type { Modellakte, Modellauszug, Modellelement, Modellgrundriss, Strukturelement } from "../generated/plattform";
  import { ifcApi } from "./api";
  import Grundriss from "./Grundriss.svelte";
  import { zahl, flaeche, laenge, klassentitel, elementtitel } from "./format";
  let { projekt, darf, start, onstatus }: {
    projekt: Project;
    darf: boolean;
    start: { modell?: string };
    onstatus?: (s: { titel: string; geaendert: boolean; id?: string | null }) => void;
  } = $props();
  let akte = $state<Datensatz<Modellakte> | null>(null);
  let akten = $state<Datensatz<Modellakte>[]>([]);
  let dokumente = $state<DocumentVersion[]>([]);
  let auszug = $state<Modellauszug | null>(null);
  let fehler = $state(""), laedt = $state(false), meldung = $state("");
  let neuDatei = $state(""), neuName = $state("");
  let ansicht = $state<"3d" | "grundriss">("grundriss");
  let geschoss = $state(""), schnitt = $state(1);
  let grundriss = $state<Modellgrundriss | null>(null);
  let auswahl = $state("");
  let rechts = $state<"eigenschaften" | "bericht">("eigenschaften");
  const element = $derived(auszug?.elemente.find(e => e.global_id === auswahl) ?? null);
  const struktur = $derived(auszug?.geschosse.find(g => g.global_id === auswahl) ?? auszug?.gebaeude.find(g => g.global_id === auswahl) ?? null);
  const geschossName = (gid: string | null | undefined) => auszug?.geschosse.find(g => g.global_id === gid)?.name || "ohne Geschoss";

  async function listen() {
    try {
      const [a, d] = await Promise.all([plattformApi.liste<Modellakte>(projekt.id, "ifcmodell"), api.dokumente(projekt.id)]);
      akten = a.sort((x, y) => x.daten.bezeichnung.localeCompare(y.daten.bezeichnung, "de"));
      dokumente = d.filter(x => istIfc(x.file_name)).sort((x, y) => x.file_name.localeCompare(y.file_name, "de") || y.version - x.version);
    } catch (e) { fehler = fehlertext(e); }
  }
  async function oeffnen(id: string) {
    laedt = true; fehler = "";
    try {
      const a = await plattformApi.lesen<Modellakte>(projekt.id, "ifcmodell", id);
      if (!a) { fehler = "Die Modellakte gibt es nicht mehr."; return; }
      akte = a;
      onstatus?.({ titel: a.daten.bezeichnung, geaendert: false, id: a.objekt });
      await modellLaden();
    } catch (e) { fehler = fehlertext(e); } finally { laedt = false; }
  }
  async function modellLaden() {
    if (!akte) return;
    laedt = true;
    try {
      auszug = await ifcApi.auszug(projekt.id, akte.daten.dokument);
      if (!auszug.geschosse.some(g => g.global_id === geschoss)) geschoss = auszug.geschosse[0]?.global_id ?? "";
    } catch (e) { fehler = fehlertext(e); auszug = null; } finally { laedt = false; }
  }
  async function anlegen() {
    const d = dokumente.find(x => `${x.document_id}|${x.version}` === neuDatei);
    if (!d) { fehler = "Eine IFC-Datei aus den Projektdateien wählen."; return; }
    laedt = true; fehler = "";
    try {
      const id = crypto.randomUUID();
      const daten: Modellakte = { bezeichnung: neuName.trim() || d.file_name, dokument: { dokument_id: d.document_id, version: d.version } };
      const f = await plattformApi.speichern<Modellakte>(projekt.id, "ifcmodell", id, 0, daten);
      akte = f; onstatus?.({ titel: f.daten.bezeichnung, geaendert: false, id: f.objekt });
      await modellLaden();
    } catch (e) { fehler = fehlertext(e); } finally { laedt = false; }
  }
  // Nach dem Anlegen merkt sich der Tab die Modellakte; dann nicht erneut laden.
  $effect(() => { const id = start.modell; untrack(() => { if (id && id === akte?.objekt) return; void listen(); if (id) void oeffnen(id); }); });
  // Grundriss neu schneiden, wenn Geschoss oder Schnitthöhe wechseln.
  $effect(() => {
    const a = akte, g = geschoss, h = schnitt, da = !!auszug;
    if (!a || !g || !da || ansicht !== "grundriss") return;
    let weg = false;
    void ifcApi.grundriss(projekt.id, a.daten.dokument, g, Number.isFinite(h) ? h : null).then(r => { if (!weg) grundriss = r; }).catch(e => { if (!weg) fehler = fehlertext(e); });
    return () => { weg = true; };
  });
  function waehlen(gid: string) {
    auswahl = gid; rechts = "eigenschaften";
    const e = auszug?.elemente.find(x => x.global_id === gid);
    if (e?.geschoss && e.geschoss !== geschoss && ansicht === "grundriss") geschoss = e.geschoss;
  }
  const gruppen = (g: Strukturelement | null) => {
    const els = (auszug?.elemente ?? []).filter(e => (g ? e.geschoss === g.global_id : !e.geschoss));
    const m = new Map<string, Modellelement[]>();
    for (const e of els) { const k = klassentitel(e.klasse); m.set(k, [...(m.get(k) ?? []), e]); }
    return [...m].sort((a, b) => a[0].localeCompare(b[0], "de"));
  };
  const ohneGeschoss = $derived((auszug?.elemente ?? []).filter(e => !e.geschoss).length);
  const sichtbarImGrundriss = $derived(new Set(grundriss?.teile.map(t => t.global_id) ?? []));
</script>

{#if !akte}
  <section class="modellwahl" aria-label="IFC-Modell wählen">
    <h3>IFC-Modell</h3>
    <p class="meta">Ein IFC-Modell (IFC2X3, IFC4, IFC4X3; .ifc oder .ifczip) aus den Projektdateien als Modell öffnen. Die Datei bleibt unverändert; Prüfung, Nacharbeit und Übernahme speichert die Modellakte.</p>
    {#if akten.length}
      <h4>Vorhandene Modelle</h4>
      <ul class="akten">
        {#each akten as a (a.objekt)}<li><button type="button" class="link" onclick={() => void oeffnen(a.objekt)}>{a.daten.bezeichnung}</button> <span class="meta">{dokumente.find(d => d.document_id === a.daten.dokument.dokument_id)?.file_name ?? ""} · V{a.daten.dokument.version}</span></li>{/each}
      </ul>
    {/if}
    {#if darf}
      <h4>Neues Modell</h4>
      {#if dokumente.length}
        <div class="zeile">
          <label>IFC-Datei<select aria-label="IFC-Datei" bind:value={neuDatei}><option value="">Datei wählen</option>{#each dokumente as d}<option value={`${d.document_id}|${d.version}`}>{d.file_name} · V{d.version}</option>{/each}</select></label>
          <label>Bezeichnung<input aria-label="Bezeichnung des Modells" bind:value={neuName} placeholder="z. B. Entwurf Architekt"/></label>
          <button type="button" disabled={!neuDatei || laedt} onclick={() => void anlegen()}>Modell öffnen</button>
        </div>
      {:else}<p class="meta">Noch keine IFC-Datei in den Projektdateien. Die Datei zuerst unter Projektdateien ablegen.</p>{/if}
    {/if}
    {#if laedt}<p class="meta" role="status">Modell wird gelesen …</p>{/if}
    {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  </section>
{:else}
  <div class="modell" aria-label={`Modell ${akte.daten.bezeichnung}`}>
    <aside class="struktur" aria-label="Modellstruktur">
      <header><strong>{akte.daten.bezeichnung}</strong>{#if auszug}<span class="meta">{auszug.schema} · {auszug.system ?? "Software unbekannt"}</span>{/if}</header>
      {#if auszug}
        {#each auszug.gebaeude as b (b.global_id)}<button type="button" class="knoten gebaeude" class:gewaehlt={auswahl === b.global_id} onclick={() => (auswahl = b.global_id)}>Gebäude {b.name || b.langname || ""}</button>{/each}
        {#each auszug.geschosse as g (g.global_id)}
          <details open={g.global_id === geschoss}>
            <summary><button type="button" class="knoten" class:gewaehlt={auswahl === g.global_id} onclick={e => { e.preventDefault(); geschoss = g.global_id; auswahl = g.global_id; }}>{g.name || g.langname || "Geschoss"} <span class="meta">{zahl(g.hoehe_m ?? 0)} m</span></button></summary>
            {#each gruppen(g) as [titel, els] (titel)}
              <details><summary>{titel} <span class="meta">({els.length})</span></summary>
                {#each els as e (e.global_id)}<button type="button" class="knoten blatt" class:gewaehlt={auswahl === e.global_id} class:ohnegeo={!e.geometrie} onclick={() => waehlen(e.global_id)}>{elementtitel(e)}</button>{/each}
              </details>
            {/each}
          </details>
        {/each}
        {#if ohneGeschoss}
          <details><summary>Ohne Geschoss <span class="meta">({ohneGeschoss})</span></summary>
            {#each gruppen(null) as [titel, els] (titel)}
              <details><summary>{titel} <span class="meta">({els.length})</span></summary>
                {#each els as e (e.global_id)}<button type="button" class="knoten blatt" class:gewaehlt={auswahl === e.global_id} onclick={() => waehlen(e.global_id)}>{elementtitel(e)}</button>{/each}
              </details>
            {/each}
          </details>
        {/if}
      {/if}
    </aside>
    <section class="mitte" aria-label="Modellansicht">
      <div class="ansichtwahl" role="toolbar" aria-label="Ansicht">
        <div class="segment" role="radiogroup" aria-label="Darstellung">
          <button type="button" role="radio" aria-checked={ansicht === "grundriss"} onclick={() => (ansicht = "grundriss")}>Grundriss</button>
          <button type="button" role="radio" aria-checked={ansicht === "3d"} onclick={() => (ansicht = "3d")}>3D</button>
        </div>
        {#if ansicht === "grundriss" && auszug}
          <label>Geschoss<select aria-label="Geschoss" bind:value={geschoss}>{#each auszug.geschosse as g (g.global_id)}<option value={g.global_id}>{g.name || g.langname || "Geschoss"}</option>{/each}</select></label>
          <label>Schnitthöhe (m)<input aria-label="Schnitthöhe" type="number" step=".1" min="-10" max="50" bind:value={schnitt}/></label>
        {/if}
        {#if laedt}<span class="meta" role="status">Modell wird gelesen …</span>{/if}
      </div>
      {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
      {#if meldung}<p class="meta" role="status">{meldung}</p>{/if}
      <div class="flaeche">
        {#if ansicht === "3d" && akte}
          {#await import("./Ansicht3d.svelte") then m}
            <m.default laden={() => ifcApi.datei(projekt.id, akte!.daten.dokument)} {auswahl} onauswahl={waehlen}/>
          {/await}
        {:else if grundriss}
          <Grundriss {grundriss} {auswahl} onauswahl={waehlen}/>
        {:else if auszug && !auszug.geschosse.length}<p class="meta">Das Modell hat keine Geschosse; die 3D-Ansicht zeigt alle Elemente.</p>{/if}
      </div>
    </section>
    <aside class="rechts" aria-label="Details">
      <div class="segment" role="tablist" aria-label="Detailbereich">
        <button type="button" role="tab" aria-selected={rechts === "eigenschaften"} onclick={() => (rechts = "eigenschaften")}>Eigenschaften</button>
        <button type="button" role="tab" aria-selected={rechts === "bericht"} onclick={() => (rechts = "bericht")}>Modellbericht</button>
      </div>
      {#if rechts === "eigenschaften"}
        {#if element}
          <dl class="werte" aria-label="Element">
            <dt>Klasse</dt><dd>{klassentitel(element.klasse)} <span class="meta">{element.klasse}</span></dd>
            <dt>Name</dt><dd>{element.name || "—"}</dd>
            {#if element.langname}<dt>Langname</dt><dd>{element.langname}</dd>{/if}
            {#if element.vordefiniert}<dt>Typ</dt><dd>{element.vordefiniert}</dd>{/if}
            <dt>Geschoss</dt><dd>{geschossName(element.geschoss)}</dd>
            <dt>GlobalId</dt><dd class="gid">{element.global_id}</dd>
            {#if element.geometrie}
              <dt>Grundfläche</dt><dd>{flaeche(element.geometrie.grundflaeche_m2)}</dd>
              <dt>Unterkante / Oberkante</dt><dd>{laenge(element.geometrie.unten_m)} / {laenge(element.geometrie.oben_m)}</dd>
              {#if element.geometrie.volumen_m3 != null}<dt>Volumen</dt><dd>{zahl(element.geometrie.volumen_m3)} m³</dd>{/if}
              {#if element.geometrie.naeherung}<dt>Geometrie</dt><dd>angenähert (boolesche Operation oder schräger Körper)</dd>{/if}
              {#if ansicht === "grundriss" && !sichtbarImGrundriss.has(element.global_id)}<dt>Grundriss</dt><dd class="meta">nicht in der Schnittebene</dd>{/if}
            {:else}<dt>Geometrie</dt><dd>nicht ausgewertet{element.geometrie_offen.length ? ` (${element.geometrie_offen.join(", ").toLowerCase()})` : " (keine Körperdarstellung)"}</dd>{/if}
          </dl>
          {#if element.mengen.length}
            <h4>Mengen</h4>
            <table class="liste"><tbody>{#each element.mengen as q}<tr><td>{q.satz}</td><td>{q.name}</td><td class="zahl">{zahl(q.wert)}{q.art === "flaeche" ? " m²" : q.art === "laenge" ? " m" : q.art === "volumen" ? " m³" : ""}</td></tr>{/each}</tbody></table>
          {/if}
          {#if element.eigenschaften.length}
            <h4>Eigenschaften</h4>
            <table class="liste"><tbody>{#each element.eigenschaften as p}<tr><td>{p.satz}</td><td>{p.name}</td><td>{p.wert}{p.vom_typ ? " (Typ)" : ""}</td></tr>{/each}</tbody></table>
          {/if}
        {:else if struktur}
          <dl class="werte"><dt>Name</dt><dd>{struktur.name || "—"}</dd>{#if struktur.langname}<dt>Langname</dt><dd>{struktur.langname}</dd>{/if}{#if struktur.hoehe_m != null}<dt>Höhe Geschossebene</dt><dd>{laenge(struktur.hoehe_m)}</dd>{/if}<dt>GlobalId</dt><dd class="gid">{struktur.global_id}</dd></dl>
          {#if struktur.eigenschaften.length}<table class="liste"><tbody>{#each struktur.eigenschaften as p}<tr><td>{p.satz}</td><td>{p.name}</td><td>{p.wert}</td></tr>{/each}</tbody></table>{/if}
        {:else}<p class="meta">Ein Element in der Struktur, im Grundriss oder in der 3D-Ansicht wählen.</p>{/if}
      {:else if auszug}
        <dl class="werte" aria-label="Modellbericht">
          <dt>Schema</dt><dd>{auszug.schema}</dd>
          <dt>Software</dt><dd>{auszug.system ?? "nicht angegeben"}</dd>
          {#if auszug.projekt}<dt>Projekt</dt><dd>{auszug.projekt}</dd>{/if}
          <dt>Längeneinheit</dt><dd>{auszug.einheit}</dd>
          <dt>Georeferenz</dt><dd>{#if auszug.georeferenz}{auszug.georeferenz.crs ?? "Lage"}{auszug.georeferenz.ostwert != null ? ` · O ${zahl(auszug.georeferenz.ostwert)} / N ${zahl(auszug.georeferenz.nordwert ?? 0)}` : ""}{auszug.georeferenz.breite != null ? ` · ${zahl(auszug.georeferenz.breite)}° N, ${zahl(auszug.georeferenz.laenge ?? 0)}° O` : ""}{:else}keine{/if}</dd>
          <dt>Elemente</dt><dd>{auszug.bericht.elemente} ({auszug.bericht.mit_geometrie} mit Geometrie, {auszug.bericht.ohne_geometrie} ohne, {auszug.bericht.naeherungen} angenähert)</dd>
          <dt>Räume</dt><dd>{auszug.bericht.raeume} ({auszug.bericht.raeume_mit_flaechenmenge} mit Flächenmenge)</dd>
        </dl>
        <table class="liste" aria-label="Klassen"><thead><tr><th>Klasse</th><th class="zahl">Anzahl</th></tr></thead><tbody>{#each auszug.bericht.klassen as k}<tr><td>{k.titel} <span class="meta">{k.klasse}</span></td><td class="zahl">{k.anzahl}</td></tr>{/each}</tbody></table>
        {#if auszug.bericht.offene_darstellungen.length}<h4>Nicht ausgewertete Darstellungen</h4><ul>{#each auszug.bericht.offene_darstellungen as o}<li>{o.titel}: {o.anzahl} Element(e)</li>{/each}</ul>{/if}
        {#if auszug.bericht.hinweise.length}<h4>Hinweise</h4><ul>{#each auszug.bericht.hinweise as h}<li>{h}</li>{/each}</ul>{/if}
      {/if}
    </aside>
  </div>
{/if}

<style>
  .modellwahl{background:var(--panel);border:1px solid var(--border);padding:12px 16px;max-width:900px}
  .modellwahl .zeile{display:flex;gap:8px;align-items:flex-end;flex-wrap:wrap}
  .akten{margin:0 0 8px;padding-left:18px}
  .modell{display:grid;grid-template-columns:260px minmax(0,1fr) 320px;gap:0;border:1px solid var(--border);background:var(--panel);height:calc(100vh - 210px);min-height:520px}
  .struktur,.rechts{overflow:auto;padding:6px 8px;border-right:1px solid var(--border)}
  .rechts{border-right:0;border-left:1px solid var(--border)}
  .struktur header{display:flex;flex-direction:column;gap:2px;padding:2px 0 6px;border-bottom:1px solid var(--border-innen);margin-bottom:4px}
  .struktur details{margin-left:6px}
  .struktur summary{cursor:pointer;padding:1px 0}
  .knoten{display:block;width:100%;text-align:left;border:0;background:transparent;padding:1px 4px;min-height:22px;font:inherit;color:inherit;cursor:pointer}
  summary .knoten{display:inline;width:auto}
  .knoten.blatt{padding-left:14px}
  .knoten.gewaehlt{background:var(--auswahl)}
  .knoten.ohnegeo{color:var(--muted);font-style:italic}
  .mitte{display:flex;flex-direction:column;min-width:0;min-height:0}
  .ansichtwahl{display:flex;gap:8px;align-items:flex-end;padding:4px 6px;border-bottom:1px solid var(--border);flex-wrap:wrap}
  .ansichtwahl label{display:flex;flex-direction:column;font-size:.8rem}
  .ansichtwahl input{width:6rem}
  .flaeche{flex:1;min-height:0}
  .segment{display:inline-flex;border:1px solid var(--control-border)}
  .segment button{border:0;border-radius:0;min-height:24px;padding:0 10px;background:var(--knopf-bg);color:var(--fg);font:inherit;cursor:pointer}
  .segment button[aria-checked=true],.segment button[aria-selected=true]{background:var(--accent);color:var(--on-action)}
  .rechts .segment{margin-bottom:6px}
  .werte{display:grid;grid-template-columns:auto 1fr;gap:2px 8px;margin:0 0 8px}
  .werte dt{color:var(--muted)}
  .werte dd{margin:0}
  .gid{font-family:ui-monospace,monospace;font-size:.8rem;word-break:break-all}
  .liste{width:100%;border-collapse:collapse;font-size:.85rem;margin-bottom:8px}
  .liste td,.liste th{border-bottom:1px solid var(--border-innen);padding:2px 4px;text-align:left;vertical-align:top}
  .zahl{text-align:right;white-space:nowrap}
  h4{margin:8px 0 4px;font-size:.85rem}
</style>
