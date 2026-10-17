import regeln from "./feldDarstellung.json" with { type: "json" };
import type { Feld } from "../generated/plattform";
export function mehrzeilig(f: Feld) {
  return (
    f.typ === "textarea" ||
    regeln.mehrzeilig.some((k) => f.name.split(".").at(-1)?.includes(k))
  );
}
export function feldSichtbar(
  modul: string,
  pfad: string,
  feld: Feld,
  row: Record<string, unknown>,
) {
  const regel = (regeln.aufmass as Record<string, Record<string, string[]>>)[
    `${modul}.${pfad}`
  ]?.[feld.name];
  // Unbekannte/alte Aufmaßformen nie durch versteckte Eingaben kaschieren.
  return (
    !regel ||
    !["aufmass", "rechteck", "dreieck", "trapez"].includes(String(row.form)) ||
    regel.includes(String(row.form))
  );
}
export function feldFehler(f: Feld, wert: unknown) {
  if (
    f.typ === "number" &&
    (typeof wert !== "number" || !Number.isFinite(wert))
  )
    return "Bitte eine Zahl eingeben. 0 ist eine eigene Angabe und kein Leerwert.";
  if (
    f.typ === "optional_number" &&
    wert !== null &&
    wert !== undefined &&
    (typeof wert !== "number" || !Number.isFinite(wert))
  )
    return "Bitte eine gültige Zahl eingeben oder das Feld leer lassen.";
  return "";
}
