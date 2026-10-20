import Ueberbaubarkeit from "../berechnungen/Ueberbaubarkeit.svelte";
import Stellplaetze from "../berechnungen/Stellplaetze.svelte";
import Wohnflaechen from "../berechnungen/Wohnflaechen.svelte";
import Din277 from "../berechnungen/Din277.svelte";
import Vollgeschosse from "../berechnungen/Vollgeschosse.svelte";
import Hoehen from "../berechnungen/Hoehen.svelte";
import Freiflaechen from "../berechnungen/Freiflaechen.svelte";
import Entwaesserung from "../berechnungen/Entwaesserung.svelte";
import type { Project } from "../api";
import Gebaeude from "./Gebaeude.svelte";
import Aufgaben from "./Aufgaben.svelte";
import Unterlagen from "./Unterlagen.svelte";
import Raumbuch from "./Raumbuch.svelte";
import Verfahrensakte from "../verfahren/Verfahrensakte.svelte";
import AktenSuche from "../verfahren/AktenSuche.svelte";
import Nachweise from "./Nachweise.svelte";
import Planaufmass from "../planaufmass/Planaufmass.svelte";
export interface FachProps {
  projekt: Project;
  benutzerId: string;
  darf: boolean;
  darfFreigeben?: boolean;
}
export const FACHMODULE = [
  {
    id: "planaufmass",
    titel: "PDF-Aufmaß",
    gruppe: "Unterlagen",
    komponente: Planaufmass,
  },
  {
    id: "ueberbaubarkeit",
    titel: "Überbaubare Grundstücksfläche",
    gruppe: "Berechnungen",
    komponente: Ueberbaubarkeit,
  },
  {
    id: "stellplaetze",
    titel: "Stellplatznachweis",
    gruppe: "Berechnungen",
    komponente: Stellplaetze,
  },
  {
    id: "wohnflaechen",
    titel: "Wohnflächenberechnung",
    gruppe: "Berechnungen",
    komponente: Wohnflaechen,
  },
  {
    id: "din277",
    titel: "DIN-277-Flächen / Rauminhalte",
    gruppe: "Berechnungen",
    komponente: Din277,
  },
  {
    id: "vollgeschosse",
    titel: "Vollgeschossprüfung",
    gruppe: "Berechnungen",
    komponente: Vollgeschosse,
  },
  {
    id: "hoehen",
    titel: "Gebäudehöhen / Dachneigung",
    gruppe: "Berechnungen",
    komponente: Hoehen,
  },
  {
    id: "freiflaechen",
    titel: "Versiegelung / Freiflächen",
    gruppe: "Berechnungen",
    komponente: Freiflaechen,
  },
  {
    id: "entwaesserung",
    titel: "Regenwasser / Rückhaltung",
    gruppe: "Berechnungen",
    komponente: Entwaesserung,
  },
  {
    id: "verfahren",
    titel: "Verfahrensakte",
    gruppe: "Organisation",
    komponente: Verfahrensakte,
  },
  {
    id: "aktensuche",
    titel: "Suche / Wiedervorlagen",
    gruppe: "Organisation",
    komponente: AktenSuche,
  },
  {
    id: "gebaeude",
    titel: "Gebäude / Geschosse",
    gruppe: "Grundlagen",
    komponente: Gebaeude,
  },
  {
    id: "raumbuch",
    titel: "Flächen- und Raumbuch",
    gruppe: "Berechnungen",
    komponente: Raumbuch,
  },
  {
    id: "nachweise",
    titel: "Nachweisstände",
    gruppe: "Berechnungen",
    komponente: Nachweise,
  },
  {
    id: "aufgaben",
    titel: "Aufgaben / Fristen",
    gruppe: "Organisation",
    komponente: Aufgaben,
  },
  {
    id: "unterlagen",
    titel: "Unterlagenregister",
    gruppe: "Unterlagen",
    komponente: Unterlagen,
  },
] as const;
