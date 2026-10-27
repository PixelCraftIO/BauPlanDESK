import type { Planbedeutung } from "../generated/plattform";
/** Bedeutungen der Markierungen mit Anzeigetext (Editor, Messwerte-Liste). */
export const BEDEUTUNGEN: [Planbedeutung, string][] = [
    ["unbestimmt","Noch nicht zugeordnet"],["flurstueck","Flurstück (Flurkarte / Lageplan)"],["baufenster","Baufenster (Baugrenze / Baulinie)"],["vorhabenumriss","Vorhabenumriss (Gebäude im Lageplan)"],["private_gruenflaeche","Private Grünfläche"],
    ["oeffentliche_gruenflaeche","Öffentliche Grünfläche"],["verkehrsflaeche","Verkehrsfläche"],["ausserhalb_bauland","Fläche außerhalb des Baulands"],["sonstiger_abzug","Sonstige nicht maßgebende Fläche"],
    ["zufahrt","Zufahrt"],["stellplatz","Stellplatz"],["garage","Garage / Carport"],["nebenanlage","Nebenanlage (§ 14 BauNVO)"],["terrasse","Terrasse"],["unterirdisch","Unterirdische Anlage"],
    ["versiegelung","Versiegelte Fläche"],["geschoss_aussenkante","Geschoss – Außenkante"],["gebaeudegrundflaeche","Gebäudegrundfläche – horizontale Projektion"],["raum","Raum"],["aufenthaltsraum","Aufenthaltsraum"],
    ["nuf","DIN 277 – Nutzungsfläche (NUF)"],["tf","DIN 277 – Technikfläche (TF)"],["vf","DIN 277 – Verkehrsfläche (VF)"],
    ["kgf","DIN 277 – Konstruktionsfläche (KGF)"],["aussenwand","Außenwand"],["gelaende","Gelände"],["traufe","Traufe"],["first","First"],["dach","Dach"],
    ["nullpunkt","Gebäudenullpunkt ±0,00"],["fussboden","OK Fertigfußboden"],["decke","OK Decke"],["fundamentsohle","Fundamentsohle"],["hoehenmass","Höhenmaß (senkrechte Strecke)"],
];
export function bedeutungText(b: Planbedeutung | string): string {
  return BEDEUTUNGEN.find(x => x[0] === b)?.[1] ?? String(b);
}
