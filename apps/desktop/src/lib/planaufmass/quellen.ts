import { fehlertext } from "../api";
import type { Gebaeude, Planaufmass, Planmarkierung, Planmesswert, Planmessgroesse } from "../generated/plattform";
import { plattformApi, type Datensatz } from "../plattformApi";
import { planApi } from "./api";

/** Gespeichertes Aufmaß mit vom Core ermittelten Messwerten. */
export type Aufmassquelle = { stand: Datensatz<Planaufmass>; werte: Planmesswert[]; fehler: string };

export async function aufmasseLaden(projektId: string): Promise<Aufmassquelle[]> {
  const liste = await plattformApi.liste<Planaufmass>(projektId, "planaufmass");
  return Promise.all(liste.map(async (stand) => {
    try { return { stand, werte: await planApi.pruefen(projektId, stand.daten), fehler: "" }; }
    catch (e) { return { stand, werte: [], fehler: fehlertext(e) }; }
  }));
}

export function messwert(w: Planmesswert, g: Planmessgroesse | undefined) {
  return g === "flaeche" ? w.flaeche_m2 : g === "hoehe" ? w.hoehe_m : g === "horizontale" ? w.horizontale_m : g === "laenge" ? w.laenge_m : g === "neigung" ? w.neigung_grad : null;
}

/**
 * Vorauswahl für Gebäudeflächen. Maßgebend bleibt die Übernahmeprüfung im Core;
 * hier werden nur Markierungen angeboten, die sie bestehen können.
 */
export function gebaeudeflaechen(aufmasse: Aufmassquelle[], gebaeudeId: string, geschossId: string | null) {
  const bedeutung = geschossId ? "geschoss_aussenkante" : "gebaeudegrundflaeche";
  const passend: { a: Aufmassquelle; m: Planmarkierung; wert: number }[] = [];
  const unpassend: { a: Aufmassquelle; m: Planmarkierung; grund: string }[] = [];
  for (const a of aufmasse) for (const m of a.stand.daten.markierungen) {
    if (m.gebaeude_id !== gebaeudeId) continue;
    const wert = a.werte.find((w) => w.id === m.id)?.flaeche_m2 ?? null;
    if (m.bedeutung !== bedeutung) {
      // Nur flächige Markierungen dieses Gebäudes sind als Hinweis hilfreich.
      if (wert !== null && (m.bedeutung === "gebaeudegrundflaeche" || m.bedeutung === "geschoss_aussenkante" || m.bedeutung === "unbestimmt"))
        unpassend.push({ a, m, grund: geschossId ? "ist nicht als „Geschoss – Außenkante“ gekennzeichnet" : "ist nicht als „Gebäudegrundfläche – horizontale Projektion“ gekennzeichnet" });
    } else if (geschossId && m.geschoss_id !== geschossId) {
      unpassend.push({ a, m, grund: m.geschoss_id ? "ist einem anderen Geschoss zugeordnet" : "ist noch keinem Geschoss zugeordnet" });
    } else if (wert === null) {
      unpassend.push({ a, m, grund: "hat noch keine Fläche (Maßstab/Geometrie prüfen)" });
    } else passend.push({ a, m, wert });
  }
  return { passend, unpassend };
}

/** Verknüpft ein Gebäudefeld mit einer Markierung und setzt den Messwert über den Core ein. */
export async function gebaeudefeldVerknuepfen(
  projektId: string, gebaeudeId: string, g: Gebaeude, ziel: string, quelle: { a: Aufmassquelle; m: Planmarkierung } | null,
) {
  const neu = JSON.parse(JSON.stringify(g)) as Gebaeude;
  neu.planbindungen = (neu.planbindungen ?? []).filter((b) => b.ziel !== ziel);
  if (quelle) neu.planbindungen.push({ aufmass_id: quelle.a.stand.objekt, revision: quelle.a.stand.revision, markierung_id: quelle.m.id, groesse: "flaeche", ziel });
  return planApi.gebaeude(projektId, gebaeudeId, neu);
}
