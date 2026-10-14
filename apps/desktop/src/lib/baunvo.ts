import r1962 from "../../../../regelwerke/baunvo/1962.json";
import r1968 from "../../../../regelwerke/baunvo/1968.json";
import r1977 from "../../../../regelwerke/baunvo/1977.json";
import r1990 from "../../../../regelwerke/baunvo/1990.json";
import r2017 from "../../../../regelwerke/baunvo/2017.json";
import type { Anrechnung, Bauteiltyp, BauNvo } from "./api";
export interface Regelwerk {
  id: string; version: string; fassung: BauNvo; grz_ii: boolean;
  bauteile: Record<Bauteiltyp, { bezeichnung: string; anrechnung: Anrechnung }>;
  nichtvollgeschoss_faktor: number; nichtvollgeschoss_planabweichung: boolean;
  gemeinschaftsanteile: boolean; gemeinschaftsanteile_rechtsgrundlage: string;
}
export const REGELWERKE = [r1962, r1968, r1977, r1990, r2017] as Regelwerk[];
export const regelwerk = (fassung: BauNvo): Regelwerk => REGELWERKE.find(r => r.fassung === fassung)!;
