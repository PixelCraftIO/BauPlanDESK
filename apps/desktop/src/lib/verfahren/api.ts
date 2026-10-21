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
  Nachrichtenstand,
  Verfahrensstatus,
  Schrittart,
  Verfahrensart,
  Nebenbestimmungsart,
  Fristauftrag,
  Bescheidfristen,
  Ereignisart,
  Berichtigungsgegenstand,
  Aktenereignis,
  Einreichungsstatus,
  Planunterlage,
  Einreichungsweg,
  ExterneEinreichungswunsch,
  Abweichungsart,
  Abweichungsabgleich,
  Verfahrensangaben,
  Verfahrenswahl,
  Unterschriftsbedarf,
  Unterschriftsrolle,
} from "../generated/plattform";
/** Bezeichnungen wie `Unterschriftsrolle::bezeichnung` im Core. */
export const UNTERSCHRIFTSROLLE: Record<Unterschriftsrolle, string> = {
  bauherrschaft: "Bauherrschaft",
  entwurfsverfasser: "Entwurfsverfasser/in",
  fachplaner: "Fachplaner/in",
};
/** Bezeichnungen wie `Abweichungsart::bezeichnung` im Core. */
export const ABWEICHUNGSART: Record<Abweichungsart, string> = {
  abweichung: "Abweichung",
  ausnahme: "Ausnahme",
  befreiung: "Befreiung",
  erleichterung: "Erleichterung",
};
/** Bezeichnungen wie `Einreichungsstatus::bezeichnung` im Core. */
export const EINREICHUNG: Record<Einreichungsstatus, string> = {
  entwurf: "Entwurf",
  eingereicht: "Eingereicht",
  nachgereicht: "Nachgereicht",
  genehmigt: "Genehmigt",
  ersetzt: "Ersetzt",
};
/** Bezeichnungen wie `Verfahrensstatus::bezeichnung` im Core. */
export const STATUS: Record<Verfahrensstatus, string> = {
  vorbereitung: "In Vorbereitung",
  eingereicht: "Eingereicht",
  unvollstaendig: "Unvollständig",
  vollstaendig: "Vollständig",
  in_pruefung: "In behördlicher Prüfung",
  ruhend: "Ruhend",
  entschieden: "Entschieden",
  zurueckgenommen: "Zurückgenommen",
};
/** Bezeichnungen wie `Schrittart::bezeichnung` im Core. */
export const SCHRITTART: Record<Schrittart, string> = {
  anhoerung: "Anhörung",
  zurueckstellung: "Zurückstellung",
  einvernehmen: "Gemeindliches Einvernehmen",
  fachbeteiligung: "Beteiligung der Fachbehörden",
  nachbarbeteiligung: "Nachbarbeteiligung",
};
/** Bezeichnungen wie `Verfahrensart::bezeichnung` im Core. */
export const VERFAHRENSART: Record<Verfahrensart, string> = {
  bauantrag: "Bauantrag",
  vorbescheid: "Vorbescheid",
  aenderungsantrag: "Änderungsantrag",
  teilbaugenehmigung: "Teilbaugenehmigung (eigene Akte)",
  freistellung: "Genehmigungsfreistellung",
  nutzungsaenderung: "Nutzungsänderung",
  beseitigung: "Beseitigung",
  widerspruch: "Widerspruch",
  sonstiges: "Sonstiges",
};
/** Wie `Verfahrensart::baugenehmigung` im Core. */
export const baugenehmigung = (art: Verfahrensart) =>
  ["bauantrag", "aenderungsantrag", "teilbaugenehmigung", "nutzungsaenderung"].includes(art);
/** Bezeichnungen wie `Nebenbestimmungsart::bezeichnung` im Core. */
export const NEBENBESTIMMUNG: Record<Nebenbestimmungsart, string> = {
  auflage: "Auflage",
  bedingung: "Bedingung",
  befristung: "Befristung",
  widerrufsvorbehalt: "Widerrufsvorbehalt",
  auflagenvorbehalt: "Auflagenvorbehalt",
  hinweis: "Hinweis",
};
/** Bezeichnungen wie `chronik::Ereignisart::bezeichnung` im Core. */
export const EREIGNISART: Record<Ereignisart, string> = {
  angelegt: "Verfahren angelegt",
  status: "Bearbeitungsstand",
  vollstaendigkeit: "Vollständigkeit",
  ruecknahme: "Rücknahme",
  kommunikation: "Kommunikation",
  nachforderung: "Nachforderung",
  schritt: "Behördliche Schritte",
  abgabe: "Abgabestand",
  versand: "Übermittlung",
  bescheid: "Bescheid",
  zustellung: "Zustellung",
  nebenbestimmung: "Nebenbestimmung",
  ausfuehrung: "Bauausführung",
  berichtigung: "Berichtigung",
  notiz: "Notiz",
  angaben: "Verfahrensangaben",
  abweichung: "Abweichungen",
  gebuehr: "Gebühren",
};
/** Nebenbestimmungsarten mit erfüllbarer Pflicht (`Nebenbestimmungsart::pflicht`). */
export const PFLICHT: Nebenbestimmungsart[] = ["auflage", "bedingung"];
/**
 * Neues Aktenereignis mit begründeter Berichtigung. Alten und neuen Wert sowie
 * den Text trägt der Core beim Speichern ein und prüft, dass sich der Wert
 * tatsächlich ändert.
 */
export function berichtigungsereignis(
  gegenstand: Berichtigungsgegenstand,
  grund: string,
  benutzerId: string,
): Aktenereignis {
  return {
    id: crypto.randomUUID(),
    datum: heute(),
    erfasst_am: new Date().toISOString(),
    erfasst_von: benutzerId,
    text: `Berichtigung: ${grund}`,
    beleg: null,
    berichtigt: null,
    berichtigung: { gegenstand, grund, vorher: "", nachher: "" },
  };
}
/** Bezeichnungen wie `Einreichungsweg::bezeichnung` im Core. */
export const EINREICHUNGSWEG: Record<Einreichungsweg, string> = {
  post: "Post",
  portal: "Portal",
  persoenlich: "Persönlich",
  email: "E-Mail",
  fax: "Fax",
  sonstiges: "Sonstiger Weg",
};
export const verfahrenApi = {
  profil: () => invoke<Anforderungskatalog>("verfahrensprofil"),
  /** Mitgelieferte Kataloge: Büroprofil und Landespakete. */
  kataloge: () => invoke<Anforderungskatalog[]>("verfahrenskataloge"),
  verfahrenswahl: (
    projektId: string,
    id: string | null,
    angaben: Verfahrensangaben,
  ) =>
    invoke<Verfahrenswahl>("verfahren_verfahrenswahl", {
      projektId,
      id,
      angaben,
    }),
  unterschriften: (projektId: string, id: string) =>
    invoke<Unterschriftsbedarf[]>("verfahren_unterschriften", {
      projektId,
      id,
    }),
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
  externeEinreichung: (
    projektId: string,
    id: string,
    basis: number,
    wunsch: ExterneEinreichungswunsch,
  ) =>
    invoke<Fachstand>("verfahren_externe_einreichung", {
      projektId,
      id,
      basis,
      wunsch,
    }),
  xbauErfassen: (projektId: string, id: string, abgabe: string) =>
    invoke<Nachrichtenstand>("verfahren_xbau_erfassen", {
      projektId,
      id,
      abgabe,
    }),
  bescheidfristen: (projektId: string, auftrag: Fristauftrag) =>
    invoke<Bescheidfristen[]>("verfahren_bescheidfristen", {
      projektId,
      auftrag,
    }),
  /** Abweichungsbedarf aus dem Planungsstand, abgeglichen mit der gespeicherten Akte. */
  abweichungen: (projektId: string, id: string) =>
    invoke<Abweichungsabgleich>("verfahren_abweichungen", { projektId, id }),
  /** Planindex und Einreichungsstatus, im Core aus der gespeicherten Akte abgeleitet. */
  planunterlagen: (projektId: string, id: string) =>
    invoke<Planunterlage[]>("verfahren_planunterlagen", { projektId, id }),
  termineIcs: (projektId: string, ziel: string) =>
    invoke<void>("verfahren_termine_ics", { projektId, ziel }),
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
    antragsangaben: {
      vorbescheidfragen: [],
      als_bauantrag_weiterfuehren: false,
    },
    status: "vorbereitung",
    behoerde: { id: crypto.randomUUID(), name: "", kennung: "", kontakt: "" },
    aktenzeichen: "",
    verantwortlich: null,
    vorgaenger: null,
    profil: null,
    kommunikation: [],
    nachforderungen: [],
    schritte: [],
    anforderungen: [],
    dokumentbeziehungen: [],
    abgaben: [],
    bescheide: [],
    ereignisse: [],
  };
}
