<script lang="ts">
  import { untrack } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, GEBIETE, type Project, type BaugrundstuecksPlan, type BaugrundstuecksPruefung, type BaugrundstueckGruppe, type GrundstuecksFlurstueck, type Festsetzungen } from "./api";
  import { REGELWERKE, regelwerk } from "./baunvo";
  import ABZUEGE from "../../../../regelwerke/grundstueck-abzuege.json";
  import FlaechenFormular from "./FlaechenFormular.svelte";
  import GrundstueckFormular from "./GrundstueckFormular.svelte";
  let { projekt, darf, onprojekt }: { projekt: Project; darf: boolean; onprojekt: (p: Project) => void } = $props();
  const initial = (): BaugrundstuecksPlan => structuredClone($state.snapshot(projekt.planungsrecht.baugrundstuecke ?? { flurstuecke: [], berechnungen: [] }));
  let eingabe = $state(untrack(initial));
  let gespeichert = $state(untrack(initial));
  let ergebnis = $state<BaugrundstuecksPruefung | null>(null);
  let ergebnisStand = $state("");
  let laeuft = $state(false);
  let fehler = $state<string | null>(null);
  let hinweis = $state<string | null>(null);
  let formular: HTMLFormElement;
  const geaendert = $derived(JSON.stringify(eingabe) !== JSON.stringify(gespeichert));
  const fingerprint = $derived(JSON.stringify([eingabe, projekt.version]));
  const aktuell = $derived(ergebnisStand === fingerprint);
  const mitPlan = $derived(!["kein", "ungeklaert"].includes(projekt.planungsrecht.bebauungsplan_art));
  const z = (n: number | null) => n == null ? "–" : n.toLocaleString("de-DE", { maximumFractionDigits: 6 });
  const titel: Record<string,string> = { grz:"GRZ I / GRZ", grz_gesamt:"GRZ II", gfz:"GFZ", bmz:"BMZ", grundflaeche:"Grundfläche (m²)", geschossflaeche:"Geschossfläche (m²)", baufensterflaeche:"Baufenster-Flächenvergleich (m²)", vollgeschosse:"Vollgeschosse" };
  const quellen = { festsetzung:"Bebauungsplan",orientierungswert:"Orientierungswert § 17 BauNVO",bau_nvo19:"§ 19 Abs. 4",keine:"Nicht festgelegt" };
  const festLabels: Record<string,string> = {grz:"GRZ I / GRZ",grz_gesamt:"GRZ II",gfz:"GFZ",bmz:"BMZ",grundflaeche_m2:"Grundfläche (m²)",geschossflaeche_m2:"Geschossfläche (m²)",ueberbaubare_flaeche_m2:"Baufensterfläche (m²)",vollgeschosse:"Vollgeschosse"};
  function schluessel(f: GrundstuecksFlurstueck): string { return f.abweichende_zuordnung ? `zuordnung:${f.abweichende_zuordnung.kennung.trim().toLowerCase()}` : JSON.stringify([f.grundbuchbezirk.trim().toLowerCase(), f.grundbuchblatt.trim().toLowerCase(), f.laufende_nummer]); }
  const gruppen = $derived.by(() => {
    const map = new Map<string, BaugrundstueckGruppe>();
    for (const f of eingabe.flurstuecke) {
      if (!f.grundbuchbezirk.trim() || !f.grundbuchblatt.trim() || !(f.laufende_nummer > 0) || (f.abweichende_zuordnung && !f.abweichende_zuordnung.kennung.trim())) continue;
      const key = schluessel(f);
      const g = map.get(key) ?? { schluessel:key, bezeichnung:f.abweichende_zuordnung ? `Zuordnung ${f.abweichende_zuordnung.kennung}` : `${f.grundbuchbezirk} · Blatt ${f.grundbuchblatt} · lfd. Nr. ${f.laufende_nummer}`, flurstueck_ids:[], eigentumsflaeche_m2:0, abzuege_m2:0, bauland_m2:0 };
      g.flurstueck_ids.push(f.id); g.eigentumsflaeche_m2 += f.flaeche_m2 || 0; g.abzuege_m2 += f.abzuege.reduce((s,a)=>s+(a.flaeche_m2||0),0); g.bauland_m2=g.eigentumsflaeche_m2-g.abzuege_m2; map.set(key,g);
    }
    return [...map.values()].sort((a,b)=>a.schluessel.localeCompare(b.schluessel));
  });
  const verwaist = $derived(eingabe.berechnungen.filter(b=>!gruppen.some(g=>g.schluessel===b.schluessel)));
  $effect(() => { const p=projekt; untrack(()=> { if (!geaendert) { gespeichert=structuredClone($state.snapshot(p.planungsrecht.baugrundstuecke ?? {flurstuecke:[],berechnungen:[]})); eingabe=structuredClone($state.snapshot(gespeichert)); } }); });
  export async function verwerfenErlaubt(): Promise<boolean> { return !geaendert || await ask("Flurstücke und Grundstücksberechnungen sind noch nicht gespeichert. Verwerfen?",{title:"Baugrundstücke",kind:"warning",okLabel:"Verwerfen",cancelLabel:"Abbrechen"}); }
  function hinzufuegen() { eingabe.flurstuecke.push({ id:crypto.randomUUID(),gemarkung:"",flur:"",nummer:"",grundbuchbezirk:"",grundbuchblatt:"",laufende_nummer:1,flaeche_m2:0,abzuege:[],abweichende_zuordnung:null }); }
  function anlegen(g: BaugrundstueckGruppe) { eingabe.berechnungen.push({schluessel:g.schluessel,eigene_festsetzungen:null,eingabe:{gebiet:projekt.planungsrecht.festgesetzte_werte.baugebiet??"WA",vollgeschosse:null,mass:{grundstuecksflaeche_m2:g.bauland_m2,grundflaeche_m2:0,grundflaeche_nebenanlagen_m2:0,geschossflaeche_m2:0,baumasse_m3:null},aufstellung:{bau_nvo:"2017",bauteile:[],geschosse:[],nichtvollgeschoss_faktor:regelwerk("2017").nichtvollgeschoss_faktor,nichtvollgeschoss_festsetzung:"",grundstueck:{basisflaeche_m2:g.eigentumsflaeche_m2,nicht_massgebend_m2:g.abzuege_m2,abzug_begruendung:"Flurstücksabzüge",gemeinschaftsanteile:[]}}}}); }
  function request(): BaugrundstuecksPlan {
    const e=structuredClone($state.snapshot(eingabe));
    for (const b of e.berechnungen) { b.eingabe.mass.baumasse_m3 ??= null; b.eingabe.vollgeschosse ??= null; }
    return e;
  }
  async function berechnen(event: SubmitEvent) { event.preventDefault(); laeuft=true;fehler=null;hinweis=null; const stand=fingerprint; try { const r=await api.projektBaugrundstueckePruefen(projekt.id,request()); if(r.projekt_version!==projekt.version) throw new Error("Projektstand geändert. Bitte Projekt erneut öffnen."); ergebnis=r;ergebnisStand=stand; } catch(e) {ergebnis=null;fehler=fehlertext(e);} finally {laeuft=false;} }
  async function speichern() {
    if (!formular.reportValidity()) return;
    laeuft=true;fehler=null;hinweis=null;
    try { const e=request();const r=await api.projektBaugrundstueckePruefen(projekt.id,e);if(r.projekt_version!==projekt.version) throw new Error("Projektstand geändert. Bitte Projekt erneut öffnen.");
      const p=await api.projektAktualisieren({id:projekt.id,expected_version:projekt.version,name:projekt.name,bundesland:projekt.bundesland,grundstuecksflaeche_m2:e.flurstuecke.reduce((s,f)=>s+f.flaeche_m2,0),planungsrecht:{...structuredClone($state.snapshot(projekt.planungsrecht)),baugrundstuecke:e}});
      gespeichert=structuredClone($state.snapshot(p.planungsrecht.baugrundstuecke!));eingabe=structuredClone($state.snapshot(gespeichert));ergebnis=null;onprojekt(p);hinweis="Flurstücke und Berechnungen gespeichert.";
    } catch(e){fehler=fehlertext(e);} finally{laeuft=false;}
  }
</script>
<p class="meta">Die grundbuchliche Einheit ergibt sich aus Grundbuchbezirk, Blatt und laufender Nummer im Bestandsverzeichnis. Flurstücke derselben Einheit werden zusammengefasst; unterschiedliche Einheiten getrennt berechnet. Besondere rechtliche Zuordnungen können mit ihrer Grundlage erfasst werden.</p>
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}{#if hinweis}<p role="status">{hinweis}</p>{/if}
<form bind:this={formular} onsubmit={berechnen} class="flaechenformular flurstuecksformular">
  <h3>Flurstücke und Flächenabzüge</h3>
  {#each eingabe.flurstuecke as f, i (f.id)}
    <fieldset><legend>Flurstück {i+1}</legend><div class="raster">
      <label>Gemarkung<input bind:value={f.gemarkung} required maxlength="500" /></label><label>Flur (optional)<input bind:value={f.flur} maxlength="500" /></label><label>Flurstücksnummer<input bind:value={f.nummer} required maxlength="500" /></label>
      <label>Grundbuchbezirk<input bind:value={f.grundbuchbezirk} required maxlength="500" /></label><label>Grundbuchblatt<input bind:value={f.grundbuchblatt} required maxlength="500" /></label><label>Laufende Nummer im Bestandsverzeichnis<input type="number" min="1" step="1" bind:value={f.laufende_nummer} required /></label>
      <label>Flurstücksfläche (m²)<input type="number" min="0.01" step="any" bind:value={f.flaeche_m2} required /></label>
    </div>
    {#each f.abzuege as a, j}<fieldset><legend>Abzugsfläche {j+1}</legend><div class="raster">
      <label>Art<select bind:value={a.art}>{#each Object.entries(ABZUEGE) as [code,r]}<option value={code}>{r.bezeichnung}</option>{/each}</select></label><label>Fläche (m²)<input type="number" min="0" step="any" bind:value={a.flaeche_m2} required /></label><label>Planfestsetzung / Grundlage<input bind:value={a.grundlage} required maxlength="4000" /></label>
    </div><button type="button" class="sekundaer" onclick={()=>f.abzuege.splice(j,1)}>Abzug entfernen</button></fieldset>{/each}
    <div class="knoepfe links"><button type="button" class="sekundaer" onclick={()=>f.abzuege.push({art:mitPlan?"private_gruenflaeche":"ausserhalb_bauland",flaeche_m2:0,grundlage:""})}>Abzugsfläche hinzufügen</button><button type="button" class="sekundaer" onclick={()=>eingabe.flurstuecke.splice(i,1)}>Flurstück entfernen</button></div>
    <details><summary>Abweichende rechtliche Grundstückszuordnung</summary><p class="meta">Nur bei einer belegten besonderen Zuordnung. Gleiche Kennungen werden zusammengefasst. Unterschiedliche Kennungen können eine grundbuchliche Einheit aufteilen.</p>
      {#if f.abweichende_zuordnung}<div class="raster"><label>Kennung des Baugrundstücks<input bind:value={f.abweichende_zuordnung.kennung} required maxlength="500" /></label><label>Rechtliche Grundlage / Nachweis<input bind:value={f.abweichende_zuordnung.grundlage} required maxlength="4000" /></label></div><button type="button" class="sekundaer" onclick={()=>f.abweichende_zuordnung=null}>Grundbuchliche Zuordnung verwenden</button>
      {:else}<button type="button" class="sekundaer" onclick={()=>f.abweichende_zuordnung={kennung:"",grundlage:""}}>Besondere Zuordnung erfassen</button>{/if}
    </details></fieldset>
  {/each}
  <button type="button" class="sekundaer" onclick={hinzufuegen}>Flurstück hinzufügen</button>
  <h3>Baugrundstücke und einzelne Berechnungen</h3>
  <p class="meta">Abzüge betreffen die maßgebende Baulandfläche, nicht die Eigentumsfläche. Überlagerte Abzugsflächen nur einmal erfassen. Eine nicht überbaubare Fläche außerhalb des Baufensters ist allein kein Grundstücksflächenabzug.</p>
  {#each gruppen as g (g.schluessel)}
    {@const b=eingabe.berechnungen.find(b=>b.schluessel===g.schluessel)}
    {@const r=aktuell ? ergebnis?.baugrundstuecke.find(r=>r.grundstueck.schluessel===g.schluessel)?.pruefung ?? null : null}
    <fieldset><legend>{g.bezeichnung}</legend><p>{g.flurstueck_ids.length} Flurstück(e): {z(g.eigentumsflaeche_m2)} m² − {z(g.abzuege_m2)} m² Abzüge = <strong>{z(g.bauland_m2)} m² Bauland</strong></p>
      <p class="meta">{eingabe.flurstuecke.filter(f=>g.flurstueck_ids.includes(f.id)).map(f=>`${f.gemarkung} ${f.flur ? `Flur ${f.flur} ` : ""}${f.nummer}`).join(" · ")}</p>
      {#if !b}<button type="button" onclick={()=>anlegen(g)}>Berechnung für dieses Baugrundstück anlegen</button>
      {:else if b.eingabe.aufstellung}
        <p class="meta">{mitPlan ? "Vergleich mit den Projektfestsetzungen oder eigenen Festsetzungen dieser Grundstückseinheit." : "Ohne Bebauungsplan: Orientierungswerte nach § 17 BauNVO, keine verbindliche Zulässigkeitsentscheidung."}</p>
        <label>Baugebiet<select bind:value={b.eingabe.gebiet}>{#each GEBIETE as gebiet}<option value={gebiet.code}>{gebiet.code} – {gebiet.name}</option>{/each}</select></label>
        <label>BauNVO-Fassung<select bind:value={b.eingabe.aufstellung.bau_nvo}>{#each REGELWERKE as r}<option value={r.fassung}>BauNVO {r.fassung}</option>{/each}</select></label>
        <label>Baumasse dieses Baugrundstücks (m³, optional)<input type="number" min="0" step="any" bind:value={b.eingabe.mass.baumasse_m3} /></label>
        <details><summary>Zugeordnete Gemeinschaftsflächen</summary><GrundstueckFormular aufstellung={b.eingabe.aufstellung} basisFallback={g.eigentumsflaeche_m2} ergebnis={r?.aufstellung?.grundstueck??null} {mitPlan} flurstuecksBasis={{basis:g.eigentumsflaeche_m2,abzug:g.abzuege_m2}} /></details>
        <details><summary>Eigene Festsetzungen für dieses Baugrundstück</summary><p class="meta">Ohne eigene Angaben gelten die Projektfestsetzungen. Eigene Angaben ersetzen diese vollständig; leere Werte bleiben ungeprüft.</p>
          {#if b.eigene_festsetzungen}<div class="raster">{#each ["grz","grz_gesamt","gfz","bmz","grundflaeche_m2","geschossflaeche_m2","ueberbaubare_flaeche_m2","vollgeschosse"] as key}<label>{festLabels[key]}<input type="number" min="0" step={key==="vollgeschosse"?"1":"any"} bind:value={b.eigene_festsetzungen[key as keyof Pick<Festsetzungen,"grz"|"grz_gesamt"|"gfz"|"bmz"|"grundflaeche_m2"|"geschossflaeche_m2"|"ueberbaubare_flaeche_m2"|"vollgeschosse">]} /></label>{/each}
          <label>Baugebiet laut Plan<select bind:value={b.eigene_festsetzungen.baugebiet}><option value={null}>Nicht festgelegt</option>{#each GEBIETE as gebiet}<option value={gebiet.code}>{gebiet.code} – {gebiet.name}</option>{/each}</select></label>
          <label>Geschossigkeit<select bind:value={b.eigene_festsetzungen.geschossigkeit}><option value="maximal">Maximal</option><option value="zwingend">Zwingend</option></select></label><label>Nichtvollgeschoss-Anteil (optional)<input type="number" min="0" max="1" step="any" bind:value={b.eigene_festsetzungen.nichtvollgeschoss_faktor} /></label><label>Planfestsetzung Nichtvollgeschosse<input bind:value={b.eigene_festsetzungen.nichtvollgeschoss_festsetzung} required={(b.eigene_festsetzungen.nichtvollgeschoss_faktor??0)>0} /></label></div><button type="button" class="sekundaer" onclick={()=>b.eigene_festsetzungen=null}>Projektfestsetzungen verwenden</button>
          {:else}<button type="button" class="sekundaer" onclick={()=>b.eigene_festsetzungen=structuredClone($state.snapshot(projekt.planungsrecht.festgesetzte_werte))}>Eigene Festsetzungen erfassen</button>{/if}
        </details>
        <FlaechenFormular bind:aufstellung={b.eingabe.aufstellung} festsetzungen={mitPlan ? (b.eigene_festsetzungen??projekt.planungsrecht.festgesetzte_werte) : {...projekt.planungsrecht.festgesetzte_werte,nichtvollgeschoss_faktor:null}} ergebnis={r?.aufstellung??null} />
        {#if r}<p class="hinweisbox">Maßgebende Bezugsfläche: <strong>{z(r.aufstellung?.mass.grundstuecksflaeche_m2??g.bauland_m2)} m²</strong>, einschließlich zugeordneter Gemeinschaftsanteile.</p><div class="tabellen-scroll"><table class="ergebnistabelle"><thead><tr><th>Nachweis</th><th>Ist</th><th>Soll</th><th>Quelle</th><th>Überschreitung / Abweichung</th><th>Ergebnis</th></tr></thead><tbody>{#each r.nachweise as n}{#if !(n.art==="grz_gesamt" && !regelwerk(b.eingabe.aufstellung.bau_nvo).grz_ii)}<tr><td>{n.art === "grz" ? (regelwerk(b.eingabe.aufstellung.bau_nvo).grz_ii ? "GRZ I" : "GRZ") : titel[n.art]}</td><td>{z(n.ist)}</td><td>{z(n.soll)}</td><td>{quellen[n.quelle]}</td><td>{z(n.zwingend?n.abweichung:n.ueberschreitung)}{n.ueberschreitung_prozent != null && n.ueberschreitung_prozent > 0 ? ` (${z(n.ueberschreitung_prozent)} %)` : ""}{n.flaechenueberschreitung_m2?` · ${z(n.flaechenueberschreitung_m2)} m²`:""}</td><td class:verstoss={n.eingehalten===false}>{n.soll==null?"Nicht festgelegt":n.ist==null?"Eingabe fehlt":n.eingehalten?"✓ Vergleichswert eingehalten":"✗ Abweichung"}</td></tr>{/if}{/each}</tbody></table></div>{/if}
      {/if}
    </fieldset>
  {/each}
  {#each verwaist as b}<p class="hinweisbox">Berechnung ohne passende Flurstücke: {b.schluessel}. Bitte Zuordnung korrigieren oder ausdrücklich entfernen.</p><button type="button" class="sekundaer" onclick={async()=>{if(await ask("Diese Berechnung mit allen Eingaben entfernen?",{title:"Verwaiste Berechnung",kind:"warning"}))eingabe.berechnungen=eingabe.berechnungen.filter(x=>x!==b);}}>Verwaiste Berechnung entfernen</button>{/each}
  {#if ergebnis && !aktuell}<p class="hinweisbox">Eingaben oder Projektfestsetzungen geändert. Bitte neu berechnen.</p>{/if}
  <p class="meta">Die Nachweise betreffen GRZ, GFZ, BMZ, Geschossigkeit und Flächengrenzen. Die Lage innerhalb der Baugrenzen wird weiterhin unter „Pläne“ geometrisch geprüft.</p>
  <div class="knoepfe links"><button type="submit" disabled={laeuft}>Alle Baugrundstücke berechnen</button><button type="button" class="sekundaer" onclick={speichern} disabled={!darf||laeuft||!geaendert}>Flurstücke und Berechnungen speichern</button></div>
</form>
