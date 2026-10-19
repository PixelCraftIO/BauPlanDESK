<script lang="ts">
  import { untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    EINHEITEN,
    fehlertext,
    type DocumentVersion,
    type Laengeneinheit,
  } from "../api";
  import type {
    AbstandsWand,
    CadHoehe,
    Dokumentbezug,
    Hoehenpunkt,
  } from "../generated/plattform";
  let {
    wand,
    dateien = [],
    projektId = "",
    onquelle = () => {},
  }: {
    wand: AbstandsWand;
    dateien?: DocumentVersion[];
    projektId?: string;
    onquelle?: (d: Dokumentbezug) => void;
  } = $props();
  // Nur zur Anzeige und als Vorschlag für neue Punkte; gerechnet wird im Core.
  const laenge = $derived(Math.hypot(wand.bis.x - wand.von.x, wand.bis.y - wand.von.y));
  const runden = (n: number) => Math.round(n * 1000) / 1000;
  const vomUmriss = (p: Hoehenpunkt) => (p.quelle ?? "").startsWith("Gebäudeumriss, Ecke ");
  const altwerte = $derived(
    !wand.gelaende?.length &&
      !wand.oberkante?.length &&
      [wand.gelaende_a_m, wand.gelaende_b_m, wand.abschluss_a_m, wand.abschluss_b_m].some((v) => v != null),
  );
  function altwerteUebernehmen() {
    const l = runden(laenge);
    const paar = (a: number | null, b: number | null) =>
      a != null && b != null ? [{ station_m: 0, hoehe_m: a }, { station_m: l, hoehe_m: b }] : [];
    wand.gelaende = paar(wand.gelaende_a_m, wand.gelaende_b_m);
    wand.oberkante = paar(wand.abschluss_a_m, wand.abschluss_b_m);
    wand.gelaende_a_m = wand.gelaende_b_m = wand.abschluss_a_m = wand.abschluss_b_m = null;
  }
  function neu(liste: "gelaende" | "oberkante") {
    const p = wand[liste] ?? [];
    const station = p.length === 0 ? 0 : p.length === 1 ? runden(laenge) : runden(laenge / 2);
    wand[liste] = [...p, { station_m: station, hoehe_m: p.at(-1)?.hoehe_m ?? 0 }].sort(
      (a, b) => a.station_m - b.station_m,
    );
  }
  function entfernen(liste: "gelaende" | "oberkante", i: number) {
    wand[liste] = (wand[liste] ?? []).filter((_, n) => n !== i);
  }
  /** Eigene Eingabe ersetzt eine übernommene Vorgabe als Quelle. */
  function bearbeitet(p: Hoehenpunkt) {
    if ((p.quelle ?? "").startsWith("Gebäudeumriss, Vorgabe")) p.quelle = "Eingabe";
  }
  // Die Komponente wird je Wand neu erzeugt ({#key}); Startwert genügt.
  let teilgrenzen = $state(untrack(() => (wand.wandteilgrenzen ?? []).join("; ")));
  function grenzenSetzen() {
    wand.wandteilgrenzen = teilgrenzen
      .split(/[;\s]+/)
      .map((t) => Number(t.replace(",", ".")))
      .filter((n) => Number.isFinite(n) && n > 0);
  }
  // CAD-Höhen
  let dokument = $state(""),
    einheit = $state<Laengeneinheit>("meter"),
    abstand = $state(1),
    hoehen = $state<CadHoehe[]>([]),
    ebene = $state(""),
    vorschlag = $state<Hoehenpunkt[]>([]),
    busy = $state(false),
    fehler = $state("");
  const ebenen = $derived([...new Set(hoehen.map((h) => h.ebene))].sort());
  async function hoehenLaden() {
    const d = dateien.find((d) => `${d.document_id}:${d.version}` === dokument);
    if (!d) return;
    busy = true;
    fehler = "";
    vorschlag = [];
    try {
      hoehen = await invoke<CadHoehe[]>("abstandscad_hoehen", {
        projektId,
        dokument: { dokument_id: d.document_id, version: d.version },
        einheit,
      });
      ebene = "";
      if (!hoehen.length) fehler = "Keine Höhenkoten (Zahlen mit Dezimalstellen) oder Punkte mit Höhe gefunden.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  async function vorschlagen() {
    busy = true;
    fehler = "";
    try {
      vorschlag = await invoke<Hoehenpunkt[]>("abstand_gelaende_vorschlag", {
        wand: $state.snapshot(wand),
        hoehen: $state.snapshot(hoehen.filter((h) => !ebene || h.ebene === ebene)),
        maxAbstandM: abstand,
      });
      if (!vorschlag.length) fehler = "Keine Höhen in diesem Abstand zur Wand.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  function vorschlagUebernehmen() {
    const eigene = (wand.gelaende ?? []).filter(vomUmriss);
    wand.gelaende = [...eigene, ...$state.snapshot(vorschlag)].sort((a, b) => a.station_m - b.station_m);
    if (hoehen[0]) onquelle(hoehen[0].quelle);
    vorschlag = [];
  }
</script>

<fieldset class="profil">
  <legend>Höhenprofil der Wand · Länge {laenge.toLocaleString("de-DE", { maximumFractionDigits: 3 })} m</legend>
  <p class="meta">
    Stationen in Metern ab Wandanfang, Höhen im gemeinsamen Höhenbezug. Zwischen
    den Punkten wird linear interpoliert; beide Profile müssen von 0 m bis zur
    Wandlänge reichen. Wandoberkante = Schnittlinie mit der Dachhaut bzw. oberer
    Wandabschluss. Zwei Oberkantenpunkte an derselben Station bilden einen
    Höhensprung und damit einen eigenen Wandteil. Abgrabungen für Belichtung,
    Zugang oder Zufahrt nicht als Geländeoberfläche ansetzen, soweit das
    Landesrecht sie außer Betracht lässt.
  </p>
  {#if altwerte}<p class="hinweisbox" role="status">
      Diese Wand nutzt noch die Einzelwerte an Anfang und Ende
      (Gelände {wand.gelaende_a_m ?? "–"} / {wand.gelaende_b_m ?? "–"} m,
      Abschluss {wand.abschluss_a_m ?? "–"} / {wand.abschluss_b_m ?? "–"} m).
      <button type="button" class="sekundaer" onclick={altwerteUebernehmen}
        >In Höhenprofil übernehmen</button
      >
    </p>{/if}
  {#each [["gelaende", "Geländeoberfläche"], ["oberkante", "Wandoberkante"]] as [liste, titel] (liste)}
    {@const art = liste as "gelaende" | "oberkante"}
    <h5>{titel}</h5>
    <table class="ergebnistabelle">
      <thead><tr><th>Station (m)</th><th>Höhe (m)</th><th>Quelle</th><th></th></tr></thead>
      <tbody>
        {#each wand[art] ?? [] as p, i}<tr>
            <td
              ><input
                aria-label={`${titel} Station ${i + 1}`}
                type="number"
                step="any"
                bind:value={p.station_m}
                readonly={vomUmriss(p)}
              /></td
            ><td
              ><input
                aria-label={`${titel} Höhe ${i + 1}`}
                type="number"
                step="any"
                bind:value={p.hoehe_m}
                readonly={vomUmriss(p)}
                oninput={() => bearbeitet(p)}
              /></td
            ><td
              ><input
                aria-label={`${titel} Quelle ${i + 1}`}
                value={p.quelle ?? ""}
                readonly={vomUmriss(p)}
                oninput={(e) => (p.quelle = e.currentTarget.value)}
              /></td
            ><td
              >{#if vomUmriss(p)}<span class="meta">aus Umriss</span>{:else}<button
                  type="button"
                  class="link"
                  onclick={() => entfernen(art, i)}>Entfernen</button
                >{/if}</td
            >
          </tr>{/each}
      </tbody>
    </table>
    <button type="button" class="sekundaer" onclick={() => neu(art)}>{titel}: Punkt hinzufügen</button>
  {/each}
  <label
    >Zusätzliche Wandteilgrenzen (Stationen in m, mit Semikolon getrennt)<input
      bind:value={teilgrenzen}
      onchange={grenzenSetzen}
      placeholder="z. B. 4,5; 9"
    /></label
  >
  {#if projektId && dateien.some((d) => d.file_name.toLowerCase().endsWith(".dxf"))}<details>
      <summary>Geländehöhen aus DXF-Lageplan vorschlagen</summary>
      <p class="meta">
        Liest Höhenkoten (Text mit Dezimalstellen, z. B. „101,35“) und Punkte mit
        Höhe. Übernommen werden Höhen bis zum gewählten Abstand neben der
        Wandlinie, projiziert auf die Wand. Ob die Koten das maßgebliche Gelände
        an der Wand beschreiben, bleibt fachlich zu prüfen.
      </p>
      <div class="raster">
        <label
          >DXF-Quellversion<select bind:value={dokument} onchange={() => (hoehen = [])}
            ><option value="">Quelle wählen</option
            >{#each dateien.filter((d) => d.file_name.toLowerCase().endsWith(".dxf")) as d}<option
                value={`${d.document_id}:${d.version}`}>{d.file_name} · Version {d.version}</option
              >{/each}</select
          ></label
        ><label
          >Zeichnungseinheit<select bind:value={einheit} onchange={() => (hoehen = [])}
            >{#each EINHEITEN as e}<option value={e.code}>{e.name}</option>{/each}</select
          ></label
        ><label
          >Suchabstand zur Wand (m)<input type="number" min="0" max="10" step="0.1" bind:value={abstand} /></label
        >
      </div>
      <button type="button" class="sekundaer" disabled={busy || !dokument} onclick={hoehenLaden}
        >DXF-Höhen laden</button
      >
      {#if hoehen.length}<label
          >Ebene<select bind:value={ebene}
            ><option value="">Alle Ebenen ({hoehen.length} Höhen)</option
            >{#each ebenen as e}<option value={e}>{e}</option>{/each}</select
          ></label
        ><button type="button" class="sekundaer" disabled={busy} onclick={vorschlagen}
          >Geländepunkte vorschlagen</button
        >{/if}
      {#if fehler}<p role="alert">{fehler}</p>{/if}
      {#if vorschlag.length}<table class="ergebnistabelle">
          <thead><tr><th>Station (m)</th><th>Höhe (m)</th><th>Quelle</th></tr></thead>
          <tbody
            >{#each vorschlag as p}<tr><td>{p.station_m}</td><td>{p.hoehe_m}</td><td>{p.quelle}</td></tr>{/each}</tbody
          >
        </table>
        <button type="button" onclick={vorschlagUebernehmen}
          >Vorschlag als Geländeprofil übernehmen (ersetzt eigene Geländepunkte)</button
        >{/if}
    </details>{/if}
</fieldset>

<style>
  .profil {
    margin: 1rem 0;
  }
  .profil input {
    min-width: 6rem;
  }
  h5 {
    margin: 0.8rem 0 0.3rem;
  }
</style>
