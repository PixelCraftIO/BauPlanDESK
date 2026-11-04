<script lang="ts">
  /** Aufmaß mit Zeichnungs-Tabs: ein Tab „Aufmaßliste“ und je offenem Aufmaß ein eigener Editor. */
  import { untrack } from "svelte";
  import type { FachProps } from "../fachmodule/register";
  import type { Datensatz } from "../plattformApi";
  import type { Planaufmass as PlanaufmassDaten } from "../generated/plattform";
  import Planaufmass from "./Planaufmass.svelte";
  import IfcModell from "../ifc/IfcModell.svelte";
  import Symbolknopf from "../Symbolknopf.svelte";
  import { tabsLesen, tabsMerken, type Tabstart, type Zeichnungstab } from "./tabs";
  import { api, fehlertext, istPlan, type DocumentVersion } from "../api";
  import { plattformApi } from "../plattformApi";
  import { projektNavigation } from "../projektNavigation.svelte";
  import WerkzeugIcon from "./WerkzeugIcon.svelte";
  let { projekt, benutzerId, darf, onprojekt, unterlage = null }: FachProps = $props();
  const gemerkt = untrack(() => tabsLesen(benutzerId, projekt.id));
  let tabs = $state<Zeichnungstab[]>(gemerkt.tabs);
  let aktiv = $state(gemerkt.aktiv);
  const instanzen: Record<string, Planaufmass | undefined> = {};
  // Nur das Hauptfenster bietet das eigene Editorfenster an.
  const imHauptfenster = !new URLSearchParams(location.search).has("fenster");
  let fensterFehler = $state("");
  async function fensterOeffnen() {
    fensterFehler = "";
    try { await api.editorFenster(projekt.id); } catch (e) { fensterFehler = fehlertext(e); }
  }
  $effect(() => {
    if (!tabs.some(t => t.key === aktiv) && aktiv !== "liste") aktiv = "liste";
  });
  $effect(() => { tabsMerken(benutzerId, projekt.id, tabs, aktiv); });
  function oeffnen(f: Datensatz<PlanaufmassDaten>) {
    const da = tabs.find(t => t.start.id === f.objekt);
    if (da) { aktiv = da.key; return; }
    tabAnlegen({ id: f.objekt }, f.daten.bezeichnung);
  }
  function neu(start?: Tabstart) {
    tabAnlegen(start ?? {}, start?.ifc ? "IFC-Modell" : start?.zeichnung ? "Neue Zeichnung" : "Neues Aufmaß");
  }
  function tabAnlegen(start: Tabstart, titel: string) {
    const key = crypto.randomUUID();
    tabs = [...tabs, { key, start, titel, geaendert: false }];
    aktiv = key;
  }
  function status(key: string, st: { titel: string; geaendert: boolean; id?: string | null }) {
    const t = tabs.find(t => t.key === key);
    if (!t) return;
    if (st.titel) t.titel = st.titel;
    t.geaendert = st.geaendert;
    // Nach dem ersten Speichern kennt der Tab sein Aufmaß; erneutes Öffnen springt ihn dann an.
    if (st.id && t.start.ifc && t.start.modell !== st.id) t.start = { ...t.start, modell: st.id };
    else if (st.id && !t.start.ifc && t.start.id !== st.id) t.start = { ...t.start, id: st.id };
  }
  function entfernen(key: string) {
    const i = tabs.findIndex(t => t.key === key);
    if (i < 0) return;
    tabs = tabs.filter(t => t.key !== key);
    delete instanzen[key];
    if (aktiv === key) aktiv = tabs[Math.min(i, tabs.length - 1)]?.key ?? "liste";
  }
  async function schliessen(key: string) {
    const inst = instanzen[key];
    if (inst && !(await inst.tabSchliessen())) return;
    entfernen(key);
  }
  // Flurstücksauftrag aus den Grundstücksinformationen: Unterlage wählen, dann
  // den Tab mit dem Auftrag öffnen. Gibt es genau eine Möglichkeit, gleich diese.
  const navigation = projektNavigation();
  let flur = $state<{ id: string; bezeichnung: string } | null>(null);
  let flurAufmasse = $state<Datensatz<PlanaufmassDaten>[]>([]), flurDateien = $state<DocumentVersion[]>([]), flurWahl = $state(""), flurFehler = $state("");
  let flurGeneration = 0;
  $effect(() => {
    const a = navigation?.flurstuecksauftrag;
    if (!a) return;
    untrack(() => {
      navigation!.flurstuecksauftrag = null;
      const n = ++flurGeneration, auftrag = { id: a.flurstueck_id, bezeichnung: a.bezeichnung };
      flur = auftrag; flurWahl = ""; flurFehler = ""; flurAufmasse = []; flurDateien = [];
      aktiv = "liste";
      if (a.aufmass_id) { flurOeffnen(`a:${a.aufmass_id}`); return; }
      void Promise.all([plattformApi.liste<PlanaufmassDaten>(projekt.id, "planaufmass"), api.dokumente(projekt.id), navigation?.kategorienLaden()])
        .then(([liste, dokumente]) => {
          if (n !== flurGeneration) return;
          // Je Unterlage die neueste Version; Unterlagen mit eigenem Aufmaß erreicht man über dieses.
          const neueste = new Map<string, DocumentVersion>();
          for (const d of dokumente.filter((d) => istPlan(d.file_name)))
            if ((neueste.get(d.document_id)?.version ?? 0) < d.version) neueste.set(d.document_id, d);
          const mitAufmass = new Set(liste.map((x) => x.daten.dokument?.dokument_id));
          const karte = (d: DocumentVersion) => /flurkarte|liegenschaft|lageplan/i.test(`${navigation?.kategorien[d.document_id] ?? ""} ${d.file_name}`) ? 0 : 1;
          flurAufmasse = liste;
          flurDateien = [...neueste.values()].filter((d) => !mitAufmass.has(d.document_id))
            .sort((x, y) => karte(x) - karte(y) || x.file_name.localeCompare(y.file_name, "de"));
          const wahl = [...flurAufmasse.map((x) => `a:${x.objekt}`), ...flurDateien.map((d) => `d:${d.document_id}`)];
          if (wahl.length === 1) flurOeffnen(wahl[0]);
          else flurWahl = wahl[0] ?? "z";
        })
        .catch((e) => { if (n === flurGeneration) flurFehler = fehlertext(e); });
    });
  });
  function flurOeffnen(wahl: string) {
    const flurstueck = flur;
    if (!flurstueck) return;
    flur = null;
    const id = wahl.slice(2);
    if (wahl.startsWith("a:")) {
      const da = tabs.find((t) => t.start.id === id);
      if (da) { da.start = { ...da.start, flurstueck }; aktiv = da.key; }
      else tabAnlegen({ id, flurstueck }, flurAufmasse.find((x) => x.objekt === id)?.daten.bezeichnung ?? "Aufmaß");
    } else if (wahl.startsWith("d:")) {
      const d = flurDateien.find((x) => x.document_id === id);
      if (d) tabAnlegen({ dokument: { id: d.document_id, version: d.version, name: d.file_name }, flurstueck }, "Neues Aufmaß");
    } else tabAnlegen({ zeichnung: true, flurstueck }, "Neue Zeichnung");
  }
  function flurErledigt(key: string) {
    const t = tabs.find((t) => t.key === key);
    if (t?.start.flurstueck) { const { flurstueck: _f, ...rest } = t.start; void _f; t.start = rest; }
  }
  function taste(e: KeyboardEvent, i: number) {
    const reihe = ["liste", ...tabs.map(t => t.key)];
    const j = e.key === "ArrowRight" ? i + 1 : e.key === "ArrowLeft" ? i - 1 : e.key === "Home" ? 0 : e.key === "End" ? reihe.length - 1 : -1;
    if (j < 0 || j >= reihe.length) return;
    e.preventDefault(); aktiv = reihe[j];
    (e.currentTarget as HTMLElement).parentElement?.querySelectorAll<HTMLElement>("[role=tab]")[j]?.focus();
  }
</script>

<div class="zeichnungstabs" role="tablist" aria-label="Zeichnungen">
  <button type="button" role="tab" class="zeichnungstab" aria-selected={aktiv === "liste"} tabindex={aktiv === "liste" ? 0 : -1} onclick={() => (aktiv = "liste")} onkeydown={e => taste(e, 0)}>Aufmaßliste</button>
  {#each tabs as t, i (t.key)}
    <div class="zeichnungstab" class:aktiv={aktiv === t.key} class:geaendert={t.geaendert}>
      <button type="button" role="tab" aria-selected={aktiv === t.key} tabindex={aktiv === t.key ? 0 : -1} onclick={() => (aktiv = t.key)} onkeydown={e => taste(e, i + 1)}>{t.titel || "Aufmaß"}{#if t.geaendert}<span class="punkt" aria-label="ungespeichert"></span>{/if}</button>
      <Symbolknopf klasse="tab-schliessen" titel={`${t.titel || "Aufmaß"} schließen`} onclick={() => void schliessen(t.key)}><svg viewBox="0 0 16 16"><path d="M4 4l8 8M12 4l-8 8"/></svg></Symbolknopf>
    </div>
  {/each}
  {#if darf}<button type="button" class="sekundaer neuezeichnung" title="Eigene Zeichnung in Metern anlegen; PDF oder DXF als Hintergrund einpassbar" onclick={() => neu({ zeichnung: true })}>Neue Zeichnung</button>{/if}
  <button type="button" class="sekundaer neuezeichnung ifcknopf" title="IFC-Modell aus den Projektdateien ansehen, prüfen, nacharbeiten und auswerten" onclick={() => neu({ ifc: true })}>IFC-Modell</button>
  {#if imHauptfenster}<Symbolknopf klasse="fensterknopf" titel="Aufmaß in eigenem Fenster öffnen" onclick={() => void fensterOeffnen()}><WerkzeugIcon art="fenster"/></Symbolknopf>{/if}
</div>
{#if fensterFehler}<p class="fehlertext" role="alert">{fensterFehler}</p>{/if}
{#if flur}<aside class="hinweisbox flurwahl" aria-label="Flurstücksfläche ermitteln">
    <strong>Flurstück {flur.bezeichnung}: Fläche im Zeicheneditor ermitteln</strong>
    <label>Zeichnen auf<select aria-label="Zeichnen auf" bind:value={flurWahl}>
      {#if flurAufmasse.length}<optgroup label="Vorhandenes Aufmaß">{#each flurAufmasse as x (x.objekt)}<option value={`a:${x.objekt}`}>{x.daten.bezeichnung}</option>{/each}</optgroup>{/if}
      {#if flurDateien.length}<optgroup label="Neues Aufmaß auf Projektdatei">{#each flurDateien as d (d.document_id)}<option value={`d:${d.document_id}`}>{d.file_name}{navigation?.kategorien[d.document_id] ? ` · ${navigation.kategorien[d.document_id]}` : ""}</option>{/each}</optgroup>{/if}
      <option value="z">Eigene Zeichnung in Metern</option>
    </select></label>
    <button type="button" onclick={() => flurOeffnen(flurWahl)}>Öffnen</button>
    <button type="button" class="link" onclick={() => (flur = null)}>Abbrechen</button>
    {#if !flurAufmasse.length && !flurDateien.length}<span class="meta">Keine Flurkarte oder kein Lageplan in den Projektdateien. Unter Projektdateien eine PDF oder DXF hinzufügen oder die Grenze als eigene Zeichnung nach Maßen konstruieren.</span>{/if}
    {#if flurFehler}<span class="fehlertext" role="alert">{flurFehler}</span>{/if}
  </aside>{/if}
<section hidden={aktiv !== "liste"} aria-label="Aufmaßliste">
  <Planaufmass {projekt} {benutzerId} {darf} {onprojekt} {unterlage} modus="liste" sichtbar={aktiv === "liste"} onoeffnen={oeffnen} onneu={neu} />
</section>
{#each tabs as t (t.key)}
  <section hidden={aktiv !== t.key} aria-label={t.titel || "Aufmaß"}>
    {#if t.start.ifc}
      <IfcModell {projekt} {darf} start={t.start} onstatus={st => status(t.key, st)} onzeichnung={(id, titel) => { const da = tabs.find(x => x.start.id === id); if (da) aktiv = da.key; else tabAnlegen({ id }, titel); }} />
    {:else}
    <Planaufmass bind:this={instanzen[t.key]} {projekt} {benutzerId} {darf} {onprojekt} modus="tab" start={t.start} entwurfObjekt={t.key} sichtbar={aktiv === t.key} onstatus={st => status(t.key, st)} onschliessen={() => entfernen(t.key)} onflurstueckfertig={() => flurErledigt(t.key)} />
    {/if}
  </section>
{/each}

<style>
  .zeichnungstabs{display:flex;gap:2px;align-items:flex-end;padding:4px 0 0;margin:0 0 8px;border-bottom:1px solid var(--border);overflow-x:auto}
  .zeichnungstab{display:flex;align-items:center;flex:none;max-width:260px;margin-bottom:-1px;border:1px solid var(--border);border-bottom-color:transparent;background:var(--leiste);color:var(--muted)}
  button.zeichnungstab,.zeichnungstab > [role=tab]{min-height:28px;padding:0 10px;border:0;border-radius:0;background:transparent;color:inherit;font:inherit;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;cursor:pointer}
  .zeichnungstab.aktiv,button.zeichnungstab[aria-selected=true]{background:var(--panel);color:var(--fg);box-shadow:inset 0 2px 0 var(--accent)}
  .zeichnungstab > [role=tab]:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}
  .punkt{display:inline-block;width:7px;height:7px;margin-left:6px;background:var(--warning-text,#6b4a00);vertical-align:middle}
  .zeichnungstab :global(.tab-schliessen){width:22px;min-height:22px;margin-right:3px}
  .zeichnungstab :global(.tab-schliessen svg){width:12px;height:12px;stroke:currentColor;fill:none;stroke-width:1.6;stroke-linecap:round}
  .zeichnungstabs .neuezeichnung{margin-left:auto;margin-bottom:2px;align-self:center;min-height:26px;padding:0 10px;font-size:.85rem;white-space:nowrap}
  .zeichnungstabs .neuezeichnung + .ifcknopf{margin-left:4px}
  .zeichnungstabs .neuezeichnung + :global(.fensterknopf){margin-left:4px}
  .zeichnungstabs :global(.fensterknopf){margin-left:auto;margin-bottom:2px;align-self:center}
  .zeichnungstabs :global(.fensterknopf svg){width:16px;height:16px;stroke:currentColor;fill:none;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round}
  section[hidden]{display:none}
  .flurwahl{display:flex;flex-wrap:wrap;gap:.4rem .8rem;align-items:end;margin:0 0 8px}
  .flurwahl > strong,.flurwahl > span{flex-basis:100%}
</style>
