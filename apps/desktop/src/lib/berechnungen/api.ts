import { invoke } from "@tauri-apps/api/core";
import type {
  BerechnungsVorlage,
  FachPlan,
  FachErgebnis,
} from "../generated/plattform";
export const berechnungsApi = {
  ausRaumbuch: (
    projektId: string,
    id: string,
    modul: import("../generated/plattform").BerechnungsArt,
  ) => invoke<FachPlan>("berechnung_aus_raumbuch", { projektId, id, modul }),
  vorlagen: () => invoke<BerechnungsVorlage[]>("berechnungsvorlagen"),
  pruefen: (projektId: string, plan: FachPlan) =>
    invoke<{ plan: FachPlan; ergebnis: FachErgebnis }>(
      "fachberechnung_pruefen",
      { projektId, plan },
    ),
};
