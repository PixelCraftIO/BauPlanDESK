import type { Bauteiltyp, Planbedeutung } from "../generated/plattform";

/**
 * Bedeutungen, die Abzugsflächen der Baugrundstücksermittlung liefern
 * (Anzeige und Vorauswahl; maßgebend ist `Planbedeutung::abzugsart` im Core).
 */
export const ABZUGSBEDEUTUNGEN: Planbedeutung[] = [
  "private_gruenflaeche",
  "oeffentliche_gruenflaeche",
  "verkehrsflaeche",
  "ausserhalb_bauland",
  "sonstiger_abzug",
];
/** Bedeutungen, die GRZ-Bauteile ohne Gebäude liefern (`Planbedeutung::bauteiltyp`). */
export const BAUTEILBEDEUTUNGEN: Planbedeutung[] = [
  "zufahrt",
  "stellplatz",
  "garage",
  "nebenanlage",
  "terrasse",
  "unterirdisch",
];
/**
 * Bedeutung, die ein Bauteiltyp im Aufmaß erwartet (`Planbedeutung::fuer_bauteiltyp`).
 * Nur zur Einschränkung der Typauswahl gebundener Bauteile; die Prüfung macht der Core.
 */
export const BAUTEIL_BEDEUTUNG: Record<Bauteiltyp, Planbedeutung | null> = {
  hauptanlage: null,
  balkon_loggia: null,
  zufahrt: "zufahrt",
  stellplatz: "stellplatz",
  garage: "garage",
  garage_abstandsflaeche: "garage",
  nebenanlage14: "nebenanlage",
  terrasse_hauptanlage: "terrasse",
  terrasse_nebenanlage: "terrasse",
  unterirdisch: "unterirdisch",
};
