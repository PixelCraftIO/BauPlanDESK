<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import {
    EINHEITEN,
    fehlertext,
    type DocumentVersion,
    type Laengeneinheit,
  } from "../api";
  import type {
    APunkt,
    CadKontur,
    Dokumentbezug,
  } from "../generated/plattform";
  let {
    werte = $bindable(),
    label = "Eckpunkte (Meter)",
    dateien = [],
    projektId = "",
    onquelle = () => {},
    onwerte,
    nurImport = false,
  }: {
    werte: APunkt[];
    label?: string;
    dateien?: DocumentVersion[];
    projektId?: string;
    onquelle?: (d: Dokumentbezug) => void;
    /** Statt `werte` zu setzen, übernommene Punkte an den Aufrufer geben. */
    onwerte?: (punkte: APunkt[]) => void;
    /** Nur die DXF-Übernahme zeigen, ohne Punkttabelle. */
    nurImport?: boolean;
  } = $props();
  let dokument = $state(""),
    einheit = $state<Laengeneinheit>("meter"),
    konturen = $state<CadKontur[]>([]),
    index = $state(0),
    bestaetigt = $state(false),
    busy = $state(false),
    fehler = $state("");
  async function laden() {
    const d = dateien.find((d) => `${d.document_id}:${d.version}` === dokument);
    if (!d) return;
    busy = true;
    fehler = "";
    konturen = [];
    bestaetigt = false;
    try {
      konturen = await invoke<CadKontur[]>("abstandscad_konturen", {
        projektId,
        dokument: { dokument_id: d.document_id, version: d.version },
        einheit,
      });
      if (!konturen.length)
        fehler =
          "Keine geschlossenen Konturen mit 3 bis 2000 Punkten gefunden.";
      index = 0;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  function uebernehmen() {
    const c = konturen[index];
    if (!c || !bestaetigt) return;
    const punkte = structuredClone($state.snapshot(c.punkte));
    if (onwerte) onwerte(punkte);
    else werte = punkte;
    onquelle(c.quelle);
    konturen = [];
  }
</script>

<details>
  <summary>{label}{#if !nurImport} · {werte.length} Punkte{/if}</summary>
  {#if !nurImport}<p class="meta">
    Koordinaten in Metern und gemeinsamem Bezug. Umriss ohne doppelte
    Schlusszeile erfassen.
  </p>
  <table class="ergebnistabelle">
    <thead><tr><th>Punkt</th><th>X (m)</th><th>Y (m)</th><th></th></tr></thead
    ><tbody>
      {#each werte as p, i}<tr
          ><td>{i + 1}</td><td
            ><input
              aria-label={`${label} X ${i + 1}`}
              type="number"
              step="any"
              bind:value={p.x}
            /></td
          ><td
            ><input
              aria-label={`${label} Y ${i + 1}`}
              type="number"
              step="any"
              bind:value={p.y}
            /></td
          ><td
            ><button
              type="button"
              class="link"
              onclick={() => (werte = werte.filter((_, n) => n !== i))}
              >Punkt entfernen</button
            ></td
          ></tr
        >{/each}
    </tbody>
  </table>
  <button
    type="button"
    class="sekundaer"
    onclick={() => (werte = [...werte, { x: 0, y: 0 }])}
    >Eckpunkt hinzufügen</button
  >{/if}
  {#if projektId && dateien.some((d) => d.file_name
        .toLowerCase()
        .endsWith(".dxf"))}<details>
      <summary>Geschlossene DXF-Kontur übernehmen</summary>
      <p class="meta">
        Zeichnungseinheit bewusst wählen, Kontur und Lage prüfen. Bögen werden
        vom CAD-Adapter angenähert; Mess-/Vermessungsqualität gesondert
        bestätigen. Höchstens 200 geschlossene Konturen.
      </p>
      <label
        >DXF-Quellversion<select
          bind:value={dokument}
          onchange={() => (konturen = [])}
          ><option value="">Quelle wählen</option
          >{#each dateien.filter((d) => d.file_name
              .toLowerCase()
              .endsWith(".dxf")) as d}<option
              value={`${d.document_id}:${d.version}`}
              >{d.file_name} · Version {d.version}</option
            >{/each}</select
        ></label
      ><label
        >Zeichnungseinheit<select
          bind:value={einheit}
          onchange={() => (konturen = [])}
          >{#each EINHEITEN as e}<option value={e.code}>{e.name}</option
            >{/each}</select
        ></label
      ><button
        type="button"
        class="sekundaer"
        disabled={busy || !dokument}
        onclick={laden}>DXF-Konturen laden</button
      >{#if fehler}<p role="alert">{fehler}</p>{/if}{#if konturen.length}<label
          >Kontur<select bind:value={index}
            >{#each konturen as c, i}<option value={i}
                >Linie {c.linie + 1} · {c.ebene} · {c.punkte.length} Punkte</option
              >{/each}</select
          ></label
        ><label
          ><input type="checkbox" bind:checked={bestaetigt} />Einheit,
          Koordinatenbezug und gewählte Kontur geprüft</label
        ><button type="button" disabled={!bestaetigt} onclick={uebernehmen}
          >Kontur als Eckpunkte übernehmen</button
        >{/if}
    </details>{/if}
</details>
