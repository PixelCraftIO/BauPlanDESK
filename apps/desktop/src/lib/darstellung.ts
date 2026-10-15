export type Stil = "system" | "light" | "dark";
const schluessel = "bauprojekt.darstellung";
export function gespeicherterStil(): Stil {
  const v = localStorage.getItem(schluessel);
  return v === "light" || v === "dark" ? v : "system";
}
export function anwenden(stil: Stil) {
  document.documentElement.dataset.theme =
    stil === "system"
      ? matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : stil;
  document.documentElement.style.colorScheme =
    document.documentElement.dataset.theme!;
}
export function speichern(stil: Stil) {
  localStorage.setItem(schluessel, stil);
  anwenden(stil);
}
export function initialisieren() {
  anwenden(gespeicherterStil());
  const media = matchMedia("(prefers-color-scheme: dark)");
  const update = () => {
    if (gespeicherterStil() === "system") anwenden("system");
  };
  media.addEventListener("change", update);
  return () => media.removeEventListener("change", update);
}
