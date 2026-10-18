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
  pdf_base64: string;
  seiten: number;
  positionen: Druckposition[];
}
export const druckApi = {
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
  erstellen: (projektId: string, auftrag: Druckauftrag) =>
    invoke<Druckvorschau>("druckausgabe_erstellen", { projektId, auftrag }),
  exportieren: (pdfBase64: string, ziel: string) =>
    invoke<void>("druckausgabe_exportieren", { pdfBase64, ziel }),
};
