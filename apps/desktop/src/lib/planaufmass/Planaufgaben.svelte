<script lang="ts">
  import { fehlertext } from "../api";
  import type { Planaufgabe, Planaufmass, Gebaeude, AbstandsPlan } from "../generated/plattform";
  import { plattformApi } from "../plattformApi";
  import { planApi } from "./api";
  import { bereichAktiv, ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  // Was das Projekt aus diesem Aufmaß noch braucht. Die Liste kommt vom Core;
  // „Zeichnen“ belegt Werkzeug und Zuordnung vor, „Übernehmen“ verknüpft die
  // Markierung über die bestehenden Übernahmedienste und speichert den Verbraucher.
  let { projektId, plan, aufmassId, gespeichert, darf, onzeichnen, onwaehlen }: {
    projektId: string; plan: Planaufmass; aufmassId: string;
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
  async function uebernehmen(a: Planaufgabe) {
    if (!a.markierung || busy) return;
    busy = a.ziel; fehler = ""; meldung = "";
    try {
      // Die Bindung verweist auf den gespeicherten Stand dieses Aufmaßes.
      const aufmass = (await plattformApi.liste<Planaufmass>(projektId, "planaufmass")).find((x) => x.objekt === aufmassId);
      if (!aufmass) throw Error("Aufmaß zuerst speichern.");
      const bindung = { aufmass_id: aufmassId, revision: aufmass.revision, markierung_id: a.markierung, groesse: a.groesse, ziel: a.ziel };
      if (a.verbraucher === "gebaeude") {
        const stand = (await plattformApi.liste<Gebaeude>(projektId, "gebaeude")).find((g) => g.objekt === a.objekt);
        if (!stand) throw Error("Gebäude ist nicht mehr gespeichert.");
        const neu = structuredClone(stand.daten);
        neu.planbindungen = [...(neu.planbindungen ?? []).filter((b) => b.ziel !== a.ziel), bindung];
        const geprueft = await planApi.gebaeude(projektId, a.objekt, neu);
        await plattformApi.speichern(projektId, "gebaeude", a.objekt, stand.revision, geprueft);
      } else {
        const stand = (await plattformApi.liste<AbstandsPlan>(projektId, "abstandsflaechen")).find((g) => g.objekt === a.objekt);
        if (!stand) throw Error("Abstandsplan ist nicht mehr gespeichert.");
        const neu = structuredClone(stand.daten);
        neu.planbindungen = [...(neu.planbindungen ?? []).filter((b) => b.ziel !== a.ziel), bindung];
        const geprueft = await planApi.abstand(projektId, neu);
        await plattformApi.speichern(projektId, "abstandsflaechen", a.objekt, stand.revision, geprueft);
      }
      const n = name(a.markierung);
      meldung = n === `${a.gruppe} · ${a.titel}` ? `„${n}“ übernommen und gespeichert.` : `„${n}“ als ${a.gruppe} · ${a.titel} übernommen und gespeichert.`;
      laden();
    } catch (e) { fehler = fehlertext(e); } finally { busy = ""; }
  }
</script>

<section class="aufgaben" aria-label="Aufmaß-Aufgaben">
  <h3>Aufgaben <span class="meta">({offen} offen)</span></h3>
  {#if !aufgaben.length}<p class="meta">Keine Zielfelder: zuerst unter Grundlagen → Gebäude / Geschosse ein Gebäude speichern oder im Abstandsflächenplan eine manuelle Wand anlegen.</p>
  {:else}
    <div class="filter">
      <select aria-label="Aufgaben für" bind:value={filter}><option value="">Alle Gebäude und Wände</option>{#each objekte as o}<option value={`${o.verbraucher}:${o.objekt}`}>{o.objekt_name}</option>{/each}</select>
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
              <button type="button" disabled={!darf || !gespeichert || busy !== ""} title={gespeichert ? "Markierung verknüpfen und Verbraucher speichern" : "Aufmaß zuerst speichern"} onclick={() => void uebernehmen(a)}>{busy === a.ziel ? "…" : "Übernehmen"}</button>
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
