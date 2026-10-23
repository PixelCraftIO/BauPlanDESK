import { invoke } from "@tauri-apps/api/core";
import type { Planaufmass, Planmesswert, FachPlan, Planziel, Planaufgabe } from "../generated/plattform";
export const planApi = {
  cadExportieren:(projektId:string,dokument:import("../generated/plattform").Dokumentbezug,revision:number,ziel:string)=>invoke<void>("cadaufmass_exportieren",{projektId,dokument,revision,ziel}),
  cadMesswerte:(aufmass:import("../generated/plattform").CadProjektaufmass)=>invoke<import("../api").Messung[]>("cadaufmass_messwerte",{aufmass}),
  cadLaden:(projektId:string,dokument:import("../generated/plattform").Dokumentbezug)=>invoke<import("../plattformApi").Datensatz<import("../generated/plattform").CadProjektaufmass>|null>("cadaufmass_laden",{projektId,dokument}),
  cadSpeichern:(projektId:string,aufmass:import("../generated/plattform").CadProjektaufmass,basis:number)=>invoke<import("../plattformApi").Datensatz<import("../generated/plattform").CadProjektaufmass>>("cadaufmass_speichern",{projektId,aufmass,basis}),
  exportieren:(projektId:string,wunsch:import("../generated/plattform").Planexportwunsch,ziel:string)=>invoke<void>("planaufmass_exportieren",{projektId,wunsch,ziel}),
  diagnose: (projektId: string, plan: Planaufmass) => invoke<import("../generated/plattform").Planpruefung>("planaufmass_diagnose", {projektId,plan}),
  flaechenoperation: (a:import("../generated/plattform").Plangeometrie,b:import("../generated/plattform").Plangeometrie,operation:import("../generated/plattform").Flaechenoperation) => invoke<import("../generated/plattform").Plangeometrie[]>("planaufmass_flaechenoperation",{a,b,operation}),
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
