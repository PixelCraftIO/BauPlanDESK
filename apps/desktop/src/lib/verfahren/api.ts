import { invoke } from "@tauri-apps/api/core";
import type {
  Fachstand,
  Verfahrensakte,
  Vorhabenprofil,
  Anforderungskatalog,
  Abgabewunsch,
  Dokumentbezug,
  Suchtreffer,
  Erinnerung,
  Textquelle,
} from "../generated/plattform";
export const verfahrenApi = {
  profil: () => invoke<Anforderungskatalog>("verfahrensprofil"),
  profilAnwenden: (
    projektId: string,
    id: string,
    basis: number,
    profil: Vorhabenprofil,
    katalog: Anforderungskatalog,
  ) =>
    invoke<Fachstand>("verfahren_profil_anwenden", {
      projektId,
      id,
      basis,
      profil,
      katalog,
    }),
  abgabe: (
    projektId: string,
    id: string,
    basis: number,
    wunsch: Abgabewunsch,
  ) => invoke<Fachstand>("verfahren_abgabe", { projektId, id, basis, wunsch }),
  export: (
    projektId: string,
    id: string,
    abgabe: string,
    format: string,
    ziel: string,
  ) =>
    invoke<void>("verfahren_export", { projektId, id, abgabe, format, ziel }),
  erinnerungen: (projektId: string, heute: string) =>
    invoke<Erinnerung[]>("verfahren_erinnerungen", { projektId, heute }),
  suche: (projektId: string, suche: string) =>
    invoke<Suchtreffer[]>("verfahren_suche", { projektId, suche }),
  index: (
    projektId: string,
    dokument: Dokumentbezug,
    quelle: Textquelle,
    text: string,
  ) =>
    invoke<void>("suchtext_speichern", { projektId, dokument, quelle, text }),
  aktenexport: (projektId: string, ziel: string) =>
    invoke<void>("verfahren_aktenexport", { projektId, ziel }),
  ocr: (pngBase64: string) => invoke<string>("verfahren_ocr", { pngBase64 }),
};
export function heute() {
  const d = new Date();
  return [
    d.getFullYear(),
    String(d.getMonth() + 1).padStart(2, "0"),
    String(d.getDate()).padStart(2, "0"),
  ].join("-");
}
export function leer(): Verfahrensakte {
  return {
    bezeichnung: "",
    art: "bauantrag",
    status: "vorbereitung",
    behoerde: { id: crypto.randomUUID(), name: "", kennung: "", kontakt: "" },
    aktenzeichen: "",
    verantwortlich: null,
    vorgaenger: null,
    profil: null,
    kommunikation: [],
    nachforderungen: [],
    anforderungen: [],
    dokumentbeziehungen: [],
    abgaben: [],
    bescheide: [],
    ereignisse: [],
  };
}
