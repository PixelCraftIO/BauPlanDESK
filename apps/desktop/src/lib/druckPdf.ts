import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
let aufraeumen: (() => void) | null = null;
/** Vollständige PDF-Seiten im isolierten Druckbereich. Keine Fachberechnung. */
export async function druckPdf(
  base64: string,
  onfortschritt: (seite: number, gesamt: number) => void = () => {},
) {
  aufraeumen?.();
  const { getDocument, GlobalWorkerOptions } = await import("pdfjs-dist");
  GlobalWorkerOptions.workerSrc = workerUrl;
  const loader = getDocument({
    data: Uint8Array.from(atob(base64), (c) => c.charCodeAt(0)),
    isEvalSupported: false,
    cMapUrl: new URL("./pdfjs/cmaps/", document.baseURI).href,
    cMapPacked: true,
    standardFontDataUrl: new URL("./pdfjs/standard_fonts/", document.baseURI)
      .href,
  });
  const urls: string[] = [];
  const root = document.createElement("div");
  root.id = "bau-druckbereich";
  root.style.display = "none";
  const css = document.createElement("style");
  css.textContent = `@media print {html,body{height:auto!important;overflow:visible!important;margin:0!important;padding:0!important;background:white!important}body>:not(#bau-druckbereich){display:none!important}#bau-druckbereich{display:block!important}#bau-druckbereich .druckseite{break-after:page;margin:0;padding:0;overflow:hidden;background:white;break-inside:avoid}#bau-druckbereich .druckseite:last-child{break-after:auto}#bau-druckbereich img{display:block;width:100%;height:100%;max-width:none!important;object-fit:contain}}`;
  const clean = () => {
    root.remove();
    css.remove();
    urls.forEach((u) => URL.revokeObjectURL(u));
    window.removeEventListener("afterprint", clean);
    if (aufraeumen === clean) aufraeumen = null;
  };
  aufraeumen = clean;
  try {
    const pdf = await loader.promise;
    if (pdf.numPages > 300) throw new Error("Höchstens 300 Druckseiten.");
    for (let n = 1; n <= pdf.numPages; n++) {
      onfortschritt(n, pdf.numPages);
      const page = await pdf.getPage(n),
        size = page.getViewport({ scale: 1 });
      if (
        size.width <= 0 ||
        size.height <= 0 ||
        !Number.isFinite(size.width * size.height)
      )
        throw new Error("Ungültiges Papierformat.");
      // Maximal 6 MP pro Seite, anschließend komprimieren und Canvas freigeben.
      const scale = Math.min(
        2,
        Math.sqrt(6_000_000 / (size.width * size.height)),
      );
      const viewport = page.getViewport({ scale });
      const canvas = document.createElement("canvas");
      canvas.width = Math.ceil(viewport.width);
      canvas.height = Math.ceil(viewport.height);
      await page.render({
        canvasContext: canvas.getContext("2d")!,
        viewport,
        background: "rgb(255,255,255)",
      }).promise;
      const blob = await new Promise<Blob>((resolve, reject) =>
        canvas.toBlob(
          (b) =>
            b
              ? resolve(b)
              : reject(new Error("Druckseite konnte nicht erzeugt werden.")),
          "image/jpeg",
          0.95,
        ),
      );
      canvas.width = 0;
      canvas.height = 0;
      page.cleanup();
      const url = URL.createObjectURL(blob);
      urls.push(url);
      const img = new Image();
      img.src = url;
      img.alt = `Seite ${n}`;
      await img.decode();
      const seite = document.createElement("div");
      seite.className = "druckseite";
      const name = `bauDruck${n}`,
        w = (size.width * 25.4) / 72,
        h = (size.height * 25.4) / 72;
      css.textContent += `@page ${name}{size:${w}mm ${h}mm;margin:0}`;
      seite.style.page = name;
      seite.style.width = `${w}mm`;
      seite.style.height = `${h}mm`;
      seite.append(img);
      root.append(seite);
    }
    document.head.append(css);
    document.body.append(root);
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
    );
    window.addEventListener("afterprint", clean);
    window.print();
  } catch (e) {
    clean();
    throw e;
  } finally {
    await loader.destroy();
  }
}
