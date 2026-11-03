// Darstellung je Arbeitsplatz: Farbschema und Oberflächengröße.
// Beides liegt im localStorage und gilt damit für alle Fenster der Anwendung.
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { listen } from "@tauri-apps/api/event";

export type Stil = "system" | "light" | "dark";
const schluessel = "bauprojekt.darstellung";
const groessenSchluessel = "bauprojekt.oberflaechengroesse";

/** Auswahl in den Einstellungen; Menü und Tastatur ändern in 10-%-Schritten. */
export const GROESSEN = [0.9, 1, 1.1, 1.25, 1.5] as const;
const MIN = 0.5;
const MAX = 2;
const GROESSE_EREIGNIS = "bauprojekt:oberflaechengroesse";

function lesen(k: string): string | null {
  try {
    return localStorage.getItem(k);
  } catch {
    return null;
  }
}
function schreiben(k: string, v: string) {
  try {
    localStorage.setItem(k, v);
  } catch {
    /* Ohne Speicher gilt die Einstellung nur bis zum Neustart. */
  }
}

export function gespeicherterStil(): Stil {
  const v = lesen(schluessel);
  return v === "light" || v === "dark" ? v : "system";
}

function bewegungReduziert(): boolean {
  return matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function anwenden(stil: Stil, sanft = false) {
  const root = document.documentElement;
  const neu =
    stil === "system"
      ? matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : stil;
  // Helligkeitssprung beim Wechsel abfedern; beim Start und bei reduzierter
  // Bewegung wird ohne Übergang umgeschaltet.
  if (sanft && root.dataset.theme && root.dataset.theme !== neu && !bewegungReduziert()) {
    root.classList.add("themenwechsel");
    setTimeout(() => root.classList.remove("themenwechsel"), 260);
  }
  root.dataset.theme = neu;
  root.style.colorScheme = neu;
}
export function speichern(stil: Stil) {
  schreiben(schluessel, stil);
  anwenden(stil, true);
}

export function gespeicherteGroesse(): number {
  const v = Number(lesen(groessenSchluessel));
  return Number.isFinite(v) && v >= MIN && v <= MAX ? v : 1;
}

/** Rundet auf ganze Prozent, damit sich 10-%-Schritte nicht aufsummieren. */
function normieren(f: number): number {
  return Math.round(Math.min(MAX, Math.max(MIN, f)) * 100) / 100;
}

function groesseAnwenden(f: number) {
  // Seitenzoom des Webviews: skaliert Schrift, Abstände und px-Maße gemeinsam,
  // Zeigerkoordinaten im Zeicheneditor bleiben stimmig.
  try {
    void getCurrentWebview()
      .setZoom(f)
      .catch(() => {});
  } catch {
    /* Außerhalb von Tauri (UI-Tests im Browser) ohne Wirkung. */
  }
  document.documentElement.dataset.groesse = String(Math.round(f * 100));
  window.dispatchEvent(new CustomEvent(GROESSE_EREIGNIS, { detail: f }));
}

export function groesseSpeichern(f: number) {
  const n = normieren(f);
  schreiben(groessenSchluessel, String(n));
  groesseAnwenden(n);
}

let letzterSchritt = 0;
/** Menü „Fenster“ und Tastatur. Doppelte Auslösung (Menükürzel und Taste
 * zugleich) wird innerhalb von 150 ms nur einmal gezählt. */
export function groesseSchritt(schritt: "plus" | "minus" | "reset") {
  const jetzt = performance.now();
  if (schritt !== "reset" && jetzt - letzterSchritt < 150) return;
  letzterSchritt = jetzt;
  const f = gespeicherteGroesse();
  groesseSpeichern(schritt === "reset" ? 1 : f + (schritt === "plus" ? 0.1 : -0.1));
}

/** Meldet Änderungen der Oberflächengröße (auch aus anderen Fenstern). */
export function groesseBeobachten(rueckruf: (f: number) => void): () => void {
  const h = (e: Event) => rueckruf((e as CustomEvent<number>).detail);
  window.addEventListener(GROESSE_EREIGNIS, h);
  return () => window.removeEventListener(GROESSE_EREIGNIS, h);
}

let initialisiert = false;
/** Einmal je Fenster; ein zweiter Aufruf (App innerhalb von Start) ist wirkungslos. */
export function initialisieren() {
  if (initialisiert) return () => {};
  initialisiert = true;
  anwenden(gespeicherterStil());
  groesseAnwenden(gespeicherteGroesse());
  const media = matchMedia("(prefers-color-scheme: dark)");
  const update = () => {
    if (gespeicherterStil() === "system") anwenden("system", true);
  };
  media.addEventListener("change", update);
  // Andere Fenster (Editorfenster) übernehmen Änderungen sofort.
  const speicher = (e: StorageEvent) => {
    if (e.key === schluessel) anwenden(gespeicherterStil(), true);
    if (e.key === groessenSchluessel) groesseAnwenden(gespeicherteGroesse());
  };
  window.addEventListener("storage", speicher);
  // Strg/Cmd mit „+“ (deutsche Tastatur) und Ziffernblock; Strg+= / Strg+- /
  // Strg+0 kommen über die Menükürzel.
  const taste = (e: KeyboardEvent) => {
    if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
    if (e.code === "NumpadAdd" || (e.key === "+" && e.code !== "Equal")) {
      e.preventDefault();
      groesseSchritt("plus");
    } else if (e.code === "NumpadSubtract") {
      e.preventDefault();
      groesseSchritt("minus");
    } else if (e.code === "Numpad0") {
      e.preventDefault();
      groesseSchritt("reset");
    }
  };
  window.addEventListener("keydown", taste);
  let abmelden: (() => void) | undefined;
  let beendet = false;
  try {
    listen<string>("oberflaechengroesse", (e) => {
      const s = e.payload;
      if (s === "plus" || s === "minus" || s === "reset") groesseSchritt(s);
    })
      .then((f) => (beendet ? f() : (abmelden = f)))
      .catch(() => {});
  } catch {
    /* Außerhalb von Tauri gibt es kein Menü. */
  }
  return () => {
    beendet = true;
    initialisiert = false;
    media.removeEventListener("change", update);
    window.removeEventListener("storage", speicher);
    window.removeEventListener("keydown", taste);
    abmelden?.();
  };
}
