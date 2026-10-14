export type Ansicht =
  | "rechtsgrundlagen"
  | "uebersicht"
  | "mass"
  | "abstand"
  | "dokumente"
  | "plaene"
  | "bauantrag"
  | "protokoll"
  | "einreichungen";
export const reiter: { id: Ansicht; text: string }[] = [
  { id: "uebersicht", text: "Stammblatt" },
  { id: "rechtsgrundlagen", text: "Rechtsgrundlagen" },
  { id: "mass", text: "Grundstücke / Nachweise" },
  { id: "abstand", text: "Abstandsflächen" },
  { id: "dokumente", text: "Dokumente" },
  { id: "plaene", text: "Pläne / Aufmaß" },
  { id: "bauantrag", text: "Bauantrag" },
  { id: "einreichungen", text: "Einreichungen" },
  { id: "protokoll", text: "Verlauf" },
];
