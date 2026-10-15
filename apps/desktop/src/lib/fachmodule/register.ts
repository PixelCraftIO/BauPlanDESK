import type { Project } from "../api";
import Gebaeude from "./Gebaeude.svelte";
import Aufgaben from "./Aufgaben.svelte";
import Unterlagen from "./Unterlagen.svelte";
import Raumbuch from "./Raumbuch.svelte";
import Verfahrensakte from "../verfahren/Verfahrensakte.svelte";
import AktenSuche from "../verfahren/AktenSuche.svelte";
import Nachweise from "./Nachweise.svelte";
export interface FachProps {
  projekt: Project;
  benutzerId: string;
  darf: boolean;
  darfFreigeben?: boolean;
}
export const FACHMODULE = [
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
