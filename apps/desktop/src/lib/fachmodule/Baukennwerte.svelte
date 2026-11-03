<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";
  import { fehlertext, type Project } from "../api";
  import type {
    Baukennwerte,
    Baustatistikbogen,
    Gebuehrenschaetzung,
  } from "../generated/plattform";
  import { plattformApi } from "../plattformApi";
  import { ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  import { editorAnmelden } from "../editoren";
  let { projekt, darf }: { projekt: Project; benutzerId: string; darf: boolean } =
    $props();
  const VORGABE: Baukennwerte = {
    gebaeudeart: "wohngebaeude",
    bri_m3: null,
    euro_je_m3: null,
    promille: null,
    bautaetigkeit: "neubau",
    bauherrenart: null,
    wohnungen: null,
    heizung: "",
    energie_primaer: "",
    energie_sekundaer: "",
    baustoff: "",
    fertigteilbau: false,
    kosten_bauwerk_euro: null,
  };
  let eingabe = $state<Baukennwerte>(structuredClone(VORGABE)),
    original = $state(JSON.stringify(VORGABE)),
    basis = $state(0),
    schaetzung = $state<Gebuehrenschaetzung | null>(null),
    statistik = $state<Baustatistikbogen | null>(null),
    fehler = $state<string | null>(null),
    meldung = $state<string | null>(null),
    laeuft = $state(false);
  const geaendert = $derived(JSON.stringify(eingabe) !== original);
  const zahl = (v: number | null | undefined, n = 0) =>
    v == null
      ? "–"
      : v.toLocaleString("de-DE", {
          minimumFractionDigits: n,
          maximumFractionDigits: n,
        });
  async function auswerten() {
    const [s, b] = await Promise.all([
      invoke<Gebuehrenschaetzung>("gebuehrenschaetzung", { projektId: projekt.id }),
      invoke<Baustatistikbogen>("baustatistik", { projektId: projekt.id }),
    ]);
    schaetzung = s;
    statistik = b;
  }
  async function laden() {
    fehler = null;
    try {
      const d = await plattformApi.lesen<Baukennwerte>(
        projekt.id,
        "baukennwerte",
        projekt.id,
      );
      if (!geaendert) {
        eingabe = d ? { ...structuredClone(VORGABE), ...d.daten } : structuredClone(VORGABE);
        original = JSON.stringify(eingabe);
        basis = d?.revision ?? 0;
      }
      await auswerten();
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  ansichtsAbfrage(() => projekt.id, ["fachobjekte"], laden);
  async function speichern(): Promise<boolean> {
    if (!darf || !geaendert) return true;
    laeuft = true;
    fehler = null;
    try {
      const d = await plattformApi.speichern(
        projekt.id,
        "baukennwerte",
        projekt.id,
        basis,
        $state.snapshot(eingabe),
      );
      basis = d.revision;
      original = JSON.stringify(eingabe);
      await auswerten();
      meldung = "Gespeichert.";
      return true;
    } catch (e) {
      fehler = fehlertext(e);
      return false;
    } finally {
      laeuft = false;
    }
  }
  editorAnmelden("baukennwerte", {
    hatAenderungen: () => geaendert,
    istBeschaeftigt: () => laeuft,
    speichern,
  });
  async function pdf(art: "gebuehr" | "statistik") {
    if (geaendert && !(await speichern())) return;
    const name = art === "statistik" ? "Baustatistik" : "Gebuehrenschaetzung";
    const ziel = await save({
      defaultPath: `${name} ${projekt.name.replace(/[<>:"/\\|?*]/g, "_")}.pdf`,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (!ziel) return;
    laeuft = true;
    try {
      await invoke("baukennwerte_pdf", {
        projektId: projekt.id,
        statistik: art === "statistik",
        ziel,
      });
      meldung = `PDF ausgegeben: ${ziel}`;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  /** Leere Zahlenfelder bleiben `null` (= abgeleitet). */
  const leerNull = (v: unknown) =>
    v === "" || v == null || Number.isNaN(v) ? null : Number(v);
</script>

<section class="baukennwerte" aria-busy={laeuft}>
  <h2>Rohbauwert, Gebühr und Baustatistik</h2>
  <p class="meta">
    Schätzung der Baugenehmigungsgebühr aus dem Brutto-Rauminhalt und Angaben
    für den Erhebungsbogen der Baustatistik. Werte aus den Berechnungen werden
    übernommen; ein Eintrag hier überschreibt sie.
  </p>
  {#if fehler}<p role="alert">{fehler}</p>{/if}
  {#if meldung}<p role="status">{meldung}</p>{/if}
  <fieldset disabled={!darf}>
    <legend>Gebührenschätzung</legend>
    <div class="raster">
      <label
        >Gebäudeart<select bind:value={eingabe.gebaeudeart}
          >{#each schaetzung?.gebaeudearten ?? [] as a (a.id)}<option value={a.id}
              >{a.bezeichnung}</option
            >{:else}<option value={eingabe.gebaeudeart}>{eingabe.gebaeudeart}</option
            >{/each}</select
        ></label
      ><label
        >Brutto-Rauminhalt m³<input
          type="number"
          min="0"
          step="1"
          placeholder={schaetzung?.bri_m3.quelle ? `aus ${schaetzung.bri_m3.quelle}` : "aus DIN 277"}
          value={eingabe.bri_m3 ?? ""}
          oninput={(e) => (eingabe.bri_m3 = leerNull(e.currentTarget.valueAsNumber))}
        /></label
      ><label
        >Rohbauwert €/m³<input
          type="number"
          min="0"
          step="0.01"
          placeholder="Wert des Regelpakets"
          value={eingabe.euro_je_m3 ?? ""}
          oninput={(e) => (eingabe.euro_je_m3 = leerNull(e.currentTarget.valueAsNumber))}
        /></label
      ><label
        >Gebührensatz ‰<input
          type="number"
          min="0"
          step="0.1"
          placeholder="Wert des Regelpakets"
          value={eingabe.promille ?? ""}
          oninput={(e) => (eingabe.promille = leerNull(e.currentTarget.valueAsNumber))}
        /></label
      >
    </div>
  </fieldset>
  {#if schaetzung}
    {#if !schaetzung.geprueft}<p class="warnung" role="note">
        Ungeprüfte Regelwerte: {schaetzung.stand}. Rohbauwert je m³ und
        Gebührensatz mit dem aktuellen Runderlass und Gebührentarif vergleichen.
      </p>{/if}
    <table>
      <tbody>
        <tr
          ><th>Brutto-Rauminhalt</th><td>{zahl(schaetzung.bri_m3.wert)} m³</td><td
            class="meta">{schaetzung.bri_m3.quelle}</td
          ></tr
        >
        <tr
          ><th>Rohbauwert je m³</th><td>{zahl(schaetzung.euro_je_m3.wert, 2)} €</td><td
            class="meta">{schaetzung.euro_je_m3.quelle}</td
          ></tr
        >
        <tr><th>Rohbauwert</th><td>{zahl(schaetzung.rohbauwert_euro)} €</td><td></td></tr>
        <tr
          ><th>Gebührensatz</th><td>{zahl(schaetzung.promille.wert, 2)} ‰</td><td
            class="meta">{schaetzung.promille.quelle}</td
          ></tr
        >
        <tr class="summe"
          ><th>Geschätzte Gebühr</th><td>{zahl(schaetzung.gebuehr_euro)} €</td><td
            class="meta">mindestens {zahl(schaetzung.mindestgebuehr_euro)} €</td
          ></tr
        >
      </tbody>
    </table>
    {#each schaetzung.offen as o (o)}<p class="warnung">Offen: {o}</p>{/each}
    <details>
      <summary>Hinweise und Quellen</summary>
      {#each schaetzung.hinweise as h (h)}<p class="meta">{h}</p>{/each}
      <ul>
        {#each schaetzung.quellen as q (q.titel)}<li>{q.titel}</li>{/each}
      </ul>
    </details>
  {/if}
  <fieldset disabled={!darf}>
    <legend>Angaben zur Baustatistik</legend>
    <div class="raster">
      <label
        >Bautätigkeit<select bind:value={eingabe.bautaetigkeit}
          ><option value="neubau">Neubau</option><option value="bestand"
            >Baumaßnahme am Bestand</option
          ></select
        ></label
      ><label
        >Bauherr<select bind:value={eingabe.bauherrenart}
          ><option value={null}>Aus der Kundenkategorie</option><option
            value="privater_haushalt">Private Haushalte</option
          ><option value="unternehmen">Unternehmen</option><option
            value="oeffentliche_hand">Öffentlicher Bauherr</option
          ><option value="organisation_ohne_erwerbszweck"
            >Organisation ohne Erwerbszweck</option
          ></select
        ></label
      ><label
        >Wohnungen<input
          type="number"
          min="0"
          step="1"
          placeholder="aus der Wohnflächenberechnung"
          value={eingabe.wohnungen ?? ""}
          oninput={(e) => (eingabe.wohnungen = leerNull(e.currentTarget.valueAsNumber))}
        /></label
      ><label
        >Baustoff<input
          bind:value={eingabe.baustoff}
          maxlength="300"
          placeholder="z. B. Mauerwerk aus Kalksandstein"
        /></label
      ><label
        >Heizung<input
          bind:value={eingabe.heizung}
          maxlength="300"
          placeholder="z. B. Wärmepumpe"
        /></label
      ><label
        >Energie primär<input
          bind:value={eingabe.energie_primaer}
          maxlength="300"
          placeholder="z. B. Umweltthermie (Luft)"
        /></label
      ><label
        >Energie sekundär<input bind:value={eingabe.energie_sekundaer} maxlength="300" /></label
      ><label
        >Kosten Bauwerk (KG 300 + 400) €<input
          type="number"
          min="0"
          step="1000"
          value={eingabe.kosten_bauwerk_euro ?? ""}
          oninput={(e) =>
            (eingabe.kosten_bauwerk_euro = leerNull(e.currentTarget.valueAsNumber))}
        /></label
      ><label class="checkbox"
        ><input type="checkbox" bind:checked={eingabe.fertigteilbau} />Fertigteilbau</label
      >
    </div>
  </fieldset>
  {#if statistik}
    <table>
      <thead><tr><th>Merkmal</th><th>Angabe</th><th>Quelle</th></tr></thead>
      <tbody>
        {#each statistik.merkmale as m (m.gruppe + m.bezeichnung)}
          <tr class:offen={!m.wert}
            ><td>{m.gruppe}: {m.bezeichnung}</td><td>{m.wert || "– offen –"}</td><td
              class="meta">{m.quelle}</td
            ></tr
          >
        {/each}
      </tbody>
    </table>
    <p class="meta">{statistik.hinweis}</p>
  {/if}
  <div class="aktionen">
    {#if darf}<button type="button" onclick={speichern} disabled={!geaendert || laeuft}
        >Speichern</button
      >{/if}
    <button type="button" class="sekundaer" onclick={() => pdf("gebuehr")} disabled={laeuft}
      >Gebührenschätzung als PDF …</button
    >
    <button type="button" class="sekundaer" onclick={() => pdf("statistik")} disabled={laeuft}
      >Baustatistik als PDF …</button
    >
  </div>
</section>

<style>
  .warnung {
    padding: var(--space-2);
    background: var(--warn-bg);
    color: var(--warning-text);
    border: 1px solid var(--warning-text);
  }
  .summe th,
  .summe td {
    font-weight: 600;
  }
  tr.offen td {
    background: var(--warn-bg);
  }
  .aktionen {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
</style>
