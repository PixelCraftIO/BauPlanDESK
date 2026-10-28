/** IFC-Modelle: dünne Aufrufe der Core-Befehle (`apps/desktop/src-tauri/src/ifc.rs`). */
import { invoke } from "@tauri-apps/api/core";
import type { Dokumentbezug, Modellauszug, Modellgeschoss, Modellgrundriss, Modellpruefung, Modelluebernahmeergebnis, Modelluebernahmewunsch } from "../generated/plattform";
import { nachErfolg } from "../projektEreignisse";
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
  /** Prüfung gegen das mitgelieferte IDS oder ein IDS aus den Projektdateien. */
  pruefen: (projektId: string, akteId: string, ids: Dokumentbezug | null = null) =>
    invoke<Modellpruefung>("ifc_pruefen", { projektId, akteId, ids }),
  /** Gebäude, Zeichnungen je Geschoss und Raumbücher aus dem Modell anlegen oder aktualisieren. */
  uebernehmen: (projektId: string, wunsch: Modelluebernahmewunsch) =>
    nachErfolg(invoke<Modelluebernahmeergebnis>("ifc_uebernehmen", { projektId, wunsch }), { projektId, bereich: "fachobjekte" }),
};
