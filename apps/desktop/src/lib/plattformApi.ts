import { invoke } from "@tauri-apps/api/core";
import type {
  Fachstand,
  ObjektArt,
  NachweisAnsicht,
  NachweisEingabe,
  Nachweisstand,
  Freigabe,
  RaumSummen,
  JsonValue,
} from "./generated/plattform";
import { nachErfolg } from "./projektEreignisse";
export type Datensatz<T> = Omit<Fachstand, "daten"> & { daten: T };
export const plattformApi = {
  mitglieder: (projektId: string) =>
    invoke<Datensatz<import("./generated/plattform").ProjektMitglied>[]>("projektmitglieder", { projektId }),
  mitgliedSetzen: (projektId: string, mitglied: import("./generated/plattform").ProjektMitglied, basis: number) =>
    invoke<Datensatz<import("./generated/plattform").ProjektMitglied>>("projektmitglied_setzen", { projektId, mitglied, basis }),
  gebaeudeAuswerten: (projektId: string, id: string, gebaeude: import("./generated/plattform").Gebaeude) =>
    invoke<import("./generated/plattform").GebaeudeGrundlagen>("gebaeude_auswerten", { projektId, id, gebaeude }),
  uebersicht: (projektId: string) =>
    invoke<import("./generated/plattform").Projektuebersicht>(
      "projektuebersicht",
      { projektId },
    ),
  status: (projektId: string) =>
    invoke<import("./generated/plattform").Projektstatus>("projektstatus", {
      projektId,
    }),
  standardCheckliste: () =>
    invoke<import("./generated/plattform").Checkliste>("standard_checkliste"),
  checklisteAnwenden: (
    projektId: string,
    vorlage: import("./generated/plattform").Checkliste,
  ) => invoke<number>("checkliste_anwenden", { projektId, vorlage }),
  verlauf: <T>(projektId: string, art: ObjektArt, id: string) =>
    invoke<Datensatz<T>[]>("fachverlauf", { projektId, art, id }),
  liste: <T>(projektId: string, art: ObjektArt) =>
    invoke<Datensatz<T>[]>("fachobjekte", { projektId, art }),
  speichern: <T>(
    projektId: string,
    art: ObjektArt,
    id: string,
    basis: number,
    daten: T,
  ) =>
    nachErfolg(
      invoke<Datensatz<T>>("fachobjekt_speichern", {
        projektId,
        art,
        id,
        basis,
        daten,
      }),
      { projektId, bereich: "fachobjekte" },
    ),
  nachweise: (projektId: string) =>
    invoke<NachweisAnsicht[]>("nachweise", { projektId }),
  nachweisErstellen: (
    projektId: string,
    id: string,
    bezeichnung: string,
    eingabe: NachweisEingabe,
  ) =>
    nachErfolg(
      invoke<Nachweisstand>("nachweis_erstellen", {
        projektId,
        id,
        bezeichnung,
        eingabe,
      }),
      { projektId, bereich: "nachweise" },
    ),
  freigeben: (projektId: string, id: string, bemerkung: string) =>
    nachErfolg(
      invoke<Freigabe>("nachweis_freigeben", { projektId, id, bemerkung }),
      { projektId, bereich: "nachweise" },
    ),
  wiederholen: (projektId: string, id: string) =>
    nachErfolg(
      invoke<JsonValue>("nachweis_wiederholen", { projektId, id }),
      { projektId, bereich: "nachweise" },
    ),
  raumsummen: (projektId: string, id: string) =>
    invoke<RaumSummen>("raumbuch_summen", { projektId, id }),
};
