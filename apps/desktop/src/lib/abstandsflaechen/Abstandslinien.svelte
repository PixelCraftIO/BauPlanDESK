<script lang="ts">
  import { fehlertext } from "../api";
  import { plattformApi } from "../plattformApi";
  import { planApi } from "../planaufmass/api";
  import type { AbstandsPlan, Planaufmass } from "../generated/plattform";
  /**
   * Linien „Abstandsfläche“ eines gespeicherten Lageplans als Wände übernehmen.
   * Lage und Länge bleiben an die Linien gebunden, Grenzen an die Flurstücke.
   */
  let {
    plan,
    projektId,
    darf,
    onplan,
    onbusy,
  }: {
    plan: AbstandsPlan;
    projektId: string;
    darf: boolean;
    onplan: (p: AbstandsPlan) => void;
    onbusy: (b: boolean) => void;
  } = $props();
  type Stand = { objekt: string; revision: number; daten: Planaufmass };
  let aufmasse = $state<Stand[]>([]),
    wahl = $state(""),
    fehler = $state(""),
    hinweise = $state<string[]>([]),
    meldung = $state(""),
    busy = $state(false);
  const linien = (a: Planaufmass) =>
    a.markierungen.filter((m) => m.bedeutung === "abstandsflaeche");
  // Ein Plan nutzt genau einen Lageplan; mit Bindungen ist er gesetzt.
  const gebunden = $derived(
    plan.planbindungen?.find((b) => /\.(lage|grenze)$/.test(b.ziel))
      ?.aufmass_id ?? "",
  );
  $effect(() => {
    void projektId;
    let weg = false;
    plattformApi
      .liste<Planaufmass>(projektId, "planaufmass")
      .then((l) => {
        if (weg) return;
        aufmasse = l
          .filter((a) => linien(a.daten).length)
          .map((a) => ({ objekt: a.objekt, revision: a.revision, daten: a.daten }));
        if (!aufmasse.some((a) => a.objekt === wahl))
          wahl =
            aufmasse.find((a) => a.objekt === gebunden)?.objekt ??
            aufmasse[0]?.objekt ??
            "";
      })
      .catch((e) => {
        if (!weg) fehler = fehlertext(e);
      });
    return () => {
      weg = true;
    };
  });
  const gewaehlt = $derived(aufmasse.find((a) => a.objekt === wahl));
  const bindungen = $derived(
    (plan.planbindungen ?? []).filter((b) => b.ziel.endsWith(".lage")),
  );
  async function uebernehmen() {
    if (!gewaehlt) return;
    busy = true;
    onbusy(true);
    fehler = "";
    meldung = "";
    hinweise = [];
    try {
      const vorher = JSON.stringify(plan);
      const r = await planApi.abstandslinien(
        projektId,
        JSON.parse(vorher) as AbstandsPlan,
        { aufmass_id: gewaehlt.objekt, revision: gewaehlt.revision },
      );
      if (JSON.stringify(plan) !== vorher)
        throw Error(
          "Eingaben wurden zwischenzeitlich geändert. Übernahme erneut ausführen.",
        );
      const neu = r.plan.waende.length - plan.waende.length;
      onplan(r.plan);
      hinweise = r.hinweise;
      meldung = `${linien(gewaehlt.daten).length} Linien übernommen${neu > 0 ? `, ${neu} neue Wände` : ""}. Angaben prüfen und speichern.`;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
      onbusy(false);
    }
  }
</script>

<section class="linien" aria-labelledby="linien-titel">
  <h4 id="linien-titel">Abstandsflächen aus dem Lageplan</h4>
  <p class="meta">
    Im Aufmaß des Lageplans je Außenwand eine Linie mit dem Werkzeug
    „Abstandsfläche“ ziehen (Vorlage am Werkzeug „Strecke / Dach“). Jede Linie wird hier
    eine Wand mit ihrem Namen (A1, A2 …). Lage und Länge bleiben an die Linie
    gebunden; zugeordnete Flurstücke im selben Plan liefern die
    Grundstücksgrenzen, ein Vorhabenumriss die Außenseite.
  </p>
  {#if !aufmasse.length}
    <p class="hinweisbox">
      Noch kein gespeichertes Aufmaß mit Linien „Abstandsfläche“.
    </p>
  {:else}
    <div class="raster">
      <label
        >Lageplan<select bind:value={wahl} disabled={!darf || busy}
          >{#each aufmasse as a (a.objekt)}<option
              value={a.objekt}
              disabled={!!gebunden && gebunden !== a.objekt}
              >{a.daten.bezeichnung} · {linien(a.daten).length} Linien · R{a.revision}</option
            >{/each}</select
        ></label
      >
    </div>
    {#if gewaehlt}<p class="meta">
        Linien: {linien(gewaehlt.daten)
          .map((m) => m.bezeichnung)
          .join(", ")}
      </p>{/if}
    <button
      type="button"
      disabled={!darf || busy || !gewaehlt}
      onclick={uebernehmen}
      >{bindungen.length
        ? "Linien aus dem Lageplan aktualisieren"
        : "Linien aus dem Lageplan übernehmen"}</button
    >
  {/if}
  {#if fehler}<p role="alert">{fehler}</p>{/if}
  {#if meldung}<p role="status">{meldung}</p>{/if}
  {#if hinweise.length}<ul class="hinweisbox">
      {#each hinweise as h}<li>{h}</li>{/each}
    </ul>{/if}
</section>

<style>
  .linien {
    margin: 0.75rem 0 1rem;
  }
</style>
