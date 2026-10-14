<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type {
    PDFDocumentProxy,
    RenderTask,
    PDFDocumentLoadingTask,
  } from "pdfjs-dist";
  import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
  import type { EigenesPdfFeld } from "./api";
  let {
    pdfBase64,
    seite = 1,
    unrotiert = false,
    felder = [],
    onposition,
    onfehler,
  }: {
    pdfBase64: string;
    seite?: number;
    unrotiert?: boolean;
    felder?: EigenesPdfFeld[];
    onposition?: (x: number, y: number) => void;
    onfehler: (text: string) => void;
  } = $props();
  let canvas = $state<HTMLCanvasElement>(),
    huelle: HTMLDivElement;
  let pdf = $state<PDFDocumentProxy | null>(null),
    breite = $state(700),
    loading = $state(true);
  let pageWidth = $state(210),
    pageHeight = $state(297);
  let task: RenderTask | undefined;
  onMount(() => {
    let gone = false;
    let loader: PDFDocumentLoadingTask | undefined;
    void (async () => {
      const { getDocument, GlobalWorkerOptions } = await import("pdfjs-dist");
      if (gone) return;
      GlobalWorkerOptions.workerSrc = workerUrl;
      const bytes = Uint8Array.from(atob(pdfBase64), (c) => c.charCodeAt(0));
      loader = getDocument({
        data: bytes,
        isEvalSupported: false,
        cMapUrl: new URL("./pdfjs/cmaps/", document.baseURI).href,
        cMapPacked: true,
        standardFontDataUrl: new URL(
          "./pdfjs/standard_fonts/",
          document.baseURI,
        ).href,
      });
      const doc = await loader.promise;
      if (!gone) pdf = doc;
    })().catch((e) => {
      if (!gone) {
        loading = false;
        onfehler(String(e));
      }
    });
    const ro = new ResizeObserver(([entry]) => {
      if (entry.contentRect.width > 0) breite = entry.contentRect.width;
    });
    ro.observe(huelle);
    return () => {
      gone = true;
      ro.disconnect();
      void loader?.destroy();
    };
  });
  $effect(() => {
    const document = pdf;
    const page = seite;
    const target = canvas;
    const width = breite;
    if (!document || !target) return;
    let cancelled = false;
    loading = true;
    void (async () => {
      try {
        const p = await document.getPage(page);
        if (cancelled) return;
        // Positions for custom fields use the unrotated, visible PDF page box.
        const rotation=unrotiert?0:p.rotate;
        const base = p.getViewport({ scale: 1, rotation });
        pageWidth = (base.width * 25.4) / 72;
        pageHeight = (base.height * 25.4) / 72;
        const scale = Math.min(width, 1000) / base.width;
        const pixel = Math.min(devicePixelRatio || 1, 2);
        const viewport = p.getViewport({ scale: scale * pixel, rotation });
        target.width = viewport.width;
        target.height = viewport.height;
        target.style.width = `${base.width * scale}px`;
        target.style.height = `${base.height * scale}px`;
        const ctx = target.getContext("2d");
        if (!ctx) throw Error("PDF-Zeichenfläche nicht verfügbar");
        task = p.render({ canvasContext: ctx, viewport });
        await task.promise;
        if (!cancelled) loading = false;
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
  onDestroy(() => task?.cancel());
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
  {#if loading}<p class="meta" role="status">PDF-Seite wird geladen …</p>{/if}
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
