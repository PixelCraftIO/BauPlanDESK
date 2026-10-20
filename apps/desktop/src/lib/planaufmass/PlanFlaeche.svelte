<script lang="ts">
  import { fehlertext } from "../api";
  import type { Gebaeude } from "../generated/plattform";
  import { gebaeudeflaechen, gebaeudefeldVerknuepfen, type Aufmassquelle } from "./quellen";
  // Direkte Flächenübernahme an einem Gebäude- oder Geschossfeld. Die Vorauswahl
  // filtert nur; Prüfung und Messwert stammen aus dem Core (planaufmass_gebaeude).
  let { projektId, gebaeudeId, g, aufmasse, geschossId = null, geschossName = "", gespeichert, onplan, onbusy }: {
    projektId: string; gebaeudeId: string; g: Gebaeude; aufmasse: Aufmassquelle[]; geschossId?: string | null; geschossName?: string;
    gespeichert: boolean; onplan: (g: Gebaeude) => void; onbusy: (b: boolean) => void;
  } = $props();
  const ziel = $derived(geschossId ? `geschosse.${geschossId}.flaeche_m2` : "geometrie.grundflaeche_m2");
  const bindung = $derived(g.planbindungen?.find((b) => b.ziel === ziel));
  const quelle = $derived(bindung ? aufmasse.find((a) => a.stand.objekt === bindung.aufmass_id) : undefined);
  const auswahl = $derived(gebaeudeflaechen(aufmasse, gebaeudeId, geschossId));
  const z = (v: number) => v.toLocaleString("de-DE", { maximumFractionDigits: 3 });
  let gewaehlt = $state(""), busy = $state(false), fehler = $state(""), meldung = $state("");
  const kandidat = $derived(auswahl.passend.find((q) => `${q.a.stand.objekt}|${q.m.id}` === gewaehlt) ?? (auswahl.passend.length === 1 ? auswahl.passend[0] : undefined));
  async function setzen(q: { a: Aufmassquelle; m: { id: string } } | null) {
    const vorher = JSON.stringify(g);
    const m = q ? q.a.stand.daten.markierungen.find((x) => x.id === q.m.id) ?? null : null;
    busy = true; onbusy(true); fehler = ""; meldung = "";
    try {
      const neu = await gebaeudefeldVerknuepfen(projektId, gebaeudeId, JSON.parse(vorher), ziel, q && m ? { a: q.a, m } : null);
      if (JSON.stringify(g) !== vorher) throw Error("Eingaben wurden zwischenzeitlich geändert. Übernahme erneut ausführen.");
      onplan(neu);
      meldung = "Fläche im Entwurf eingesetzt. Bitte prüfen und speichern.";
    } catch (e) { fehler = fehlertext(e); } finally { busy = false; onbusy(false); }
  }
  function loesen() {
    // Der eingesetzte Wert bleibt als manuelle Angabe erhalten.
    g.planbindungen = g.planbindungen?.filter((b) => b.ziel !== ziel);
  }
</script>

<div class="planflaeche" aria-label={geschossId ? "Geschossfläche aus PDF-Aufmaß" : "Gebäudegrundfläche aus PDF-Aufmaß"}>
  {#if bindung}
    {@const m = quelle?.stand.daten.markierungen.find((x) => x.id === bindung.markierung_id)}
    <span>Aus PDF-Aufmaß: <strong>{quelle?.stand.daten.bezeichnung ?? "Aufmaß fehlt"} · {m?.bezeichnung ?? "Markierung fehlt"}</strong> (Stand R{bindung.revision})</span>
    {#if quelle && quelle.stand.revision !== bindung.revision}<span class="meta">Aufmaß inzwischen gespeichert (R{quelle.stand.revision}); bei geänderter Kontur aktualisieren.</span>
      <button type="button" class="sekundaer" disabled={busy || !m} onclick={() => quelle && m && setzen({ a: quelle, m })}>Aktuellen Aufmaßstand übernehmen</button>{/if}
    <button type="button" class="link" disabled={busy} onclick={loesen}>Verknüpfung lösen</button>
  {:else if !gespeichert}
    <span class="meta">Aus PDF-Aufmaß: {geschossId ? "Geschoss" : "Gebäude"} zuerst speichern. Danach im PDF-Aufmaß eine Fläche zeichnen und {geschossId ? "diesem Geschoss" : "diesem Gebäude"} zuordnen.</span>
  {:else if auswahl.passend.length}
    <label>Aus PDF-Aufmaß<select bind:value={gewaehlt} disabled={busy}>
      {#if auswahl.passend.length > 1}<option value="">Markierung auswählen</option>{/if}
      {#each auswahl.passend as q}<option value={`${q.a.stand.objekt}|${q.m.id}`}>{q.a.stand.daten.bezeichnung} · {q.m.bezeichnung} · {z(q.wert)} m²</option>{/each}
    </select></label>
    <button type="button" disabled={busy || !kandidat} onclick={() => kandidat && setzen(kandidat)}>Fläche übernehmen</button>
  {:else}
    <details class="meta"><summary>Fläche aus PDF-Aufmaß übernehmen – so geht's</summary>
      <ol>
        <li>Unter Unterlagen → PDF-Aufmaß die Außenkontur als Polygon oder Rechteck zeichnen.</li>
        <li>Bedeutung „{geschossId ? "Geschoss – Außenkante" : "Gebäudegrundfläche – horizontale Projektion"}“, Gebäude „{g.bezeichnung || "dieses Gebäude"}“{geschossId ? ` und Geschoss „${geschossName || "dieses Geschoss"}“` : ""} wählen und das Aufmaß speichern.</li>
        <li>Hier erscheint die Markierung anschließend zur Übernahme.</li>
      </ol>
      {#if geschossId}<p>Dieselbe Kontur kann zugleich die Gebäudegrundfläche liefern: Markierung im PDF-Aufmaß mit „Duplizieren“ kopieren und die Kopie anders kennzeichnen.</p>{/if}
    </details>
    {#each auswahl.unpassend as u}<p class="hinweisbox">„{u.a.stand.daten.bezeichnung} · {u.m.bezeichnung}“ {u.grund}.</p>{/each}
  {/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if meldung}<p class="meta" role="status">{meldung}</p>{/if}
</div>

<style>
  .planflaeche { display: flex; flex-wrap: wrap; gap: .4rem .8rem; align-items: end; margin-block: .4rem .8rem; }
  .planflaeche > p, .planflaeche > details { flex-basis: 100%; margin: 0; }
  .planflaeche ol { margin-block: .3rem; }
</style>
