import type { DocumentVersion } from "../api";
import { invoke } from "@tauri-apps/api/core";
import type {
  Dokumentbezug,
  Fachstand,
  Nachrichtentyp,
  Pruefbericht,
  Nachrichtenstand,
  Nachrichtenaufnahme,
  Nachrichtenbeleg,
} from "../generated/plattform";
export const xbau = {
  anlage: (projektId: string, id: string, dateiname: string) =>
    invoke<DocumentVersion>("xbau_anlage_ablegen", {
      projektId,
      id,
      dateiname,
    }),
  nachreichung: (projektId: string, quelle: string) =>
    invoke<Nachrichtenstand>("xbau_nachreichung_vorbereiten", {
      projektId,
      quelle,
    }),
  typen: () => invoke<Nachrichtentyp[]>("xbau_nachrichtentypen"),
  nachrichten: (projektId: string) =>
    invoke<Nachrichtenstand[]>("xbau_nachrichten", { projektId }),
  pruefen: (projektId: string, original: Dokumentbezug) =>
    invoke<Pruefbericht>("xbau_pruefen", { projektId, original }),
  aufnehmen: (projektId: string, wunsch: Nachrichtenaufnahme) =>
    invoke<Nachrichtenstand>("xbau_aufnehmen", { projektId, wunsch }),
  ausgang: (projektId: string) =>
    invoke<Nachrichtenstand>("xbau_ausgang_vorbereiten", { projektId }),
  beleg: (
    projektId: string,
    id: string,
    basis: number,
    beleg: Nachrichtenbeleg,
  ) => invoke<Nachrichtenstand>("xbau_beleg", { projektId, id, basis, beleg }),
  verfahren: (
    projektId: string,
    id: string,
    verfahren: string,
    basis: number,
  ) =>
    invoke<Fachstand>("xbau_in_verfahren", { projektId, id, verfahren, basis }),
};
