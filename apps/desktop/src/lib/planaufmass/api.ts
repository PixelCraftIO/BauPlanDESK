import { invoke } from "@tauri-apps/api/core";
import type { Planaufmass, Planmesswert, FachPlan, Planziel, Planaufgabe } from "../generated/plattform";
export const planApi = {
  pruefen: (projektId: string, plan: Planaufmass) =>
    invoke<Planmesswert[]>("planaufmass_pruefen", { projektId, plan }),
  aufgaben: (projektId: string, plan: Planaufmass) =>
    invoke<Planaufgabe[]>("planaufmass_aufgaben", { projektId, plan }),
  ziele: (plan: FachPlan) => invoke<Planziel[]>("planaufmass_ziele", { plan }),
  einsetzen: (projektId: string, plan: FachPlan) => invoke<FachPlan>("planaufmass_einsetzen", { projektId, plan }),
  raumbuch: (projektId: string, aufmassId: string, revision: number, raumbuch: import("../generated/plattform").Raumbuch) =>
    invoke<import("../generated/plattform").Raumbuch>("planaufmass_raumbuch", { projektId, aufmassId, revision, raumbuch }),
  abstandsziele: (plan: import("../generated/plattform").AbstandsPlan) => invoke<Planziel[]>("planaufmass_abstandsziele", { plan }),
  abstand: (projektId: string, plan: import("../generated/plattform").AbstandsPlan) =>
    invoke<import("../generated/plattform").AbstandsPlan>("planaufmass_abstand", { projektId, plan }),
  gebaeudeziele: (plan: import("../generated/plattform").Gebaeude) => invoke<Planziel[]>("planaufmass_gebaeudeziele", { plan }),
  gebaeude: (projektId: string, id: string, plan: import("../generated/plattform").Gebaeude) =>
    invoke<import("../generated/plattform").Gebaeude>("planaufmass_gebaeude", { projektId, id, plan }),
};
