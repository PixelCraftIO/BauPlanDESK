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
  Nachrichtenauswertung,
  Uebernahmewunsch,
} from "../generated/plattform";
export const xbau = {
  anlage: (projektId: string, id: string, dateiname: string) =>
    invoke<DocumentVersion>("xbau_anlage_ablegen", {
      projektId,
      id,
      dateiname,
    }),
  typen: () => invoke<Nachrichtentyp[]>("xbau_nachrichtentypen"),
  nachrichten: (projektId: string) =>
    invoke<Nachrichtenstand[]>("xbau_nachrichten", { projektId }),
  pruefen: (projektId: string, original: Dokumentbezug) =>
    invoke<Pruefbericht>("xbau_pruefen", { projektId, original }),
  aufnehmen: (projektId: string, wunsch: Nachrichtenaufnahme) =>
    invoke<Nachrichtenstand>("xbau_aufnehmen", { projektId, wunsch }),
  beleg: (
    projektId: string,
    id: string,
    basis: number,
    beleg: Nachrichtenbeleg,
  ) => invoke<Nachrichtenstand>("xbau_beleg", { projektId, id, basis, beleg }),
  /** Fachlicher Inhalt einer Behördennachricht und Wirkung in der gewählten Akte. */
  auswertung: (projektId: string, id: string, verfahren: string | null) =>
    invoke<Nachrichtenauswertung>("xbau_auswertung", {
      projektId,
      id,
      verfahren,
    }),
  uebernehmen: (projektId: string, id: string, wunsch: Uebernahmewunsch) =>
    invoke<Fachstand>("xbau_auswertung_uebernehmen", {
      projektId,
      id,
      wunsch,
    }),
  verfahren: (
    projektId: string,
    id: string,
    verfahren: string,
    basis: number,
    /** Bestätigtes Zugangs- bzw. Versanddatum; ohne Angabe der Erfassungstag. */
    zugang: string | null = null,
  ) =>
    invoke<Fachstand>("xbau_in_verfahren", {
      projektId,
      id,
      verfahren,
      basis,
      zugang,
    }),
};
