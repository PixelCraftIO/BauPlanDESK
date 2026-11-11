<script lang="ts">
  /**
   * Messwerte und Verwendung: alle Markierungen aller Aufmaße des Projekts mit
   * Wert, Bedeutung und den Berechnungen, in die sie eingehen; dazu die offenen
   * Aufmaß-Aufgaben. „Im Editor zeigen“ öffnet das Aufmaß als Tab und wählt die
   * Markierung; der Verbraucher ist direkt anspringbar.
   */
  import { untrack } from "svelte";
  import type { FachProps } from "../fachmodule/register";
  import type { Planaufgabe, Planmessgroesse } from "../generated/plattform";
  import { fehlertext } from "../api";
  import { ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  import { projektNavigation } from "../projektNavigation.svelte";
  import { planApi } from "./api";
  import { aufmasseLaden, messwert, type Aufmassquelle } from "./quellen";
  import { bedeutungText } from "./bedeutungen";
  import { verwendungenLaden, editorZiel, type Verwendung } from "./verwendung";
  let { projekt }: FachProps = $props();
  const navigation = projektNavigation();
  let aufmasse = $state<Aufmassquelle[]>([]), verwendungen = $state<Map<string, Verwendung[]>>(new Map()), aufgaben = $state<Record<string, Planaufgabe[]>>({});
  let fehler = $state(""), laedt = $state(false);
  let filterAufmass = $state(""), filterStatus = $state<"" | "verwendet" | "unverwendet" | "veraltet">(""), suche = $state("");
  let generation = 0;
  async function laden() {
    const n = ++generation; laedt = true; fehler = "";
    try {
      const [a, v] = await Promise.all([aufmasseLaden(projekt.id), verwendungenLaden(projekt)]);
      if (n !== generation) return;
      aufmasse = a; verwendungen = v;
      const je: Record<string, Planaufgabe[]> = {};
      await Promise.all(a.map(async q => { try { je[q.stand.objekt] = await planApi.aufgaben(projekt.id, q.stand.daten); } catch { je[q.stand.objekt] = []; } }));
      if (n !== generation) return;
      aufgaben = je;
    } catch (e) { if (n === generation) fehler = fehlertext(e); }
    finally { if (n === generation) laedt = false; }
  }
  const vormerken = ansichtsAbfrage(() => projekt.id, ["dokumente", "fachobjekte"], laden);
  $effect(() => { void projekt.version; untrack(vormerken); });
  const z = (n: number | null | undefined, stellen = 2) => n == null ? "—" : n.toLocaleString("de-DE", { minimumFractionDigits: stellen, maximumFractionDigits: stellen });
  const einheit = (g: Planmessgroesse) => g === "flaeche" ? "m²" : g === "neigung" ? "°" : "m";
  type Zeile = { a: Aufmassquelle; id: string; name: string; bedeutung: string; flaeche: number | null; laenge: number | null; hoehe: number | null; verwendungen: Verwendung[]; veraltet: boolean; fehler: boolean };
  const zeilen = $derived.by((): Zeile[] => aufmasse.flatMap(a => a.stand.daten.markierungen.map(m => {
    const w = a.werte.find(w => w.id === m.id), v = verwendungen.get(m.id) ?? [];
    return { a, id: m.id, name: m.bezeichnung, bedeutung: bedeutungText(m.bedeutung), flaeche: w?.flaeche_m2 ?? null, laenge: w?.laenge_m ?? null, hoehe: w?.hoehe_m ?? null, verwendungen: v, veraltet: v.some(x => x.bindung.revision !== a.stand.revision), fehler: !w || (w.offen?.length ?? 0) > 0 };
  })));
  const gefiltert = $derived(zeilen.filter(r => (!filterAufmass || r.a.stand.objekt === filterAufmass)
    && (filterStatus === "" || (filterStatus === "verwendet" ? r.verwendungen.length > 0 : filterStatus === "unverwendet" ? r.verwendungen.length === 0 : r.veraltet))
    && (!suche || `${r.name} ${r.bedeutung} ${r.verwendungen.map(v => v.verbraucher).join(" ")}`.toLocaleLowerCase("de").includes(suche.toLocaleLowerCase("de")))));
  const offeneAufgaben = $derived(Object.values(aufgaben).flat().filter(t => !t.gebunden));
  // Wie im Projektbaum gezählt: Eine Aufgabe ist offen, solange in keinem Aufmaß
  // eine Markierung verknüpft und auch kein Wert von Hand erfasst ist.
  const aufgabenStand = $derived.by(() => {
    const alle = new Map<string, { gebunden: boolean; wert: boolean }>();
    for (const t of Object.values(aufgaben).flat()) {
      const k = `${t.verbraucher}|${t.objekt}|${t.ziel}`, e = alle.get(k) ?? { gebunden: false, wert: false };
      alle.set(k, { gebunden: e.gebunden || !!t.gebunden, wert: e.wert || t.wert_vorhanden });
    }
    const werte = [...alle.values()];
    return { ohneWert: werte.filter(e => !e.gebunden && !e.wert).length, vonHand: werte.filter(e => !e.gebunden && e.wert).length };
  });
  const summe = $derived({ markierungen: zeilen.length, verwendet: zeilen.filter(r => r.verwendungen.length).length, veraltet: zeilen.filter(r => r.veraltet).length });
  function zeigen(r: Zeile) { navigation?.springen(editorZiel(r.a.stand.objekt, r.id)); }
</script>

<h2>Messwerte und Verwendung</h2>
<p class="meta">Welche Fläche, Länge oder Höhe aus welchem Aufmaß in welche Berechnung eingeht und was noch offen ist. Werte stammen aus dem Core; verbindlich bleibt die Übernahme im Zielbereich.</p>
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
<div class="kennzahlen" aria-label="Stand der Messwerte">
  <div><strong>{summe.markierungen}</strong><span>Markierungen in {aufmasse.length} Aufmaßen</span></div>
  <div><strong>{summe.verwendet}</strong><span>in Berechnungen verwendet</span></div>
  <div class:warn={summe.veraltet}><strong>{summe.veraltet}</strong><span>mit älterem Aufmaßstand</span></div>
  <div class:warn={aufgabenStand.ohneWert}><strong>{aufgabenStand.ohneWert}</strong><span>Aufgaben ohne Wert</span></div>
  <div><strong>{aufgabenStand.vonHand}</strong><span>Werte von Hand, nicht gezeichnet</span></div>
</div>
<div class="filter">
  <label>Aufmaß<select aria-label="Aufmaß" bind:value={filterAufmass}><option value="">Alle Aufmaße</option>{#each aufmasse as a}<option value={a.stand.objekt}>{a.stand.daten.bezeichnung} · R{a.stand.revision}</option>{/each}</select></label>
  <label>Status<select aria-label="Status" bind:value={filterStatus}><option value="">Alle</option><option value="verwendet">Verwendet</option><option value="unverwendet">Noch nicht verwendet</option><option value="veraltet">Älterer Aufmaßstand</option></select></label>
  <label>Suche<input type="search" aria-label="Suche" bind:value={suche} placeholder="Markierung, Bedeutung, Berechnung"/></label>
  <button type="button" class="sekundaer" disabled={laedt} onclick={laden}>Aktualisieren</button>
</div>
<table class="ergebnistabelle messwerte" aria-label="Messwerte">
  <thead><tr><th>Aufmaß</th><th>Markierung</th><th>Bedeutung</th><th class="zahl">Fläche</th><th class="zahl">Länge</th><th class="zahl">Höhe</th><th>Verwendung</th><th></th></tr></thead>
  <tbody>
    {#each gefiltert as r (r.a.stand.objekt + r.id)}
      <tr class:veraltet={r.veraltet}>
        <td>{r.a.stand.daten.bezeichnung}<span class="meta"> · R{r.a.stand.revision}</span></td>
        <td>{r.name}{#if r.fehler}<span class="meta"> · Wert offen</span>{/if}</td>
        <td>{r.bedeutung}</td>
        <td class="zahl">{r.flaeche == null ? "—" : `${z(r.flaeche)} m²`}</td>
        <td class="zahl">{r.laenge == null ? "—" : `${z(r.laenge)} m`}</td>
        <td class="zahl">{r.hoehe == null ? "—" : `${z(r.hoehe)} m`}</td>
        <td>{#if r.verwendungen.length}<ul>{#each r.verwendungen as v}<li><button type="button" class="link" onclick={() => navigation?.springen(v.ziel)}>{v.verbraucher}</button><span class="meta"> → {v.bindung.ziel}{v.bindung.revision !== r.a.stand.revision ? ` · R${v.bindung.revision}, Aufmaß seitdem gespeichert` : ""}</span></li>{/each}</ul>{:else}<span class="meta">noch nicht verwendet</span>{/if}</td>
        <td><button type="button" class="link" onclick={() => zeigen(r)}>Im Editor zeigen</button></td>
      </tr>
    {:else}<tr><td colspan="8">{laedt ? "Wird geladen …" : aufmasse.length ? "Keine Markierung passt zu den Filtern." : "Noch kein gespeichertes Aufmaß vorhanden."}</td></tr>{/each}
  </tbody>
</table>
{#if offeneAufgaben.length}
  <details class="offene" open>
    <summary>Noch nicht aus einer Zeichnung übernommen ({offeneAufgaben.length})</summary>
    <table class="ergebnistabelle" aria-label="Offene Aufgaben">
      <thead><tr><th>Aufmaß</th><th>Verbraucher</th><th>Zielwert</th><th>Benötigt</th><th></th></tr></thead>
      <tbody>{#each Object.entries(aufgaben) as [id, liste]}{#each liste.filter(t => !t.gebunden) as t}
        {@const a = aufmasse.find(a => a.stand.objekt === id)}
        <tr><td>{a?.stand.daten.bezeichnung ?? id}</td><td>{t.gruppe} · {t.objekt_name}</td><td>{t.titel} ({einheit(t.groesse)})</td><td>{bedeutungText(t.bedeutung)}{t.markierung ? " · passende Markierung vorhanden" : ""}</td>
          <td><button type="button" class="link" onclick={() => navigation?.springen({ bereich: "planaufmass", objekt: id, register: t.markierung ? "markierung" : null, eintrag: t.markierung })}>Im Editor zeigen</button></td></tr>
      {/each}{/each}</tbody>
    </table>
  </details>
{/if}

<style>
  .kennzahlen{display:flex;gap:8px;flex-wrap:wrap;margin:.6rem 0}
  .kennzahlen > div{display:flex;flex-direction:column;min-width:150px;padding:6px 10px;border:1px solid var(--border);background:var(--panel)}
  .kennzahlen strong{font-size:1.3rem;font-variant-numeric:tabular-nums}
  .kennzahlen span{font-size:.85rem;color:var(--muted)}
  .kennzahlen .warn strong{color:var(--warning-text,#6b4a00)}
  .filter{display:flex;gap:10px;flex-wrap:wrap;align-items:end;margin:.4rem 0 .6rem}
  .filter label{display:flex;flex-direction:column;gap:2px;font-size:.85rem;color:var(--muted)}
  .messwerte td.zahl,.messwerte th.zahl{text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap}
  .messwerte ul{margin:0;padding:0;list-style:none}
  .messwerte td > .link{min-height:0;padding:0;vertical-align:baseline}
  .messwerte tr.veraltet td{background:var(--warn-bg,#fff4d6)}
  .offene{margin-top:1rem}
</style>
