<script lang="ts">
  import { bereichAktiv } from "./ansichtsAbfrage.svelte";
  const sichtbar = bereichAktiv();
  import { onMount } from "svelte";
  import type {
    PDFDocumentProxy,
    RenderTask,
    PDFDocumentLoadingTask,
  } from "pdfjs-dist";
  import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
  import type { EigenesPdfFeld } from "./api";
  let {
    pdfBase64 = "",
    pdfDaten,
    aktiv = true,
    seite = 1,
    unrotiert = false,
    felder = [],
    onposition,
    onbereit,
    onfehler,
  }: {
    pdfBase64?: string;
    pdfDaten?: Uint8Array;
    aktiv?: boolean;
    seite?: number;
    unrotiert?: boolean;
    felder?: EigenesPdfFeld[];
    onposition?: (x: number, y: number) => void;
    onbereit?: (pdfBase64: string) => void;
    onfehler: (text: string) => void;
  } = $props();
  let canvas = $state<HTMLCanvasElement>(),
    huelle: HTMLDivElement;
  let pdf = $state<PDFDocumentProxy | null>(null),
    breite = $state(700),
    loading = $state(true);
  let pageWidth = $state(210),
    pageHeight = $state(297);
  let pdfStand = $state("");
  onMount(() => {
    const ro = new ResizeObserver(([entry]) => {
      if (entry.contentRect.width > 0) breite = entry.contentRect.width;
    });
    ro.observe(huelle);
    return () => ro.disconnect();
  });
  $effect(() => {
    const quelle = pdfBase64, daten = pdfDaten;
    if (!aktiv || !sichtbar()) return;
    let gone = false;
    let loader: PDFDocumentLoadingTask | undefined;
    pdf = null;
    loading = true;
    void (async () => {
      const { getDocument, GlobalWorkerOptions } = await import("pdfjs-dist");
      if (gone) return;
      GlobalWorkerOptions.workerSrc = workerUrl;
      loader = getDocument({
        data: daten ? daten.slice() : Uint8Array.from(atob(quelle), (c) => c.charCodeAt(0)),
        isEvalSupported: false,
        cMapUrl: new URL("./pdfjs/cmaps/", document.baseURI).href,
        cMapPacked: true,
        standardFontDataUrl: new URL(
          "./pdfjs/standard_fonts/",
          document.baseURI,
        ).href,
      });
      const doc = await loader.promise;
      if (!gone) {
        pdf = doc;
        pdfStand = quelle;
      }
    })().catch((e) => {
      if (!gone) {
        loading = false;
        onfehler(`PDF-Vorschau: ${String(e)}`);
      }
    });
    return () => {
      gone = true;
      void loader?.destroy();
    };
  });
  $effect(() => {
    const doc = pdf,
      stand = pdfStand,
      page = seite,
      target = canvas,
      width = breite,
      ohneRotation = unrotiert;
    if (!aktiv || !sichtbar() || !doc || !target) return;
    let cancelled = false;
    let task: RenderTask | undefined;
    loading = true;
    void (async () => {
      try {
        const p = await doc.getPage(page);
        if (cancelled) return;
        const rotation = ohneRotation ? 0 : p.rotate;
        const base = p.getViewport({ scale: 1, rotation });
        const scale = Math.min(width, 1000) / base.width;
        const pixel = Math.min(devicePixelRatio || 1, 2);
        const viewport = p.getViewport({ scale: scale * pixel, rotation });
        // Erst vollständig im Hintergrund rendern. Das letzte Bild bleibt bis dahin erhalten.
        const puffer = document.createElement("canvas");
        puffer.width = Math.ceil(viewport.width);
        puffer.height = Math.ceil(viewport.height);
        const ctx = puffer.getContext("2d");
        if (!ctx) throw Error("PDF-Zeichenfläche nicht verfügbar");
        task = p.render({ canvasContext: ctx, viewport });
        await task.promise;
        if (cancelled) return;
        target.width = puffer.width;
        target.height = puffer.height;
        target.style.width = `${base.width * scale}px`;
        target.style.height = `${base.height * scale}px`;
        target.getContext("2d")!.drawImage(puffer, 0, 0);
        pageWidth = (base.width * 25.4) / 72;
        pageHeight = (base.height * 25.4) / 72;
        loading = false;
        onbereit?.(stand);
      } catch (e) {
        if (!cancelled) {
          loading = false;
          onfehler(`PDF-Vorschau: ${String(e)}`);
        }
      }
    })();
    return () => {
      cancelled = true;
      task?.cancel();
    };
  });
  function position(e: MouseEvent) {
    if (!onposition || loading) return;
    const r =
      e.currentTarget instanceof HTMLElement
        ? e.currentTarget.getBoundingClientRect()
        : null;
    if (!r) return;
    onposition(
      e.detail === 0
        ? pageWidth / 2
        : ((e.clientX - r.left) / r.width) * pageWidth,
      e.detail === 0
        ? pageHeight / 2
        : ((e.clientY - r.top) / r.height) * pageHeight,
    );
  }
</script>

<div class="pdf-render-area" bind:this={huelle}>
  <p class="meta pdf-render-status" role="status">
    {loading ? "PDF-Seite wird geladen …" : ""}
  </p>
  <button
    class="pdf-seite"
    type="button"
    disabled={!onposition || loading}
    onclick={position}
    aria-label={onposition
      ? "Feldposition auf PDF wählen; alternativ Koordinaten rechts eingeben"
      : "PDF-Seitenvorschau"}
  >
    <canvas bind:this={canvas} aria-hidden="true"></canvas>
    {#each felder.filter((f) => f.seite === seite) as f, i}<span
        class="pdf-feldmarkierung"
        style:left={`${(f.x_mm / pageWidth) * 100}%`}
        style:top={`${(f.y_mm / pageHeight) * 100}%`}
        style:width={`${(f.breite_mm / pageWidth) * 100}%`}
        style:height={`${(f.hoehe_mm / pageHeight) * 100}%`}
        >{f.art === "ankreuzfeld"
          ? "☑"
          : `<${f.name.trim().replace(/^<\s*|\s*>$/g, "")}>`}</span
      >{/each}
  </button>
</div>

<style>
  .pdf-render-status {
    min-height: 1.3em;
    margin: 0 0 0.35rem;
  }
</style>
