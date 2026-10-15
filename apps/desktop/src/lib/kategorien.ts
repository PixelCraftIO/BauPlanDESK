import type { Dokumentkategorie, KategorieStand } from "./api";
export function kategorienOptionen(kategorien: Dokumentkategorie[]) {
  const optionen = kategorien.map((k) => ({
    ...k,
    bezeichnung: k.eltern_id
      ? `${kategorien.find((p) => p.id === k.eltern_id)?.name ?? k.eltern_id} / ${k.name}`
      : k.name,
  }));
  return optionen.sort((a, b) =>
    a.bezeichnung.localeCompare(b.bezeichnung, "de"),
  );
}
export function kategorieText(
  kategorien: Dokumentkategorie[],
  id?: string | null,
) {
  if (!id) return "Ohne Kategorie";
  return (
    kategorienOptionen(kategorien).find((k) => k.id === id)?.bezeichnung ??
    `Unbekannte Kategorie (${id})`
  );
}
export function kategorieStandText(k?: KategorieStand | null) {
  return k
    ? [k.hauptkategorie, k.name].filter(Boolean).join(" / ")
    : "Ohne Kategorie";
}
export function kategoriePasst(
  kategorien: Dokumentkategorie[],
  id: string | null | undefined,
  filter: string,
) {
  return (
    !filter ||
    (filter === ":ohne"
      ? !id
      : id === filter ||
        kategorien.find((k) => k.id === id)?.eltern_id === filter)
  );
}

export function kategorieGeaendert(
  a?: KategorieStand | null,
  b?: KategorieStand | null,
) {
  return (
    a?.id !== b?.id ||
    a?.name !== b?.name ||
    a?.hauptkategorie !== b?.hauptkategorie
  );
}
