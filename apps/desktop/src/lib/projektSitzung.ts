export type BerechnungsBereich =
  | "grundstueck"
  | "grz"
  | "gfz"
  | "geschosse"
  | "volumen";
export type Ansicht =
  | "rechtsgrundlagen"
  | "uebersicht"
  | "festsetzungen"
  | "grundstueck"
  | "grz"
  | "gfz"
  | "geschosse"
  | "volumen"
  | "mass"
  | "abstand"
  | "dokumente"
  | "formulare"
  | "plaene"
  | "bauantrag"
  | "protokoll"
  | "einreichungen";
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
export const reiter: { id: Ansicht; text: string }[] = [
  { id: "uebersicht", text: "Stammdaten" },
  { id: "rechtsgrundlagen", text: "Rechtsgrundlagen" },
  { id: "festsetzungen", text: "Bebauungsplanfestsetzungen" },
  { id: "grundstueck", text: "Baugrundstück" },
  { id: "grz", text: "GRZ" },
  { id: "gfz", text: "GFZ" },
  { id: "geschosse", text: "Geschossigkeit" },
  { id: "abstand", text: "Abstandsflächen" },
  { id: "volumen", text: "Bauvolumen" },
  { id: "formulare", text: "Formulare" },
  { id: "dokumente", text: "Projektdateien" },
  { id: "plaene", text: "Pläne / Aufmaß" },
  { id: "bauantrag", text: "Bauantrag / XBau" },
  { id: "einreichungen", text: "Einreichungen" },
  { id: "protokoll", text: "Chronik" },
];
