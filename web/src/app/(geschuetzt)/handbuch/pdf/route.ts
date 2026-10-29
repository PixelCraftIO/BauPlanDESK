import { readFile } from "node:fs/promises";
import path from "node:path";
import { aktuelleSitzung } from "@/lib/auth";
import { HANDBUCH_ORDNER, PDF_DATEI } from "@/lib/handbuch";

export async function GET(request: Request) {
  if (!(await aktuelleSitzung(request.headers))) return new Response("Nicht angemeldet", { status: 401 });
  try {
    const inhalt = await readFile(path.join(/*turbopackIgnore: true*/ HANDBUCH_ORDNER, PDF_DATEI));
    return new Response(inhalt, {
      headers: {
        "Content-Type": "application/pdf",
        "Content-Disposition": `inline; filename="BauPlanIO-${PDF_DATEI}"`,
        "Cache-Control": "private, max-age=3600",
      },
    });
  } catch {
    return new Response("Nicht gefunden", { status: 404 });
  }
}
