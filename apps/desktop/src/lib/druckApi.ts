import { invoke } from "@tauri-apps/api/core";
import type {
  Druckauftrag,
  Druckkatalog,
  Druckposition,
  Druckquelle,
} from "./generated/plattform";
export type { Druckquelle };
export interface Druckvorschau {
  pdf_base64: string;
  seiten: number;
  positionen: Druckposition[];
}
export const druckApi = {
  katalog: (projektId: string) =>
    invoke<Druckkatalog>("druckkatalog", { projektId }),
  erstellen: (projektId: string, auftrag: Druckauftrag) =>
    invoke<Druckvorschau>("druckausgabe_erstellen", { projektId, auftrag }),
  exportieren: (pdfBase64: string, ziel: string) =>
    invoke<void>("druckausgabe_exportieren", { pdfBase64, ziel }),
};
