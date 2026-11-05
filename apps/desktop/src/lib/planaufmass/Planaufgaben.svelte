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
  import { Abgleich } from "./Abgleich.svelte";
  import AbgleichDialog from "./AbgleichDialog.svelte";
  import { flurText, flurstueckVorbereiten, flurstueckeSpeichern as flurstueckeAblegen } from "./flurstueck";
  const v=new Uebernahme();
  const abgleich=new Abgleich();
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
  let filter = $state(""), nurOffen = $state(true), zu = $state<Record<string, boolean>>({});
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
  async function flurstueckeSpeichern(basis: Project, infos: Grundstuecksinformationen) {
    onprojekt?.(await flurstueckeAblegen(basis, infos));
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
        const { basis, geprueft, vorher, nachher } = await flurstueckVorbereiten(projekt, a.objekt, bindung);
        if(!(await v.fragen([{ziel:`${a.objekt_name} · ${a.titel}`,vorher,nachher,quelle:`${plan.bezeichnung} · ${name(a.markierung)} · R${aufmass.revision}`}],true)))return;
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
          const zuordnung = await abgleich.fragen(await planApi.raumbuchAbgleich(projektId, aufmassId, aufmass.revision, structuredClone(stand.daten)));
          if (!zuordnung) return;
          geprueft = await planApi.raumbuch(projektId, aufmassId, aufmass.revision, structuredClone(stand.daten), zuordnung);
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
<AbgleichDialog a={abgleich}/>

<section class="aufgaben" aria-label="Aufmaß-Aufgaben">
  {#if !aufgaben.length}<p class="meta">Keine Zielfelder: zuerst unter Grundlagen → Flurstücke ein Flurstück, unter Grundlagen → Gebäude / Geschosse ein Gebäude, ein Raumbuch oder eine Fachberechnung mit Positionen speichern oder im Abstandsflächenplan eine manuelle Wand anlegen.</p>
  {:else}
    <div class="stand" role="status"><span><strong>{offen}</strong> von {aufgaben.length} offen</span><span class="balken" aria-hidden="true"><span style={`width:${Math.round(100*(aufgaben.length-offen)/aufgaben.length)}%`}></span></span></div>
    <div class="filter">
      <select aria-label="Aufgaben für" bind:value={filter}><option value="">Alle Ziele</option>{#each objekte as o}<option value={`${o.verbraucher}:${o.objekt}`}>{o.objekt_name}</option>{/each}</select>
      <label><input type="checkbox" bind:checked={nurOffen}/>Nur offene</label>
    </div>
    {#each gruppen as [gruppe, liste] (gruppe)}
      {@const alle = aufgaben.filter((a) => a.gruppe === gruppe && (!filter || `${a.verbraucher}:${a.objekt}` === filter))}
      {@const fertig = alle.filter((a) => a.gebunden).length}
      {@const auf = !zu[gruppe]}
      <h4><button type="button" class="gruppe" aria-expanded={auf} onclick={() => zu = { ...zu, [gruppe]: auf }}><span class="pfeil" class:auf aria-hidden="true"></span><span class="name">{gruppe}</span><span class="zahl" title={`${fertig} von ${alle.length} verknüpft`}>{fertig}/{alle.length}</span><span class="balken" aria-hidden="true"><span style={`width:${alle.length ? Math.round(100 * fertig / alle.length) : 0}%`}></span></span></button></h4>
      {#if auf}<ul>
        {#each liste as a (`${a.verbraucher}:${a.ziel}`)}
          {@const ziel = a.gebunden ?? a.markierung}
          {@const tipp = [a.titel, a.gebunden ? `verknüpft mit „${name(a.gebunden)}“` : a.markierung ? `Markierung „${name(a.markierung)}“ passt` : "noch nicht gezeichnet", a.wert_vorhanden && !a.gebunden ? "Wert bisher von Hand erfasst" : "", a.hinweis ?? ""].filter(Boolean).join(" · ")}
          <li class:erledigt={!!a.gebunden} class:bereit={!!a.markierung && !a.gebunden} title={tipp}>
            <span class="status" aria-hidden="true">{a.gebunden ? "✔" : a.markierung ? "●" : "○"}</span>
            {#if ziel}<button type="button" class="titel" aria-label={`${a.titel}: „${name(ziel)}“ zeigen`} onclick={() => onwaehlen(ziel)}>{a.titel}</button>
            {:else}<span class="titel">{a.titel}</span>{/if}
            {#if a.wert_vorhanden && !a.gebunden}<span class="marke">Handwert</span>{/if}
            {#if a.gebunden}<span class="meta">verknüpft</span>
            {:else if a.markierung}
              {#if a.verbraucher === "abzug" || a.verbraucher === "bauteil"}<span class="meta">erkannt</span>
              {:else}<button type="button" class="klein" disabled={!darf || !gespeichert || busy !== ""} title={gespeichert ? "Markierung verknüpfen und Verbraucher speichern" : "Aufmaß zuerst speichern"} onclick={() => void uebernehmen(a)}>{busy === a.ziel ? "…" : "Übernehmen"}</button>{/if}
            {:else}
              <button type="button" class="sekundaer klein" disabled={!darf} title={a.hinweis ?? "Werkzeug, Bedeutung und Zuordnung vorbelegen"} onclick={() => onzeichnen(a)}>Zeichnen</button>
            {/if}
          </li>
        {/each}
      </ul>{/if}
    {/each}
    {#if !gruppen.length}<p class="meta">Alle Aufgaben erledigt.{nurOffen ? " „Nur offene“ abwählen, um die verknüpften zu sehen." : ""}</p>{/if}
    {#if aufgaben.some(a=>a.gebunden)}<button type="button" class="sekundaer pruefen" disabled={!darf||!gespeichert||!!busy} onclick={()=>void gesammeltPruefen()}>Verknüpfte Werte gesammelt prüfen …</button>{/if}
    {#if !gespeichert}<p class="meta">Übernehmen ist nach dem Speichern des Aufmaßes möglich.</p>{/if}
  {/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if meldung}<p class="meta" role="status">{meldung}</p>{/if}
</section>

<style>
  .stand { display: flex; align-items: center; gap: .6rem; font-size: .85rem; margin: .2rem 0 .4rem; }
  .stand .balken { flex: 1; }
  .balken { display: block; height: 4px; background: var(--border-innen); border-radius: 2px; overflow: hidden; }
  .balken > span { display: block; height: 100%; background: #1a7f37; }
  .filter { display: flex; gap: .6rem; align-items: center; margin-bottom: .3rem; }
  .filter select { flex: 1; min-width: 0; }
  .filter label { display: flex; gap: .3rem; align-items: center; white-space: nowrap; font-size: .85rem; }
  .aufgaben h4 { margin: .4rem 0 0; }
  .gruppe { display: flex; align-items: center; gap: .4rem; width: 100%; min-height: 26px; padding: 2px 4px; border: 0; border-bottom: 1px solid var(--border-innen); background: transparent; color: var(--fg); font-size: .85rem; font-weight: 600; text-align: left; border-radius: 0; }
  .gruppe .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .gruppe .zahl { font-weight: 400; font-size: .75rem; color: var(--muted); font-variant-numeric: tabular-nums; }
  .gruppe .balken { width: 3rem; flex: none; }
  .pfeil { width: 0; height: 0; border-style: solid; border-width: 4px 0 4px 6px; border-color: transparent transparent transparent var(--muted); transition: transform .1s; }
  .pfeil.auf { transform: rotate(90deg); }
  .aufgaben ul { list-style: none; margin: 0; padding: 0; }
  .aufgaben li { display: flex; flex-wrap: nowrap; gap: .4rem; align-items: center; min-height: 28px; padding: 1px 4px 1px 14px; border-top: 0; }
  .aufgaben li:hover { background: var(--leiste); }
  .aufgaben li.erledigt { opacity: .75; }
  .status { width: 1.1em; flex: none; text-align: center; color: #126d97; }
  .erledigt .status { color: #1a7f37; }
  /* Höchstens zwei Zeilen: lange Titel bleiben unterscheidbar ("… Direkte Höhe" / "… Lichte Höhe"). */
  .titel { flex: 1; min-width: 0; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; font-size: .85rem; line-height: 1.25; }
  button.titel { min-height: 0; padding: 0; border: 0; background: transparent; color: var(--accent); font-weight: 400; text-align: left; text-decoration: underline; text-decoration-color: transparent; }
  button.titel:hover { text-decoration-color: currentColor; }
  .marke { flex: none; font-size: .68rem; padding: 0 3px; border: 1px solid var(--border); color: var(--muted); }
  li .meta { flex: none; font-size: .78rem; }
  .klein { flex: none; min-height: 24px; padding: 1px 8px; font-size: .8rem; }
  .pruefen { margin-top: .6rem; }
</style>
