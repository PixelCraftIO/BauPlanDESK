import type { Project } from "../api";
import Gebaeude from "./Gebaeude.svelte";
import Aufgaben from "./Aufgaben.svelte";
import Unterlagen from "./Unterlagen.svelte";
import Raumbuch from "./Raumbuch.svelte";
import Nachweise from "./Nachweise.svelte";
export interface FachProps {
  projekt: Project;
  benutzerId: string;
  darf: boolean;
  darfFreigeben?: boolean;
}
export const FACHMODULE = [
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
