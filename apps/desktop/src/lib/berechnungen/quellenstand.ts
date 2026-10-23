import type { Project, GrundstueckKontext } from "../api";
import type {
  FachPlan,
  Gebaeude,
  Raumbuch,
  NachweisAnsicht,
} from "../generated/plattform";
import type { Datensatz } from "../plattformApi";

/** Nur die tatsächlich ausgewählten Fachquellen betrachten. Rechtliche Grundlagen
 * bleiben konservativ vollständig enthalten; die verbindliche Archivaktualität
 * wird weiterhin ausschließlich vom Core bestimmt. */
export function quellenstand(
  p: Project,
  plan: FachPlan | null,
  g: Datensatz<Gebaeude>[],
  r: Datensatz<Raumbuch>[],
  grund: GrundstueckKontext[],
  n: NachweisAnsicht[],
  planGeneration: number,
  fehler: unknown[],
) {
  const einheit = grund.find((g) => g.id === plan?.grundstueck_id);
  const fach = (plan?.fachquellen ?? []).map((q) => [
    q.art,
    q.objekt,
    (q.art === "raumbuch" ? r : g).find((f) => f.objekt === q.objekt)
      ?.revision ?? null,
  ]);
  return JSON.stringify([
    p.bundesland,
    p.planungsrecht,
    einheit
      ? {
          id: einheit.id,
          schluessel: einheit.schluessel,
          flurstueck_ids: einheit.flurstueck_ids,
          eigentumsflaeche_m2: einheit.eigentumsflaeche_m2,
          abzug_m2: einheit.abzug_m2,
          zuschlag_m2: einheit.zuschlag_m2,
          bezugsflaeche_m2: einheit.bezugsflaeche_m2,
        }
      : null,
    plan?.gebaeude_id
      ? (g.find((g) => g.objekt === plan.gebaeude_id)?.revision ?? null)
      : null,
    fach,
    (plan?.uebernahmen ?? []).map((q) => {
      const a = n.find((n) => n.stand.id === q.nachweis_id);
      return [q.nachweis_id, a?.aktuell ?? null, a?.prueffehler ?? null];
    }),
    plan?.planbindungen?.length ? planGeneration : 0,
    fehler,
  ]);
}
