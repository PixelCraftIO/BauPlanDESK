import { leseAbfrage } from "./leseAbfrage";
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
  seite: <T>(projektId: string, art: ObjektArt, nach: string | null = null) => invoke<Datensatz<T>[]>("fachobjektseite", { projektId, art, nach, limit: 100 }),
  lesen: <T>(projektId: string, art: ObjektArt, id: string) => invoke<Datensatz<T> | null>("fachobjekt_lesen", { projektId, art, id }),
  verlaufsseite: <T>(projektId: string, art: ObjektArt, id: string, vor: number | null = null) => invoke<Datensatz<T>[]>("fachverlaufsseite", { projektId, art, id, vor, limit: 50 }),
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
    leseAbfrage("fachobjekte", [projektId,art], () => invoke<Datensatz<T>[]>("fachobjekte", { projektId, art })),
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
  /** Gebäude und Abstandspläne, die auf diese Flurstücke verweisen. */
  flurstuecksverweise: (projektId: string, ids: string[]) =>
    invoke<import("./generated/plattform").Flurstuecksverweis[]>(
      "flurstuecksverweise",
      { projektId, ids },
    ),
  /** Verweise auf nicht mehr erfasste Flurstücke entfernen (nach dem Speichern). */
  flurstuecksverweiseEntfernen: (projektId: string, ids: string[]) =>
    nachErfolg(
      invoke<number>("flurstuecksverweise_entfernen", { projektId, ids }),
      { projektId, bereich: "fachobjekte" },
    ),
  /** Gebäudezuordnungen einer nicht mehr bestehenden Grundstückseinheit übertragen. */
  gebaeudezuordnungUebertragen: (projektId: string, alt: string, neu: string) =>
    nachErfolg(
      invoke<number>("gebaeudezuordnung_uebertragen", { projektId, alt, neu }),
      { projektId, bereich: "fachobjekte" },
    ),
  nachweise: (projektId: string) =>
    leseAbfrage("nachweise",projektId,() => invoke<NachweisAnsicht[]>("nachweise", { projektId })),
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
