/** IFC-Modelle: dünne Aufrufe der Core-Befehle (`apps/desktop/src-tauri/src/ifc.rs`). */
import { invoke } from "@tauri-apps/api/core";
import type { Dokumentbezug, Modellauszug, Modellgeschoss, Modellgrundriss } from "../generated/plattform";
import type { Plan } from "../api";
export const ifcApi = {
  auszug: (projektId: string, dokument: Dokumentbezug) => invoke<Modellauszug>("ifc_auszug", { projektId, dokument }),
  geschosse: (projektId: string, dokument: Dokumentbezug) => invoke<Modellgeschoss[]>("ifc_geschosse", { projektId, dokument }),
  grundriss: (projektId: string, dokument: Dokumentbezug, geschoss: string, schnitthoeheM: number | null = null) =>
    invoke<Modellgrundriss>("ifc_grundriss", { projektId, dokument, geschoss, schnitthoeheM }),
  /** Grundriss-Schnitt als Plan in Metern (IFC-Hintergrund im Zeicheneditor). */
  grundrissPlan: (projektId: string, dokument: Dokumentbezug, geschoss: string) =>
    invoke<Plan>("ifc_grundriss_plan", { projektId, dokument, geschoss }),
  /** Modelldatei für die 3D-Ansicht (Base64). */
  datei: (projektId: string, dokument: Dokumentbezug) => invoke<string>("ifc_datei", { projektId, dokument }),
};
