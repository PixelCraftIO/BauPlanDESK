/** IFC-Modelle: dünne Aufrufe der Core-Befehle (`apps/desktop/src-tauri/src/ifc.rs`). */
import { invoke } from "@tauri-apps/api/core";
import type { Dokumentbezug, Modellauszug, Modellgeschoss, Modellgrundriss, Modellpruefung, Modelluebernahmeergebnis, Modelluebernahmewunsch, Raumvorschlag, Modellabgleich, Reimportwunsch, Fachstand } from "../generated/plattform";
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
  /** Auszug der Modellakte mit angewendeter Nacharbeit. */
  akteAuszug: (projektId: string, akteId: string) => invoke<Modellauszug>("ifc_akte_auszug", { projektId, akteId }),
  akteGrundriss: (projektId: string, akteId: string, geschoss: string, schnitthoeheM: number | null = null) =>
    invoke<Modellgrundriss>("ifc_akte_grundriss", { projektId, akteId, geschoss, schnitthoeheM }),
  raumvorschlaege: (projektId: string, akteId: string, geschoss: string) =>
    invoke<Raumvorschlag[]>("ifc_raumvorschlaege", { projektId, akteId, geschoss }),
  abgleich: (projektId: string, akteId: string, dokument: Dokumentbezug) => invoke<Modellabgleich>("ifc_abgleich", { projektId, akteId, dokument }),
  reimport: (projektId: string, wunsch: Reimportwunsch) =>
    nachErfolg(invoke<Fachstand>("ifc_reimport", { projektId, wunsch }), { projektId, bereich: "fachobjekte" }),
  /** Prüfung gegen das mitgelieferte IDS oder ein IDS aus den Projektdateien. */
  pruefen: (projektId: string, akteId: string, ids: Dokumentbezug | null = null) =>
    invoke<Modellpruefung>("ifc_pruefen", { projektId, akteId, ids }),
  /** Gebäude, Zeichnungen je Geschoss und Raumbücher aus dem Modell anlegen oder aktualisieren. */
  uebernehmen: (projektId: string, wunsch: Modelluebernahmewunsch) =>
    nachErfolg(invoke<Modelluebernahmeergebnis>("ifc_uebernehmen", { projektId, wunsch }), { projektId, bereich: "fachobjekte" }),
};
