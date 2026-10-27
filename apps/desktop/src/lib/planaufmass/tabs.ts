/** Zeichnungs-Tabs des Aufmaßes: offene Aufmaße eines Projekts je Benutzer, über Sitzungen hinweg gemerkt. */
export type Tabstart = { id?: string; dokument?: { id: string; version: number; name: string } };
export interface Zeichnungstab {
  /** Eindeutiger Schlüssel des Tabs; zugleich Entwurfsobjekt der lokalen Entwurfssicherung. */
  key: string;
  start: Tabstart;
  titel: string;
  geaendert: boolean;
}
const schluessel = (benutzerId: string, projektId: string) => `bauprojekt.aufmasstabs.v1.${benutzerId}.${projektId}`;
export function tabsLesen(benutzerId: string, projektId: string): { tabs: Zeichnungstab[]; aktiv: string } {
  try {
    const v = JSON.parse(localStorage.getItem(schluessel(benutzerId, projektId)) ?? "null");
    if (v && Array.isArray(v.tabs))
      return {
        tabs: v.tabs
          .filter((t: Zeichnungstab) => typeof t?.key === "string" && t.start && typeof t.start === "object")
          .map((t: Zeichnungstab) => ({ key: t.key, start: t.start, titel: typeof t.titel === "string" ? t.titel : "", geaendert: false })),
        aktiv: typeof v.aktiv === "string" ? v.aktiv : "liste",
      };
  } catch {}
  return { tabs: [], aktiv: "liste" };
}
export function tabsMerken(benutzerId: string, projektId: string, tabs: Zeichnungstab[], aktiv: string) {
  try {
    localStorage.setItem(schluessel(benutzerId, projektId), JSON.stringify({ tabs: tabs.map(t => ({ key: t.key, start: t.start, titel: t.titel })), aktiv }));
  } catch {}
}
