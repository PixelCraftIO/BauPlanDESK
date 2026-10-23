import { invoke } from "@tauri-apps/api/core";
import type {
  Druckauftrag,
  Druckkatalog,
  Druckposition,
  Druckquelle,
} from "./generated/plattform";
import { nachErfolg } from "./projektEreignisse";
export type { Druckquelle };
export interface Druckvorschau {
  pdf_base64?: string;
  artefakt?: string;
  seiten: number;
  positionen: Druckposition[];
}
export const druckApi = {
  daten: (projektId: string, artefakt: string) =>
    invoke<ArrayBuffer>("druckartefakt_lesen", { projektId, artefakt }),
  entfernen: (projektId: string, artefakt: string) =>
    invoke<void>("druckartefakt_entfernen", { projektId, artefakt }),
  artefaktExportieren: (projektId: string, artefakt: string, ziel: string) =>
    invoke<void>("druckartefakt_exportieren", { projektId, artefakt, ziel }),
  artefaktAblegen: (
    projektId: string,
    artefakt: string,
    name: string,
    vorgang: string,
  ) =>
    nachErfolg(
      invoke<import("./api").DocumentVersion>("druckartefakt_ablegen", {
        projektId,
        artefakt,
        name,
        vorgang,
      }),
      { projektId, bereich: "dokumente" },
    ),
  gestaltung: () =>
    invoke<import("./generated/plattform").Druckgestaltung>("druckgestaltung"),
  gestaltungSpeichern: (
    gestaltung: import("./generated/plattform").Druckgestaltung,
  ) =>
    invoke<import("./generated/plattform").Druckgestaltung>(
      "druckgestaltung_speichern",
      { gestaltung },
    ),
  ablegen: (
    projektId: string,
    name: string,
    pdfBase64: string,
    vorgang: string,
  ) =>
    nachErfolg(
      invoke<import("./api").DocumentVersion>("druckausgabe_ablegen", {
        projektId,
        name,
        pdfBase64,
        vorgang,
      }),
      { projektId, bereich: "dokumente" },
    ),
  katalog: (projektId: string) =>
    invoke<Druckkatalog>("druckkatalog", { projektId }),
  abbrechen: (projektId: string, vorgang: string) =>
    invoke<void>("druckartefakt_abbrechen", { projektId, vorgang }),
  erstellen: (projektId: string, auftrag: Druckauftrag, vorgang: string) =>
    invoke<Druckvorschau>("druckartefakt_erstellen", {
      projektId,
      auftrag,
      vorgang,
    }),
  exportieren: (pdfBase64: string, ziel: string) =>
    invoke<void>("druckausgabe_exportieren", { pdfBase64, ziel }),
};
