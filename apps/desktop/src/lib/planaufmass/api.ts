import { invoke } from "@tauri-apps/api/core";
import type { Planaufmass, Planmesswert, FachPlan, Planziel, Planaufgabe } from "../generated/plattform";
export interface BaugrenzenUebernahme { projekt: import("../api").Project; pruefung: import("../api").BaugrenzenPruefung }
export const planApi = {
  /** Übernimmt gespeicherte CAD-Aufmaße (Altbestand der DXF-Ansicht) als Aufmaße auf der DXF-Unterlage; wiederholbar. */
  cadUebernehmen:(projektId:string)=>invoke<import("../generated/plattform").CadUebernahme>("cadaufmass_uebernehmen",{projektId}),
  /** Ordnet Baufenster und Vorhabenumriss des gespeicherten Aufmaßstands dem Projekt zu und liefert die Lageprüfung. */
  baugrenzen:(projektId:string,aufmass:import("../generated/plattform").Aufmassstand)=>invoke<BaugrenzenUebernahme>("planaufmass_baugrenzen",{projektId,aufmass}),
  exportieren:(projektId:string,wunsch:import("../generated/plattform").Planexportwunsch,ziel:string)=>invoke<void>("planaufmass_exportieren",{projektId,wunsch,ziel}),
  diagnose: (projektId: string, plan: Planaufmass) => invoke<import("../generated/plattform").Planpruefung>("planaufmass_diagnose", {projektId,plan}),
  flaechenoperation: (a:import("../generated/plattform").Plangeometrie,b:import("../generated/plattform").Plangeometrie,operation:import("../generated/plattform").Flaechenoperation) => invoke<import("../generated/plattform").Plangeometrie[]>("planaufmass_flaechenoperation",{a,b,operation}),
  pruefen: (projektId: string, plan: Planaufmass) =>
    invoke<Planmesswert[]>("planaufmass_pruefen", { projektId, plan }),
  aufgaben: (projektId: string, plan: Planaufmass) =>
    invoke<Planaufgabe[]>("planaufmass_aufgaben", { projektId, plan }),
  /** Löst Flurstücksbindungen auf und setzt die gezeichneten Flächen ein (Entwurf; Speichern über die Projektgrenze). */
  flurstuecke: (projektId: string, infos: import("../generated/plattform").Grundstuecksinformationen) =>
    invoke<import("../generated/plattform").Grundstuecksinformationen>("planaufmass_flurstuecke", { projektId, infos }),
  /** Legt aus dem Aufmaßstand (optional) Abzugszeilen je zugeordnetem Flurstück an und löst alle Abzugsbindungen auf (Entwurf; Speichern über Berechnungen → Baugrundstück). */
  abzuege: (projektId: string, aufmass: import("../generated/plattform").Aufmassstand | null, ermittlung: import("../generated/plattform").BaugrundstuecksErmittlung) =>
    invoke<import("../generated/plattform").BaugrundstuecksErmittlung>("planaufmass_abzuege", { projektId, aufmass, ermittlung }),
  /** Legt aus dem Aufmaßstand (optional) GRZ-Bauteile des Baugrundstücks `schluessel` an und löst alle Bauteilbindungen auf (Entwurf). */
  bauteile: (projektId: string, schluessel: string, aufmass: import("../generated/plattform").Aufmassstand | null, aufstellung: import("../api").FlaechenAufstellung) =>
    invoke<import("../api").FlaechenAufstellung>("planaufmass_bauteile", { projektId, schluessel, aufmass, aufstellung }),
  ziele: (plan: FachPlan) => invoke<Planziel[]>("planaufmass_ziele", { plan }),
  einsetzen: (projektId: string, plan: FachPlan) => invoke<FachPlan>("planaufmass_einsetzen", { projektId, plan }),
  raumbuch: (projektId: string, aufmassId: string, revision: number, raumbuch: import("../generated/plattform").Raumbuch) =>
    invoke<import("../generated/plattform").Raumbuch>("planaufmass_raumbuch", { projektId, aufmassId, revision, raumbuch }),
  /** Löst die Planbindungen eines Raumbuchentwurfs auf und setzt die Raumflächen ein (Entwurf; Speichern über die Fachgrenze). */
  raumbuchEinsetzen: (projektId: string, raumbuch: import("../generated/plattform").Raumbuch) =>
    invoke<import("../generated/plattform").Raumbuch>("planaufmass_raumbuch_einsetzen", { projektId, raumbuch }),
  /** Vorlagen der Werkzeugkiste: Werkzeug, Bedeutung und Zeichenebene in einem Schritt. */
  werkzeugkiste: () => invoke<import("../generated/plattform").Werkzeugvorlage[]>("planaufmass_werkzeugkiste"),
  abstandsziele: (plan: import("../generated/plattform").AbstandsPlan) => invoke<Planziel[]>("planaufmass_abstandsziele", { plan }),
  abstand: (projektId: string, plan: import("../generated/plattform").AbstandsPlan) =>
    invoke<import("../generated/plattform").AbstandsPlan>("planaufmass_abstand", { projektId, plan }),
  gebaeudeziele: (plan: import("../generated/plattform").Gebaeude) => invoke<Planziel[]>("planaufmass_gebaeudeziele", { plan }),
  gebaeude: (projektId: string, id: string, plan: import("../generated/plattform").Gebaeude) =>
    invoke<import("../generated/plattform").Gebaeude>("planaufmass_gebaeude", { projektId, id, plan }),
};
