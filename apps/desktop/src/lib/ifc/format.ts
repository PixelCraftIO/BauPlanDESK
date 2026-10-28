/** Anzeigeformate des IFC-Modells (deutsch, SI-Einheiten). */
import type { Modellelement } from "../generated/plattform";
export const zahl = (v: number, stellen = 2) => v.toLocaleString("de-DE", { minimumFractionDigits: 0, maximumFractionDigits: stellen });
export const flaeche = (v: number) => `${zahl(v)} m²`;
export const laenge = (v: number) => `${zahl(v)} m`;
// Spiegel von core-lib ifc::klassentitel für die Anzeige im Strukturbaum.
const TITEL: Record<string, string> = {
  IFCWALL: "Wand", IFCSLAB: "Decke/Platte", IFCROOF: "Dach", IFCSPACE: "Raum", IFCCOLUMN: "Stütze", IFCBEAM: "Träger",
  IFCDOOR: "Tür", IFCWINDOW: "Fenster", IFCSTAIR: "Treppe", IFCSTAIRFLIGHT: "Treppenlauf", IFCRAMP: "Rampe", IFCRAMPFLIGHT: "Rampenlauf",
  IFCRAILING: "Geländer", IFCCOVERING: "Bekleidung", IFCCURTAINWALL: "Vorhangfassade", IFCPLATE: "Platte", IFCMEMBER: "Stab",
  IFCFOOTING: "Fundament", IFCPILE: "Pfahl", IFCCHIMNEY: "Schornstein", IFCSHADINGDEVICE: "Sonnenschutz",
  IFCBUILDINGELEMENTPROXY: "Unklassifiziertes Bauteil", IFCFURNISHINGELEMENT: "Möblierung", IFCFURNITURE: "Möblierung",
  IFCFLOWTERMINAL: "TGA-Endgerät", IFCSANITARYTERMINAL: "TGA-Endgerät", IFCFLOWSEGMENT: "TGA-Leitung", IFCPIPESEGMENT: "TGA-Leitung",
  IFCDUCTSEGMENT: "TGA-Leitung", IFCTRANSPORTELEMENT: "Aufzug/Fördertechnik", IFCGEOGRAPHICELEMENT: "Geländeelement",
  IFCCIVILELEMENT: "Ingenieurbauteil", IFCSITE: "Grundstück", IFCBUILDING: "Gebäude", IFCBUILDINGSTOREY: "Geschoss",
};
export function klassentitel(klasse: string): string {
  const k = klasse.replace(/(STANDARDCASE|ELEMENTEDCASE)$/, "");
  return TITEL[k] ?? `Sonstiges (${klasse.replace(/^IFC/, "").toLowerCase()})`;
}
export const elementtitel = (e: Pick<Modellelement, "name" | "langname" | "klasse" | "global_id">) =>
  e.name || e.langname || `${klassentitel(e.klasse)} ${e.global_id.slice(0, 6)}`;
