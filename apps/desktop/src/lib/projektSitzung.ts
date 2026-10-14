import { MODULE, type ModulId } from "./module";
export type BerechnungsBereich =
  | "grundstueck"
  | "grz"
  | "gfz"
  | "geschosse"
  | "volumen";
export type Ansicht = ModulId | "mass";
export const berechnungsAnsichten: Ansicht[] = [
  "grundstueck",
  "grz",
  "gfz",
  "geschosse",
  "volumen",
  "mass",
];
export const berechnungsTitel: Record<BerechnungsBereich, string> = {
  grundstueck: "Baugrundstück",
  grz: "GRZ-Berechnung",
  gfz: "GFZ-Berechnung",
  geschosse: "Geschossigkeit",
  volumen: "Bauvolumen / BMZ",
};
export const reiter = MODULE.map((m) => ({ id: m.id, text: m.titel }));
