/** Übernahme einer gezeichneten Flurstücksgrenze in die Grundstücksinformationen.
 * Gemeinsamer Weg für das Aufgaben-Panel und den Flurstücksauftrag im Editor:
 * Bindung an den gespeicherten Aufmaßstand setzen, die Fläche vom Core einsetzen
 * lassen (`planaufmass_flurstuecke`) und über die Projektgrenze speichern. */
import { api, type Project } from "../api";
import type { FlurstuecksInfo, Grundstuecksinformationen, Planbindung } from "../generated/plattform";
import { grundstuecksinformationen } from "../grundstuecksdaten";
import { planApi } from "./api";
import { werttext } from "./Uebernahme.svelte";

export const flurText = (f: Pick<FlurstuecksInfo, "flaeche_m2" | "planflaeche_m2" | "flaeche_aus_plan" | "flaeche_offen"> | undefined) =>
  !f ? "—" : f.flaeche_offen ? "noch nicht ermittelt" : f.flaeche_aus_plan ? `${werttext(f.flaeche_m2, "m²")} (aus Plan)` : f.planflaeche_m2 == null ? werttext(f.flaeche_m2, "m²") : `${werttext(f.flaeche_m2, "m²")} Kataster · gezeichnet ${werttext(f.planflaeche_m2, "m²")}`;

/** Entwurf der Übernahme: geprüfte Grundstücksinformationen mit Bindung und Anzeige vorher/nachher. Speichert nichts. */
export async function flurstueckVorbereiten(projekt: Project, flurstueckId: string, bindung: Planbindung, ausPlan = false) {
  const basis = JSON.parse(JSON.stringify(projekt)) as Project;
  const infos = grundstuecksinformationen(basis.planungsrecht);
  const f = infos.flurstuecke.find((x) => x.id === flurstueckId);
  if (!f) throw Error("Flurstück ist nicht mehr gespeichert.");
  const vorher = { ...f };
  f.planbindung = bindung;
  // Ohne amtliche Fläche liefert die Zeichnung die Flurstücksfläche; sonst bleibt sie Kontrolle.
  // `ausPlan`: Das Flurstück wurde aus der Zeichnung angelegt, seine Fläche ist kein Katasterwert.
  if (ausPlan || !(f.flaeche_m2 > 0)) f.flaeche_aus_plan = true;
  const geprueft = await planApi.flurstuecke(projekt.id, infos);
  return { basis, geprueft, vorher: flurText(vorher), nachher: flurText(geprueft.flurstuecke.find((x) => x.id === flurstueckId)) };
}

export function flurstueckeSpeichern(basis: Project, infos: Grundstuecksinformationen) {
  return api.projektBereichSpeichern(basis, "grundstuecksinformationen", {
    id: basis.id, expected_version: basis.version, name: basis.name, bundesland: basis.bundesland, grundstuecksflaeche_m2: basis.grundstuecksflaeche_m2,
    planungsrecht: { ...basis.planungsrecht, grundstuecksinformationen: infos },
  });
}
