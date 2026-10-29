import { readFile } from "node:fs/promises";
import path from "node:path";
import { aktuelleSitzung } from "@/lib/auth";
import { HANDBUCH_ORDNER } from "@/lib/handbuch";

// Bilder des Handbuchs; wie alle Inhalte nur mit Anmeldung.
export async function GET(request: Request, { params }: { params: Promise<{ datei: string }> }) {
  if (!(await aktuelleSitzung(request.headers))) return new Response("Nicht angemeldet", { status: 401 });
  const { datei } = await params;
  if (!/^[a-z0-9-]+\.png$/.test(datei)) return new Response("Nicht gefunden", { status: 404 });
  try {
    const inhalt = await readFile(path.join(/*turbopackIgnore: true*/ HANDBUCH_ORDNER, "bilder", datei));
    return new Response(inhalt, {
      headers: { "Content-Type": "image/png", "Cache-Control": "private, max-age=3600" },
    });
  } catch {
    return new Response("Nicht gefunden", { status: 404 });
  }
}
