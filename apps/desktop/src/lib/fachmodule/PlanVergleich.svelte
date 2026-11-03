<script lang="ts">
  /**
   * Planvergleich zweier Dateiversionen eines Plans (Seite 1): Differenzbild,
   * erkannte Änderungsbereiche, Revisionswolken als PDF und Vorschlag für den
   * nächsten Index. Reine Anzeige; die Planliste speichert erst auf Knopfdruck.
   */
  import { save } from "@tauri-apps/plugin-dialog";
  import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
  import { api, fehlertext } from "../api";
  import { differenz, bereiche as bereicheFinden } from "../planaufmass/planvergleich";
  let { projektId, dokumentId, plannummer, versionen, aktuell, onindex }:
    { projektId: string; dokumentId: string; plannummer: string; versionen: number[]; aktuell: number; onindex: (aenderung: string) => void } = $props();
  let alt = $state(0), laeuft = $state(false), fehler = $state(""), meldung = $state("");
  let bild = $state(""), groesse = $state({ breite: 0, hoehe: 0 }), skala = 1;
  let gefunden = $state<[number, number, number, number][]>([]), indexText = $state("");
  $effect(() => { if (!alt) alt = versionen.filter(v => v < aktuell).at(-1) ?? 0; });

  async function rendern(version: number, breitePx: number | null) {
    const { getDocument, GlobalWorkerOptions } = await import("pdfjs-dist");
    GlobalWorkerOptions.workerSrc = workerUrl;
    const daten = await api.pdfVorschau(dokumentId, version);
    const pdf = await getDocument({ data: Uint8Array.from(atob(daten.pdf_base64), c => c.charCodeAt(0)), isEvalSupported: false }).promise;
    try {
      const seite = await pdf.getPage(1), v1 = seite.getViewport({ scale: 1 });
      // Etwa 2400 px Breite: genug für Strichänderungen, ohne den Speicher zu sprengen.
      const s = breitePx ? breitePx / v1.width : Math.min(4, 2400 / Math.max(v1.width, v1.height));
      const vp = seite.getViewport({ scale: s });
      const c = document.createElement("canvas");
      c.width = Math.round(vp.width); c.height = Math.round(vp.height);
      const ctx = c.getContext("2d", { willReadFrequently: true })!;
      ctx.fillStyle = "white"; ctx.fillRect(0, 0, c.width, c.height);
      await seite.render({ canvasContext: ctx, viewport: vp }).promise;
      return { daten: ctx.getImageData(0, 0, c.width, c.height), skala: s };
    } finally { void pdf.destroy(); }
  }
  async function vergleichen() {
    if (!alt) return;
    laeuft = true; fehler = ""; meldung = ""; gefunden = [];
    try {
      const a = await rendern(alt, null);
      const b = await rendern(aktuell, a.daten.width);
      if (a.daten.height !== b.daten.height) throw new Error("Die Versionen haben unterschiedliche Seitenformate; ein Vergleich ist nur bei gleichem Blatt sinnvoll.");
      const { bild: d, maske } = differenz(a.daten.data, b.daten.data, a.daten.width, a.daten.height);
      const c = document.createElement("canvas");
      c.width = a.daten.width; c.height = a.daten.height;
      c.getContext("2d")!.putImageData(new ImageData(d, c.width, c.height), 0, 0);
      bild = c.toDataURL("image/png"); groesse = { breite: c.width, hoehe: c.height }; skala = a.skala;
      gefunden = bereicheFinden(maske, c.width, c.height, Math.max(8, Math.round(c.width / 150)), 2, 12)
        .map(([x, y, w, h]) => [x - 6, y - 6, w + 12, h + 12]);
      meldung = gefunden.length ? `${gefunden.length} geänderte Bereiche gefunden.` : "Keine Änderungen auf Seite 1 gefunden.";
    } catch (e) { fehler = fehlertext(e); } finally { laeuft = false; }
  }
  async function wolkenSpeichern() {
    if (!gefunden.length) return;
    const ziel = await save({ defaultPath: `${plannummer || "Plan"} Revisionswolken.pdf`.replace(/[<>:"/\\|?*]/g, "_"), filters: [{ name: "PDF", extensions: ["pdf"] }] });
    if (!ziel) return;
    laeuft = true;
    try {
      await api.planRevisionswolken(projektId, { dokument_id: dokumentId, version: aktuell, seite: 1, bereiche: gefunden.map(([x, y, w, h]) => [x / skala, y / skala, w / skala, h / skala]), index: indexText.trim() }, ziel);
      meldung = `Plan mit ${gefunden.length} Revisionswolken gespeichert: ${ziel}. Das Original im Projekt bleibt unverändert.`;
    } catch (e) { fehler = fehlertext(e); } finally { laeuft = false; }
  }
</script>

<section class="planvergleich" aria-label="Planvergleich">
  <h3>Planvergleich</h3>
  {#if versionen.filter(v => v < aktuell).length === 0}<p class="meta">Erst ab der zweiten Dateiversion möglich.</p>{:else}
  <div class="zeile">
    <label>Vergleichen mit Version<select bind:value={alt}>{#each versionen.filter(v => v < aktuell) as v}<option value={v}>V{v}</option>{/each}</select></label>
    <span class="meta">gegen aktuelle V{aktuell}, Seite 1</span>
    <button type="button" class="sekundaer" disabled={laeuft || !alt} onclick={() => void vergleichen()}>{laeuft ? "Vergleicht …" : "Vergleichen"}</button>
  </div>
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if meldung}<p class="meta" role="status">{meldung}</p>{/if}
  {#if bild}
    <p class="legende"><span class="rot">■</span> entfallen <span class="gruen">■</span> neu <span class="grau">■</span> unverändert</p>
    <div class="bild"><svg viewBox={`0 0 ${groesse.breite} ${groesse.hoehe}`} role="img" aria-label="Differenzbild mit Änderungsbereichen">
      <image href={bild} width={groesse.breite} height={groesse.hoehe}/>
      {#each gefunden as [x, y, w, h], i}<rect {x} {y} width={w} height={h} class="bereich"><title>Bereich {i + 1}</title></rect>{/each}
    </svg></div>
    {#if gefunden.length}
      <div class="zeile">
        <label>Index im Wolkendreieck<input bind:value={indexText} maxlength="10" placeholder="z. B. b"/></label>
        <button type="button" class="sekundaer" disabled={laeuft} onclick={() => void wolkenSpeichern()}>Revisionswolken als PDF …</button>
        <button type="button" class="sekundaer" onclick={() => onindex(`Änderungen in ${gefunden.length} Bereich${gefunden.length === 1 ? "" : "en"} (Planvergleich V${alt} → V${aktuell})`)}>Als Änderung für den neuen Index übernehmen</button>
      </div>
    {/if}
  {/if}
  {/if}
</section>

<style>
  .planvergleich{margin:.8rem 0;padding:.5rem;border:1px solid var(--border)}
  .planvergleich h3{margin:0 0 .4rem}
  .zeile{display:flex;flex-wrap:wrap;gap:.6rem;align-items:end;margin:.3rem 0}
  .bild{max-height:60vh;overflow:auto;border:1px solid var(--border-innen);background:white}
  .bild svg{width:100%;height:auto;display:block}
  .bereich{fill:none;stroke:#c8102e;stroke-width:4;stroke-dasharray:12 6}
  .legende{font-size:.85rem;display:flex;gap:.6rem;align-items:center}
  .rot{color:#c8141e}.gruen{color:#148c32}.grau{color:#969696}
</style>
