import { invoke } from "@tauri-apps/api/core";
import type {
  Fachstand,
  Portalprofil,
  Portalvorgangswunsch,
  Portalabgabewunsch,
  Portalvorbereitung,
  Portalereigniswunsch,
  Portalrueckmeldungswunsch,
  Dokumentbezug,
  Nachrichtenstand,
} from "../generated/plattform";
export const portalApi = {
  profile: () => invoke<Portalprofil[]>("portalprofile"),
  profilHinzufuegen: (profil: Portalprofil) =>
    invoke<Portalprofil[]>("portalprofil_hinzufuegen", { profil }),
  vorgang: (
    projektId: string,
    verfahren: string,
    basis: number,
    wunsch: Portalvorgangswunsch,
  ) =>
    invoke<Fachstand>("portal_vorgang_anlegen", {
      projektId,
      verfahren,
      basis,
      wunsch,
    }),
  pruefen: (
    projektId: string,
    verfahren: string,
    vorgang: string,
    wunsch: Portalabgabewunsch,
  ) =>
    invoke<Portalvorbereitung>("portal_abgabe_pruefen", {
      projektId,
      verfahren,
      vorgang,
      wunsch,
    }),
  festhalten: (
    projektId: string,
    verfahren: string,
    vorgang: string,
    basis: number,
    wunsch: Portalabgabewunsch,
  ) =>
    invoke<Fachstand>("portal_abgabe_festhalten", {
      projektId,
      verfahren,
      vorgang,
      basis,
      wunsch,
    }),
  ereignis: (
    projektId: string,
    verfahren: string,
    vorgang: string,
    abgabe: string,
    basis: number,
    wunsch: Portalereigniswunsch,
  ) =>
    invoke<Fachstand>("portal_ereignis_erfassen", {
      projektId,
      verfahren,
      vorgang,
      abgabe,
      basis,
      wunsch,
    }),
  rueckmeldung: (
    projektId: string,
    verfahren: string,
    vorgang: string,
    basis: number,
    wunsch: Portalrueckmeldungswunsch,
  ) =>
    invoke<Fachstand>("portal_rueckmeldung_erfassen", {
      projektId,
      verfahren,
      vorgang,
      basis,
      wunsch,
    }),
  paket: (
    projektId: string,
    verfahren: string,
    vorgang: string,
    abgabe: string,
    ziel: string,
  ) =>
    invoke<void>("portal_paket_exportieren", {
      projektId,
      verfahren,
      vorgang,
      abgabe,
      ziel,
    }),
  datei: (
    projektId: string,
    verfahren: string,
    vorgang: string,
    abgabe: string,
    dokument: Dokumentbezug,
    ziel: string,
  ) =>
    invoke<void>("portal_datei_exportieren", {
      projektId,
      verfahren,
      vorgang,
      abgabe,
      dokument,
      ziel,
    }),
  oeffnen: (projektId: string, verfahren: string, vorgang: string) =>
    invoke<void>("portal_oeffnen", { projektId, verfahren, vorgang }),
  xbau: (
    projektId: string,
    verfahren: string,
    vorgang: string,
    abgabe: string,
  ) =>
    invoke<Nachrichtenstand>("portal_xbau_archivieren", {
      projektId,
      verfahren,
      vorgang,
      abgabe,
    }),
};
