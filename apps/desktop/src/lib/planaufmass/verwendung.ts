/**
 * Verwendung der Aufmaß-Markierungen in Berechnungen und Fachobjekten:
 * sammelt alle `Planbindung`en des Projekts (Fachobjekte und Projektstand)
 * und ordnet sie den Markierungen zu. Maßgebend für Werte bleibt der Core;
 * hier geht es nur um die Nachverfolgung „welche Fläche geht wo ein“.
 */
import type { Project } from "../api";
import type { ObjektArt, Planbindung, UebersichtsZiel } from "../generated/plattform";
import { plattformApi } from "../plattformApi";
import { FACHMODULE } from "../fachmodule/register";

export interface Verwendung {
  /** Fachobjekt-Art bzw. Projektbereich (für den Sprung). */
  bereich: string;
  objekt: string | null;
  /** Lesbarer Verbraucher, z. B. „GRZ-Berechnung · Nachweis Haus A“. */
  verbraucher: string;
  bindung: Planbindung;
  /** Sprungziel zum Verbraucher. */
  ziel: UebersichtsZiel;
}

/** Fachobjekt-Arten, die Planbindungen tragen (siehe generated/plattform.ts). */
const ARTEN: ObjektArt[] = [
  "ueberbaubarkeit", "stellplaetze", "wohnflaechen", "din277", "vollgeschosse", "hoehen",
  "freiflaechen", "entwaesserung", "abstandsflaechen", "gebaeude", "raumbuch",
];
const MODULTITEL: Record<string, string> = Object.fromEntries(FACHMODULE.map(m => [m.id, m.titel]));
MODULTITEL.abstandsflaechen = "Abstandsflächen";

function modulFuer(art: ObjektArt): string {
  return art === "abstandsflaechen" ? "abstand" : art;
}

/** Durchsucht den Projektstand (Flurstücke, Abzüge, Bauteile) nach Einzelbindungen. */
function projektBindungen(projekt: Project): Verwendung[] {
  const aus: Verwendung[] = [];
  const gehe = (wert: unknown, pfad: string[]) => {
    if (!wert || typeof wert !== "object") return;
    if (Array.isArray(wert)) { wert.forEach((w, i) => gehe(w, [...pfad, String(i)])); return; }
    const o = wert as Record<string, unknown>;
    const b = o.planbindung as Planbindung | undefined;
    if (b && typeof b === "object" && typeof b.markierung_id === "string") {
      const art = pfad.some(p => p.startsWith("flurstueck")) ? "Flurstück" : pfad.some(p => p.startsWith("abzu")) ? "Abzug" : pfad.some(p => p.startsWith("bauteil")) ? "Bauteil" : "Baugrundstück";
      const name = [o.flurstuecksnummer, o.bezeichnung, o.schluessel, o.nummer, o.name].find(v => typeof v === "string" && v) as string | undefined;
      const bereich = art === "Flurstück" ? "flurstuecke" : "grundstueck";
      aus.push({ bereich, objekt: null, verbraucher: `${art}${name ? ` ${name}` : ""}`, bindung: b, ziel: { bereich, objekt: null, register: null, eintrag: typeof o.id === "string" ? o.id : null } });
    }
    for (const [k, v] of Object.entries(o)) if (k !== "planbindung") gehe(v, [...pfad, k]);
  };
  gehe(projekt.planungsrecht, []);
  return aus;
}

/** Alle Verwendungen des Projekts je Markierungs-ID. */
export async function verwendungenLaden(projekt: Project): Promise<Map<string, Verwendung[]>> {
  const listen = await Promise.allSettled(ARTEN.map(art => plattformApi.liste<{ bezeichnung?: string; planbindungen?: Planbindung[] }>(projekt.id, art)));
  const alle: Verwendung[] = projektBindungen(projekt);
  listen.forEach((r, i) => {
    if (r.status !== "fulfilled") return;
    const art = ARTEN[i], modul = modulFuer(art);
    for (const f of r.value)
      for (const b of f.daten.planbindungen ?? [])
        alle.push({ bereich: modul, objekt: f.objekt, verbraucher: `${MODULTITEL[art] ?? art} · ${f.daten.bezeichnung ?? f.objekt}`, bindung: b, ziel: { bereich: modul, objekt: f.objekt, register: null, eintrag: null } });
  });
  const je = new Map<string, Verwendung[]>();
  for (const v of alle) je.set(v.bindung.markierung_id, [...(je.get(v.bindung.markierung_id) ?? []), v]);
  return je;
}

/** Sprungziel „Im Editor zeigen“: Aufmaß als Tab öffnen und die Markierung wählen. */
export function editorZiel(aufmassId: string, markierungId: string): UebersichtsZiel {
  return { bereich: "planaufmass", objekt: aufmassId, register: "markierung", eintrag: markierungId };
}
