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
    ["abstandsflaeche","Abstandsfläche (Linie an der Außenwand im Lageplan oder in der Ansicht)"],
    ["attika","Attika (Oberkante, Ansicht)"],["wandteilgrenze","Wandteilgrenze (senkrechte Linie, Ansicht)"],["gaube","Gaube / Dachaufbau (Breite, Ansicht)"],["abgrabung","Abgrabung (Gelände, nicht maßgebend)"],
];
export function bedeutungText(b: Planbedeutung | string): string {
  return BEDEUTUNGEN.find(x => x[0] === b)?.[1] ?? String(b);
}

const LAGEPLAN: Planbedeutung[] = ["flurstueck","baufenster","vorhabenumriss","private_gruenflaeche","oeffentliche_gruenflaeche","verkehrsflaeche","ausserhalb_bauland","sonstiger_abzug","zufahrt","stellplatz","garage","nebenanlage","terrasse","unterirdisch","versiegelung","abstandsflaeche"];
const GEBAEUDE: Planbedeutung[] = ["gebaeudegrundflaeche","geschoss_aussenkante","aussenwand"];
const RAEUME: Planbedeutung[] = ["raum","aufenthaltsraum","nuf","tf","vf","kgf"];
/** Nur als Fläche sinnvoll. */
const NUR_FLAECHE: Planbedeutung[] = [...LAGEPLAN.filter(b => b !== "abstandsflaeche"), "gebaeudegrundflaeche", "geschoss_aussenkante", ...RAEUME];
/** Nur als Linie oder Höhenpunkt sinnvoll. */
const NUR_LINIE: Planbedeutung[] = ["gelaende","traufe","first","nullpunkt","fussboden","decke","fundamentsohle","hoehenmass","abstandsflaeche","attika","wandteilgrenze","gaube","abgrabung"];
/**
 * Bedeutungen in Gruppen, beschränkt auf das, was zur Geometrie passt: Eine
 * Fläche bekommt keine Höhenbedeutungen, eine Linie keine Flächenbedeutungen.
 * Die gesetzte Bedeutung bleibt immer wählbar.
 */
export function bedeutungsgruppen(geometrie: string, aktuell: Planbedeutung): [string, [Planbedeutung, string][]][] {
  const flaeche = ["polygon", "rechteck", "kreis", "flaeche"].includes(geometrie);
  const erlaubt = (b: Planbedeutung) => b === aktuell || (flaeche ? !NUR_LINIE.includes(b) : !NUR_FLAECHE.includes(b));
  const gruppe = (b: Planbedeutung) => LAGEPLAN.includes(b) ? "Lageplan" : GEBAEUDE.includes(b) ? "Gebäude" : RAEUME.includes(b) ? "Räume und Flächen nach DIN 277" : "Höhen (Schnitt / Ansicht)";
  const gruppen = new Map<string, [Planbedeutung, string][]>();
  for (const e of BEDEUTUNGEN) {
    if (e[0] === "unbestimmt" || !erlaubt(e[0])) continue;
    const g = gruppe(e[0]);
    gruppen.set(g, [...(gruppen.get(g) ?? []), e]);
  }
  return [...gruppen.entries()];
}
/** Bedeutungen, die einem Baugrundstück zugeordnet werden. */
export const MIT_GRUNDSTUECK: Planbedeutung[] = ["unbestimmt","baufenster","vorhabenumriss","private_gruenflaeche","zufahrt","stellplatz","garage","nebenanlage","terrasse","unterirdisch","versiegelung","abstandsflaeche","gebaeudegrundflaeche"];
/** Bedeutungen ohne Gebäudebezug: Flurstück und Abzugsflächen des Flurstücks. */
export const OHNE_GEBAEUDE: Planbedeutung[] = ["flurstueck","oeffentliche_gruenflaeche","verkehrsflaeche","ausserhalb_bauland","sonstiger_abzug","baufenster","private_gruenflaeche"];
