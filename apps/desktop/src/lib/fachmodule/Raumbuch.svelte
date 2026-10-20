<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Project } from "../api";
  import { fehlertext } from "../api";
  import type { Raumbuch, Gebaeude, RaumSummen } from "../generated/plattform";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  import { planApi } from "../planaufmass/api";
  import { abonnieren } from "../projektEreignisse";
  import { projektNavigation } from "../projektNavigation.svelte";
  import type { Planaufmass } from "../generated/plattform";
  let {
    projekt,
    benutzerId,
    darf,
  }: { projekt: Project; benutzerId: string; darf: boolean } = $props();
  let gebaeude = $state<Datensatz<Gebaeude>[]>([]);
  let summen = $state<RaumSummen | null>(null);
  let aufmasse = $state<Datensatz<Planaufmass>[]>([]), aufmassId = $state(""), importBusy = $state(false);
  let listenGeneration=0;
  async function plaeneLaden() {
    const n=++listenGeneration;
    try { const liste = await plattformApi.liste<Planaufmass>(s.projektId, "planaufmass"); if(n===listenGeneration)aufmasse=liste; }
    catch(e) { if(n===listenGeneration)s.fehler = fehlertext(e); }
  }
  onMount(()=>{void plaeneLaden();return()=>{listenGeneration++}});
  $effect(()=>abonnieren(projekt.id,["fachobjekte","dokumente"],()=>{summen=null;void plaeneLaden()}));
  async function planraeume() {
    const a=aufmasse.find(a=>a.objekt===aufmassId);
    if(!a||!s.eingabe)return;
    const before=JSON.stringify(s.eingabe);
    importBusy=true;s.fehler=null;
    try {
      const r=await planApi.raumbuch(s.projektId,a.objekt,a.revision,JSON.parse(before));
      if(JSON.stringify(s.eingabe)!==before)throw Error("Raumbuch zwischenzeitlich geändert. Übernahme wiederholen.");
      s.eingabe=r;s.meldung="Planräume im Entwurf übernommen. Raumnummern und Nutzung prüfen, anschließend speichern.";
    } catch(e) { s.fehler=fehlertext(e); } finally { importBusy=false; }
  }
  const s = untrack(
    () =>
      new FachSitzung<Raumbuch>(
        projekt.id,
        "raumbuch",
        benutzerId,
        () => ({
          bezeichnung: "",
          gebaeude_id: gebaeude[0]?.objekt ?? "",
          geschoss_id: null,
          raeume: [],
        }),
        (r) => r.bezeichnung,
      ),
  );
  const geschosse = $derived(
    gebaeude.find((g) => g.objekt === s.eingabe?.gebaeude_id)?.daten
      .geschosse ?? [],
  );
  $effect(() => {
    void s.id;
    void s.revision;
    if (s.hatAenderungen()) summen = null;
  });
  onMount(async () => {
    try {
      gebaeude = await plattformApi.liste<Gebaeude>(projekt.id, "gebaeude");
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
  async function berechnen() {
    if (!s.id) return;
    try {
      if (await s.speichern())
        summen = await plattformApi.raumsummen(projekt.id, s.id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  const z = (n: number) =>
    n.toLocaleString("de-DE", { maximumFractionDigits: 2 });
  // Sprung aus der Projektübersicht zu veralteten Planquellen öffnet den Abschnitt.
  const navigation = projektNavigation();
  let planOffen = $state(false);
  $effect(() => {
    const a = navigation?.auftrag;
    if (a?.ziel.bereich === "raumbuch" && a.ziel.register === "planaufmass") planOffen = true;
  });
</script>

<Fachliste titel="Flächen- und Raumbuch" {s} bereich="raumbuch" {darf} zusatzBeschaeftigt={()=>importBusy}>
  {#snippet children(r)}<div class="raster">
      <label>Bezeichnung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Bezeichnung" bind:value={r.bezeichnung} required /></label
      ><label
        >Gebäude<select
          bind:value={r.gebaeude_id}
          onchange={() => (r.geschoss_id = null)}
          required
          ><option value="">Gebäude auswählen</option
          >{#each gebaeude as g (g.objekt)}<option value={g.objekt}
              >{g.daten.bezeichnung}</option
            >{/each}</select
        ></label
      ><label
        >Geschoss<select bind:value={r.geschoss_id}
          ><option value={null}>Alle Geschosse des Gebäudes</option
          >{#each geschosse as g (g.id)}<option value={g.id}
              >{g.bezeichnung}</option
            >{/each}</select
        ></label
      >
    </div>
    <details bind:open={planOffen}>
      <summary>Räume aus PDF-Aufmaß übernehmen</summary>
      <p class="meta">Übernimmt Raum-, Aufenthaltsraum-, NUF-, TF- und VF-Markierungen des gewählten Gebäudes/Geschosses. Neue Räume werden angelegt; vorhandene Raumnummern, Nutzungen und Höhen bleiben erhalten. Flächen bleiben mit der Planquelle verknüpft.</p>
      <label>PDF-Aufmaß<select aria-label="PDF-Aufmaß" bind:value={aufmassId}><option value="">Aufmaß auswählen</option>{#each aufmasse as a}<option value={a.objekt}>{a.daten.bezeichnung} · R{a.revision}</option>{/each}</select></label>
      <button type="button" disabled={!aufmassId||!r.gebaeude_id||importBusy} onclick={planraeume}>Planräume anlegen / aktualisieren</button>
      {#each r.planbindungen??[] as b}
        {@const a=aufmasse.find(a=>a.objekt===b.aufmass_id)}
        <p class="meta">{a?.daten.bezeichnung??"Planquelle fehlt"} · R{b.revision} · Raum {r.raeume.find(raum=>b.ziel===`raeume.${raum.id}.flaeche_m2`)?.bezeichnung??"fehlt"}
        {#if a&&a.revision!==b.revision}<strong> – Quelle geändert</strong>{/if}
        <button type="button" class="link" onclick={()=>r.planbindungen=r.planbindungen?.filter(q=>q!==b)}>Verknüpfung lösen</button></p>{/each}
    </details>
    <div class="tabellen-scroll">
      <table class="ergebnistabelle">
        <thead
          ><tr
            ><th>Raumnummer</th><th>Bezeichnung</th><th>Nutzung</th><th
              >Erfasste Fläche m²</th
            ><th>Lichte Höhe m</th><th></th></tr
          ></thead
        ><tbody
          >{#each r.raeume as raum, i (raum.id)}<tr
              ><td
                ><input
                  aria-label="Raumnummer"
                  bind:value={raum.nummer}
                  required
                /></td
              ><td
                ><input
                  aria-label="Raumbezeichnung"
                  bind:value={raum.bezeichnung}
                  required
                /></td
              ><td
                ><input
                  aria-label="Raumnutzung"
                  bind:value={raum.nutzung}
                /></td
              ><td
                ><input
                  aria-label="Raumfläche m²"
                  type="number"
                  min="0"
                  step="any"
                  bind:value={raum.flaeche_m2}
                  disabled={r.planbindungen?.some(b=>b.ziel===`raeume.${raum.id}.flaeche_m2`)}
                  required
                /></td
              ><td
                ><input
                  aria-label="Lichte Raumhöhe m"
                  type="number"
                  min="0.01"
                  step="any"
                  bind:value={raum.hoehe_m}
                /></td
              ><td
                ><button
                  type="button"
                  class="link"
                  onclick={() => {r.planbindungen=r.planbindungen?.filter(b=>!b.ziel.startsWith(`raeume.${raum.id}.`));r.raeume.splice(i, 1)}}>Entfernen</button
                ></td
              ></tr
            >{/each}</tbody
        >
      </table>
    </div>
    <button
      type="button"
      class="sekundaer"
      onclick={() =>
        r.raeume.push({
          id: crypto.randomUUID(),
          nummer: "",
          bezeichnung: "",
          nutzung: "",
          flaeche_m2: 0,
          hoehe_m: null,
        })}>Raum hinzufügen</button
    ><button type="button" disabled={s.laeuft} onclick={berechnen}
      >Speichern und Summen ermitteln</button
    >
    <p class="meta">
      Summe der erfassten Raumflächen. Dies ist kein Nachweis nach WoFlV oder
      DIN 277; deren Anrechnungsregeln sind getrennte Fachmodule.
    </p>
    {#if summen}<p role="status">
        {summen.anzahl} Räume · {z(summen.flaeche_m2)} m² · {summen.volumen_m3 ===
        null
          ? "Volumen unvollständig"
          : `${z(summen.volumen_m3)} m³`}
      </p>{/if}
  {/snippet}
</Fachliste>
