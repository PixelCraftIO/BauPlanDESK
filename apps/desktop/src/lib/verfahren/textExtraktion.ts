/** Lesender PDF-/Bildadapter für den ableitbaren Suchindex. */
import { api } from "../api";
import type { DocumentVersion } from "../api";
import { verfahrenApi } from "./api";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
export async function indexieren(
  projektId: string,
  d: DocumentVersion,
  ocr: boolean,
  fortschritt: (s: string) => void,
) {
  if (/\.(eml|msg)$/i.test(d.file_name)) {
    const m = await api.emailLaden(d.document_id, d.version);
    await verfahrenApi.index(
      projektId,
      { dokument_id: d.document_id, version: d.version },
      "email",
      [
        m.inhalt.betreff,
        ...m.inhalt.absender.map((a) => a.adresse),
        m.inhalt.text,
      ].join("\n"),
    );
    return;
  }
  if (!/\.pdf$/i.test(d.file_name))
    throw Error(
      "PDF und archivierte E-Mails können indexiert werden. Fotos für OCR zunächst als PDF im Projekt ablegen.",
    );
  const { getDocument, GlobalWorkerOptions } = await import("pdfjs-dist");
  GlobalWorkerOptions.workerSrc = workerUrl;
  const quelle = await api.pdfVorschau(d.document_id, d.version);
  const loader = getDocument({
    data: Uint8Array.from(atob(quelle.pdf_base64), (c) => c.charCodeAt(0)),
    isEvalSupported: false,
    cMapUrl: new URL("./pdfjs/cmaps/", document.baseURI).href,
    cMapPacked: true,
    standardFontDataUrl: new URL("./pdfjs/standard_fonts/", document.baseURI)
      .href,
  });
  try {
    const pdf = await loader.promise;
    if (pdf.numPages > (ocr ? 50 : 300))
      throw Error(
        ocr
          ? "OCR höchstens 50 Seiten je Datei"
          : "Indexierung höchstens 300 Seiten je Datei",
      );
    let text = "";
    for (let n = 1; n <= pdf.numPages; n++) {
      fortschritt(`Seite ${n} von ${pdf.numPages}`);
      const page = await pdf.getPage(n);
      let teil = "";
      if (ocr) {
        const v = page.getViewport({ scale: 1 }),
          scale = Math.min(2.5, Math.sqrt(10_000_000 / (v.width * v.height)));
        const viewport = page.getViewport({ scale }),
          c = document.createElement("canvas");
        c.width = Math.ceil(viewport.width);
        c.height = Math.ceil(viewport.height);
        await page.render({
          canvasContext: c.getContext("2d")!,
          viewport,
          background: "white",
        }).promise;
        teil = await verfahrenApi.ocr(c.toDataURL("image/png").split(",")[1]);
        c.width = 0;
        c.height = 0;
      } else {
        const t = await page.getTextContent();
        teil = t.items.map((i) => ("str" in i ? i.str : "")).join(" ");
      }
      text += `\n--- Seite ${n} ---\n${teil}`;
      if (text.length > 1_000_000)
        throw Error("Suchtext zu umfangreich. Datei aufteilen.");
      page.cleanup();
    }
    if (!text.replace(/--- Seite \d+ ---/g, "").trim())
      throw Error("Kein Text gefunden. Bei Scans OCR verwenden.");
    await verfahrenApi.index(
      projektId,
      { dokument_id: d.document_id, version: d.version },
      ocr ? "ocr" : "pdf",
      text,
    );
  } finally {
    await loader.destroy();
  }
}
