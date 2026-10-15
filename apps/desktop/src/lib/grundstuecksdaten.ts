// Datenadapter für Bestandsprojekte. Gruppierung und Flächenberechnung erfolgen im Core.
import type { Planungsrecht, BaugrundstuecksPlan } from "./api";
import type {
  Grundstuecksinformationen,
  BaugrundstuecksErmittlung,
} from "./generated/plattform";
export function grundstuecksinformationen(
  p: Planungsrecht,
): Grundstuecksinformationen {
  return structuredClone(
    p.grundstuecksinformationen ?? {
      flurstuecke: (p.baugrundstuecke?.flurstuecke ?? []).map((f) => ({
        id: f.id,
        gemarkung: f.gemarkung,
        flur: f.flur,
        nummer: f.nummer,
        flaeche_m2: f.flaeche_m2,
        grundbuchbezirk: f.grundbuchbezirk,
        grundbuchblatt: f.grundbuchblatt,
        laufende_nummer: f.laufende_nummer || null,
        grundbuchauszug: null,
        eigentuemer: "",
        bemerkung: "",
      })),
    },
  );
}
export function berechnungsplan(p: Planungsrecht): BaugrundstuecksPlan {
  const infos = grundstuecksinformationen(p);
  const e = p.baugrundstuecksermittlung;
  return {
    flurstuecke: infos.flurstuecke.map((f) => {
      const a = e
        ? e.ansaetze.find((a) => a.flurstueck_id === f.id)
        : p.baugrundstuecke?.flurstuecke.find((a) => a.id === f.id);
      return {
        ...f,
        laufende_nummer: f.laufende_nummer ?? 0,
        abzuege: a?.abzuege ?? [],
        abweichende_zuordnung: a?.abweichende_zuordnung ?? null,
      };
    }),
    berechnungen: e?.berechnungen ?? p.baugrundstuecke?.berechnungen ?? [],
  };
}
export function berechnungsansaetze(
  p: BaugrundstuecksPlan,
): BaugrundstuecksErmittlung {
  return {
    ansaetze: p.flurstuecke.map((f) => ({
      flurstueck_id: f.id,
      abzuege: f.abzuege,
      abweichende_zuordnung: f.abweichende_zuordnung,
    })),
    berechnungen: p.berechnungen,
    quellen_hash: null,
  };
}
