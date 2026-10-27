<script lang="ts">
  /** Aufmaß mit Zeichnungs-Tabs: ein Tab „Aufmaßliste“ und je offenem Aufmaß ein eigener Editor. */
  import { untrack } from "svelte";
  import type { FachProps } from "../fachmodule/register";
  import type { Datensatz } from "../plattformApi";
  import type { Planaufmass as PlanaufmassDaten } from "../generated/plattform";
  import Planaufmass from "./Planaufmass.svelte";
  import Symbolknopf from "../Symbolknopf.svelte";
  import { tabsLesen, tabsMerken, type Tabstart, type Zeichnungstab } from "./tabs";
  import { api, fehlertext } from "../api";
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
  function neu(dokument?: Tabstart["dokument"]) {
    tabAnlegen(dokument ? { dokument } : {}, "Neues Aufmaß");
  }
  function tabAnlegen(start: Tabstart, titel: string) {
    const key = crypto.randomUUID();
    tabs = [...tabs, { key, start, titel, geaendert: false }];
    aktiv = key;
  }
  function status(key: string, st: { titel: string; geaendert: boolean }) {
    const t = tabs.find(t => t.key === key);
    if (!t) return;
    if (st.titel) t.titel = st.titel;
    t.geaendert = st.geaendert;
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
  {#if imHauptfenster}<Symbolknopf klasse="fensterknopf" titel="Aufmaß in eigenem Fenster öffnen" onclick={() => void fensterOeffnen()}><WerkzeugIcon art="fenster"/></Symbolknopf>{/if}
</div>
{#if fensterFehler}<p class="fehlertext" role="alert">{fensterFehler}</p>{/if}
<section hidden={aktiv !== "liste"} aria-label="Aufmaßliste">
  <Planaufmass {projekt} {benutzerId} {darf} {onprojekt} {unterlage} modus="liste" sichtbar={aktiv === "liste"} onoeffnen={oeffnen} onneu={neu} />
</section>
{#each tabs as t (t.key)}
  <section hidden={aktiv !== t.key} aria-label={t.titel || "Aufmaß"}>
    <Planaufmass bind:this={instanzen[t.key]} {projekt} {benutzerId} {darf} {onprojekt} modus="tab" start={t.start} entwurfObjekt={t.key} sichtbar={aktiv === t.key} onstatus={st => status(t.key, st)} onschliessen={() => entfernen(t.key)} />
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
  .zeichnungstabs :global(.fensterknopf){margin-left:auto;margin-bottom:2px;align-self:center}
  .zeichnungstabs :global(.fensterknopf svg){width:16px;height:16px;stroke:currentColor;fill:none;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round}
  section[hidden]{display:none}
</style>
