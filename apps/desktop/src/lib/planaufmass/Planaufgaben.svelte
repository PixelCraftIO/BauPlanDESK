<script lang="ts">
  import { api, fehlertext, type Project } from "../api";
  import type { Planaufgabe, Planaufmass, Gebaeude, AbstandsPlan, FachPlan, Raumbuch, Grundstuecksinformationen } from "../generated/plattform";
  import type { ObjektArt } from "../generated/plattform";
  import { grundstuecksinformationen } from "../grundstuecksdaten";
  import { plattformApi } from "../plattformApi";
  import { planApi } from "./api";
  import { bereichAktiv, ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  import { Uebernahme, zielfeld, werttext, type Wertvergleich } from "./Uebernahme.svelte";
  import UebernahmeVorschau from "./UebernahmeVorschau.svelte";
  const v=new Uebernahme();
  const einheit=(a:Planaufgabe)=>a.groesse==="flaeche"?"m²":a.groesse==="neigung"?"°":"m";
  // Fachobjektart, unter der ein Verbraucher gespeichert wird.
  const artVon=(a:Planaufgabe):ObjektArt=>a.verbraucher==="gebaeude"?"gebaeude":a.verbraucher==="raumbuch"?"raumbuch":a.verbraucher==="berechnung"?(a.modul as ObjektArt):"abstandsflaechen";
  // Eingaben, in denen das Zielfeld liegt: bei Fachberechnungen die Eingabetabellen.
  const zielwerte=(a:Planaufgabe,daten:unknown)=>a.verbraucher==="berechnung"?(daten as FachPlan).eingaben:daten;
  // Was das Projekt aus diesem Aufmaß noch braucht. Die Liste kommt vom Core;
  // „Zeichnen“ belegt Werkzeug und Zuordnung vor, „Übernehmen“ verknüpft die
  // Markierung über die bestehenden Übernahmedienste und speichert den Verbraucher.
  let { projektId, projekt, onprojekt, plan, aufmassId, gespeichert, darf, onzeichnen, onwaehlen }: {
    projektId: string; plan: Planaufmass; aufmassId: string;
    /** Projektstand der Akte: Grundlage für die Übernahme in Flurstücke; der gespeicherte Stand geht über `onprojekt` zurück. */
    projekt: Project; onprojekt?: (p: Project) => void;
    /** Aufmaß ist gespeichert und ohne offene Änderungen; nur dann ist eine Übernahme möglich. */
    gespeichert: boolean; darf: boolean;
    onzeichnen: (a: Planaufgabe) => void; onwaehlen: (markierung: string) => void;
  } = $props();
  let aufgaben = $state<Planaufgabe[]>([]), fehler = $state(""), meldung = $state(""), busy = $state("");
  let filter = $state(""), nurOffen = $state(true);
  let generation = 0;
  function laden(snapshot = JSON.stringify(plan)) {
    const n = ++generation;
    return planApi.aufgaben(projektId, JSON.parse(snapshot)).then((a) => { if (n === generation) { aufgaben = a; fehler = ""; } })
      .catch((e) => { if (n === generation) fehler = fehlertext(e); });
  }
  const aktiv=bereichAktiv();
  $effect(() => {
    if(!aktiv()) return;
    const snapshot = JSON.stringify(plan);
    const timer = setTimeout(() => laden(snapshot), 300);
    return () => clearTimeout(timer);
  });
  // Gebäude oder Abstandspläne wurden anderswo gespeichert: Stand der Ziele neu holen.
  ansichtsAbfrage(()=>projektId,["fachobjekte"],()=>laden());
  const objekte = $derived([...new Map(aufgaben.map((a) => [`${a.verbraucher}:${a.objekt}`, a])).values()]);
  const sichtbar = $derived(aufgaben.filter((a) => (!filter || `${a.verbraucher}:${a.objekt}` === filter) && (!nurOffen || !a.gebunden)));
  const gruppen = $derived([...new Map(sichtbar.map((a) => [a.gruppe, sichtbar.filter((x) => x.gruppe === a.gruppe)])).entries()]);
  const offen = $derived(aufgaben.filter((a) => !a.gebunden).length);
  const name = (m: string | null) => plan.markierungen.find((x) => x.id === m)?.bezeichnung ?? "Markierung";
  // Flurstücke werden über die Projektgrenze gespeichert (wie unter Grundlagen → Flurstücke);
  // die gezeichnete Fläche setzt der Core ein, die Oberfläche trägt keine Werte selbst ein.
  const flurText = (f: { flaeche_m2: number; planflaeche_m2?: number; flaeche_aus_plan?: boolean } | undefined) =>
    !f ? "—" : f.flaeche_aus_plan ? `${werttext(f.flaeche_m2, "m²")} (aus Plan)` : f.planflaeche_m2 == null ? werttext(f.flaeche_m2, "m²") : `${werttext(f.flaeche_m2, "m²")} Kataster · gezeichnet ${werttext(f.planflaeche_m2, "m²")}`;
  async function flurstueckeSpeichern(basis: Project, infos: Grundstuecksinformationen) {
    const p = await api.projektBereichSpeichern(basis, "grundstuecksinformationen", {
      id: basis.id, expected_version: basis.version, name: basis.name, bundesland: basis.bundesland, grundstuecksflaeche_m2: basis.grundstuecksflaeche_m2,
      planungsrecht: { ...basis.planungsrecht, grundstuecksinformationen: infos },
    });
    onprojekt?.(p);
  }
  async function uebernehmen(a: Planaufgabe) {
    if (!a.markierung || busy) return;
    busy = a.ziel; fehler = ""; meldung = "";
    try {
      // Die Bindung verweist auf den gespeicherten Stand dieses Aufmaßes.
      const aufmass = (await plattformApi.liste<Planaufmass>(projektId, "planaufmass")).find((x) => x.objekt === aufmassId);
      if (!aufmass) throw Error("Aufmaß zuerst speichern.");
      const bindung = { aufmass_id: aufmassId, revision: aufmass.revision, markierung_id: a.markierung, groesse: a.groesse, ziel: a.ziel };
      if (a.verbraucher === "flurstueck") {
        const basis = JSON.parse(JSON.stringify(projekt)) as Project;
        const infos = grundstuecksinformationen(basis.planungsrecht);
        const f = infos.flurstuecke.find((x) => x.id === a.objekt);
        if (!f) throw Error("Flurstück ist nicht mehr gespeichert.");
        const vorher = { ...f };
        f.planbindung = bindung;
        // Ohne amtliche Fläche liefert die Zeichnung die Flurstücksfläche; sonst bleibt sie Kontrolle.
        if (!(f.flaeche_m2 > 0)) f.flaeche_aus_plan = true;
        const geprueft = await planApi.flurstuecke(projektId, infos);
        const neu = geprueft.flurstuecke.find((x) => x.id === a.objekt);
        if(!(await v.fragen([{ziel:`${a.objekt_name} · ${a.titel}`,vorher:flurText(vorher),nachher:flurText(neu),quelle:`${plan.bezeichnung} · ${name(a.markierung)} · R${aufmass.revision}`}],true)))return;
        await flurstueckeSpeichern(basis, geprueft);
      } else if (a.verbraucher === "gebaeude") {
        const stand = (await plattformApi.liste<Gebaeude>(projektId, "gebaeude")).find((g) => g.objekt === a.objekt);
        if (!stand) throw Error("Gebäude ist nicht mehr gespeichert.");
        const neu = structuredClone(stand.daten);
        neu.planbindungen = [...(neu.planbindungen ?? []).filter((b) => b.ziel !== a.ziel), bindung];
        const geprueft = await planApi.gebaeude(projektId, a.objekt, neu);
        if(!(await v.fragen([{ziel:`${a.objekt_name} · ${a.titel}`,vorher:werttext(zielfeld(stand.daten,a.ziel),einheit(a)),nachher:werttext(zielfeld(geprueft,a.ziel),einheit(a)),quelle:`${plan.bezeichnung} · ${name(a.markierung)} · R${aufmass.revision}`}],true)))return;
        await plattformApi.speichern(projektId, "gebaeude", a.objekt, stand.revision, geprueft);
      } else if (a.verbraucher === "berechnung") {
        // Fachberechnung: Bindung eintragen, Messwert im Entwurf einsetzen, über die Fachgrenze speichern.
        const art = artVon(a);
        const stand = await plattformApi.lesen<FachPlan>(projektId, art, a.objekt);
        if (!stand) throw Error("Berechnung ist nicht mehr gespeichert.");
        const neu = structuredClone(stand.daten);
        neu.planbindungen = [...(neu.planbindungen ?? []).filter((b) => b.ziel !== a.ziel), bindung];
        neu.uebernahmen = neu.uebernahmen.filter((u) => u.ziel !== a.ziel);
        const geprueft = await planApi.einsetzen(projektId, neu);
        if(!(await v.fragen([{ziel:`${a.objekt_name} · ${a.titel}`,vorher:werttext(zielfeld(stand.daten.eingaben,a.ziel),einheit(a)),nachher:werttext(zielfeld(geprueft.eingaben,a.ziel),einheit(a)),quelle:`${plan.bezeichnung} · ${name(a.markierung)} · R${aufmass.revision}`}],true)))return;
        await plattformApi.speichern(projektId, art, a.objekt, stand.revision, geprueft);
      } else if (a.verbraucher === "raumbuch") {
        const stand = await plattformApi.lesen<Raumbuch>(projektId, "raumbuch", a.objekt);
        if (!stand) throw Error("Raumbuch ist nicht mehr gespeichert.");
        let geprueft: Raumbuch, zeile;
        if (a.ziel === "raeume") {
          // Alle freien Raumkonturen des Gebäudes/Geschosses als Räume übernehmen (wie im Raumbuch).
          geprueft = await planApi.raumbuch(projektId, aufmassId, aufmass.revision, structuredClone(stand.daten));
          zeile = {ziel:`${a.objekt_name} · Räume`,vorher:`${stand.daten.raeume.length} Räume`,nachher:`${geprueft.raeume.length} Räume`,quelle:`${plan.bezeichnung} · R${aufmass.revision}`};
        } else {
          const neu = structuredClone(stand.daten);
          neu.planbindungen = [...(neu.planbindungen ?? []).filter((b) => b.ziel !== a.ziel), bindung];
          geprueft = await planApi.raumbuchEinsetzen(projektId, neu);
          zeile = {ziel:`${a.objekt_name} · ${a.titel}`,vorher:werttext(zielfeld(stand.daten,a.ziel),"m²"),nachher:werttext(zielfeld(geprueft,a.ziel),"m²"),quelle:`${plan.bezeichnung} · ${name(a.markierung)} · R${aufmass.revision}`};
        }
        if(!(await v.fragen([zeile],true)))return;
        await plattformApi.speichern(projektId, "raumbuch", a.objekt, stand.revision, geprueft);
      } else {
        const stand = (await plattformApi.liste<AbstandsPlan>(projektId, "abstandsflaechen")).find((g) => g.objekt === a.objekt);
        if (!stand) throw Error("Abstandsplan ist nicht mehr gespeichert.");
        const neu = structuredClone(stand.daten);
        neu.planbindungen = [...(neu.planbindungen ?? []).filter((b) => b.ziel !== a.ziel), bindung];
        const geprueft = await planApi.abstand(projektId, neu);
        if(!(await v.fragen([{ziel:`${a.objekt_name} · ${a.titel}`,vorher:werttext(zielfeld(stand.daten,a.ziel),einheit(a)),nachher:werttext(zielfeld(geprueft,a.ziel),einheit(a)),quelle:`${plan.bezeichnung} · ${name(a.markierung)} · R${aufmass.revision}`}],true)))return;
        await plattformApi.speichern(projektId, "abstandsflaechen", a.objekt, stand.revision, geprueft);
      }
      const n = name(a.markierung);
      meldung = a.ziel === "raeume" ? `Räume aus „${plan.bezeichnung}“ in ${a.objekt_name} übernommen und gespeichert.` : n === `${a.gruppe} · ${a.titel}` ? `„${n}“ übernommen und gespeichert.` : `„${n}“ als ${a.gruppe} · ${a.titel} übernommen und gespeichert.`;
      laden();
    } catch (e) { fehler = fehlertext(e); } finally { busy = ""; }
  }
  async function gesammeltPruefen(){
    if(busy||!darf||!gespeichert)return;busy="sammel";fehler="";meldung="";
    const jobs:{a:Planaufgabe;revision:number;daten:Gebaeude|AbstandsPlan|FachPlan|Raumbuch}[]=[],zeilen:Wertvergleich[]=[],probleme:string[]=[];
    let flurJob:{basis:Project;infos:Grundstuecksinformationen}|null=null;
    try{
      const aufmass=await plattformApi.lesen<Planaufmass>(projektId,"planaufmass",aufmassId);if(!aufmass)throw Error("Aufmaß nicht mehr vorhanden.");
      // Flurstücke: ein Speichervorgang für alle Bindungen dieses Aufmaßes.
      const flur=aufgaben.filter(a=>a.verbraucher==="flurstueck"&&a.gebunden);
      if(flur.length)try{
        const basis=JSON.parse(JSON.stringify(projekt)) as Project;
        const infos=grundstuecksinformationen(basis.planungsrecht);
        for(const f of infos.flurstuecke)if(f.planbindung?.aufmass_id===aufmassId)f.planbindung={...f.planbindung,revision:aufmass.revision};
        const geprueft=await planApi.flurstuecke(projektId,infos);
        for(const a of flur){const alt=infos.flurstuecke.find(x=>x.id===a.objekt),neu=geprueft.flurstuecke.find(x=>x.id===a.objekt);zeilen.push({ziel:`${a.objekt_name} · ${a.titel}`,vorher:flurText(alt),nachher:flurText(neu),quelle:`${plan.bezeichnung} · ${name(a.gebunden)} · R${aufmass.revision}`})}
        flurJob={basis,infos:geprueft};
      }catch(e){probleme.push(`Flurstücke: ${fehlertext(e)}`)}
      // Abzugsflächen und GRZ-Bauteile werden unter Berechnungen → Baugrundstück aktualisiert und gespeichert.
      const gruppen=[...new Map(aufgaben.filter(a=>a.gebunden&&!["flurstueck","abzug","bauteil"].includes(a.verbraucher)).map(a=>[`${a.verbraucher}:${a.objekt}`,a])).values()];
      for(const a of gruppen)try{
        const art=artVon(a);
        const stand=await plattformApi.lesen<Gebaeude|AbstandsPlan|FachPlan|Raumbuch>(projektId,art,a.objekt);if(!stand)throw Error("Zielobjekt fehlt.");
        const neu=structuredClone(stand.daten);neu.planbindungen=(neu.planbindungen??[]).map(b=>b.aufmass_id===aufmassId?{...b,revision:aufmass.revision}:b);
        const geprueft=a.verbraucher==="berechnung"?await planApi.einsetzen(projektId,neu as FachPlan):a.verbraucher==="raumbuch"?await planApi.raumbuchEinsetzen(projektId,neu as Raumbuch):"waende" in neu?await planApi.abstand(projektId,neu):await planApi.gebaeude(projektId,a.objekt,neu as Gebaeude);
        jobs.push({a,revision:stand.revision,daten:geprueft});
        for(const feld of aufgaben.filter(x=>x.verbraucher===a.verbraucher&&x.objekt===a.objekt&&x.gebunden))zeilen.push({ziel:`${a.objekt_name} · ${feld.titel}`,vorher:werttext(zielfeld(zielwerte(a,stand.daten),feld.ziel),einheit(feld)),nachher:werttext(zielfeld(zielwerte(a,geprueft),feld.ziel),einheit(feld)),quelle:`${plan.bezeichnung} · ${name(feld.gebunden)} · R${aufmass.revision}`});
      }catch(e){probleme.push(`${a.objekt_name}: ${fehlertext(e)}`)}
      if(!jobs.length&&!flurJob){fehler=probleme.join(" · ")||"Keine verknüpften Zielobjekte vorhanden.";return}
      for(const p of probleme)zeilen.push({ziel:"Nicht aktualisierbar",vorher:"Bleibt unverändert",nachher:"Quelle zuerst klären",quelle:p});
      if(!(await v.fragen(zeilen,true)))return;
      let anzahl=0;
      if(flurJob)try{await flurstueckeSpeichern(flurJob.basis,flurJob.infos);anzahl++}catch(e){probleme.push(`Flurstücke: ${fehlertext(e)}`)}
      for(const job of jobs)try{await plattformApi.speichern(projektId,artVon(job.a),job.a.objekt,job.revision,job.daten);anzahl++}catch(e){probleme.push(`${job.a.objekt_name}: ${fehlertext(e)}`)}
      meldung=`${anzahl} Zielobjekt(e) aktualisiert und gespeichert.`;await laden();fehler=probleme.join(" · ");
    }catch(e){fehler=fehlertext(e)}finally{busy=""}
  }
</script>
<UebernahmeVorschau {v}/>

<section class="aufgaben" aria-label="Aufmaß-Aufgaben">
  <h3>Aufgaben <span class="meta">({offen} offen)</span></h3>
  {#if aufgaben.some(a=>a.gebunden)}<button type="button" class="sekundaer" disabled={!darf||!gespeichert||!!busy} onclick={()=>void gesammeltPruefen()}>Verknüpfte Werte gesammelt prüfen …</button>{/if}
  {#if !aufgaben.length}<p class="meta">Keine Zielfelder: zuerst unter Grundlagen → Flurstücke ein Flurstück, unter Grundlagen → Gebäude / Geschosse ein Gebäude, ein Raumbuch oder eine Fachberechnung mit Positionen speichern oder im Abstandsflächenplan eine manuelle Wand anlegen.</p>
  {:else}
    <div class="filter">
      <select aria-label="Aufgaben für" bind:value={filter}><option value="">Alle Flurstücke, Abzüge, Bauteile, Gebäude, Wände, Berechnungen und Raumbücher</option>{#each objekte as o}<option value={`${o.verbraucher}:${o.objekt}`}>{o.objekt_name}</option>{/each}</select>
      <label><input type="checkbox" bind:checked={nurOffen}/>nur offene</label>
    </div>
    {#each gruppen as [gruppe, liste] (gruppe)}
      <h4>{gruppe}</h4>
      <ul>
        {#each liste as a (`${a.verbraucher}:${a.ziel}`)}
          <li class:erledigt={!!a.gebunden} class:bereit={!!a.markierung}>
            <span class="status" aria-hidden="true">{a.gebunden ? "✔" : a.markierung ? "●" : "○"}</span>
            <span class="titel">{a.titel}{a.wert_vorhanden && !a.gebunden ? " · manuell erfasst" : ""}</span>
            {#if a.gebunden}<span class="meta">verknüpft mit „{name(a.gebunden)}“</span>
            {:else if a.markierung}
              <button type="button" class="link" onclick={() => onwaehlen(a.markierung!)}>„{name(a.markierung)}“</button>
              {#if a.verbraucher === "abzug" || a.verbraucher === "bauteil"}<span class="meta hinweis">{a.hinweis}</span>
              {:else}<button type="button" disabled={!darf || !gespeichert || busy !== ""} title={gespeichert ? "Markierung verknüpfen und Verbraucher speichern" : "Aufmaß zuerst speichern"} onclick={() => void uebernehmen(a)}>{busy === a.ziel ? "…" : "Übernehmen"}</button>{/if}
            {:else}
              <button type="button" class="sekundaer" disabled={!darf} title={a.hinweis ?? ""} onclick={() => onzeichnen(a)}>Zeichnen</button>
              {#if a.hinweis}<span class="meta hinweis">{a.hinweis}</span>{/if}
            {/if}
          </li>
        {/each}
      </ul>
    {/each}
    {#if !gespeichert}<p class="meta">Übernehmen ist nach dem Speichern des Aufmaßes möglich.</p>{/if}
  {/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if meldung}<p class="meta" role="status">{meldung}</p>{/if}
</section>

<style>
  .aufgaben h4 { margin: .5rem 0 .2rem; font-size: .9rem; }
  .aufgaben ul { list-style: none; margin: 0; padding: 0; display: grid; gap: .2rem; }
  .aufgaben li { display: flex; flex-wrap: wrap; gap: .3rem .5rem; align-items: center; padding: .15rem .3rem; border-radius: 4px; }
  .aufgaben li.bereit { background: #168bb114; }
  .aufgaben li.erledigt { opacity: .75; }
  .status { width: 1.1em; text-align: center; color: #126d97; }
  .erledigt .status { color: #1a7f37; }
  .titel { flex: 1; min-width: 8rem; }
  .hinweis { flex-basis: 100%; padding-left: 1.6em; }
  .filter { display: flex; gap: .6rem; align-items: center; margin-bottom: .3rem; }
  .filter label { display: flex; gap: .3rem; align-items: center; white-space: nowrap; font-size: .85rem; }
</style>
