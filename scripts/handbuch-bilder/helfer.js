// Hilfsfunktionen für die Handbuchaufnahmen. Werden jedem Befehl vorangestellt.
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const sichtbar = (e) => !!(e.offsetWidth || e.offsetHeight || e.getClientRects().length);
const txt = (e) => (e.innerText || e.value || e.getAttribute("aria-label") || "").replace(/\s+/g, " ").trim();
const KLICKBAR = "button,a,[role=tab],summary,[role=menuitem],[role=option],label";
function finde(t, sel, wurzel) {
  const alle = [...(wurzel || document).querySelectorAll(sel || KLICKBAR)].filter(sichtbar);
  return (
    alle.find((e) => txt(e) === t) ||
    alle.find((e) => e.getAttribute("aria-label") === t || e.title === t) ||
    alle.find((e) => txt(e).startsWith(t)) ||
    alle.find((e) => txt(e).includes(t))
  );
}
async function klick(t, sel, wurzel) {
  const e = typeof t === "string" ? finde(t, sel, wurzel) : t;
  if (!e) throw new Error("nicht gefunden: " + t);
  e.scrollIntoView({ block: "center" });
  e.click();
  await sleep(800);
  return txt(e);
}
function knoepfe(w) {
  return [...(w || document).querySelectorAll("button,[role=tab],summary,a")].filter(sichtbar).map(txt).filter(Boolean);
}
function haupt() {
  return [...document.querySelectorAll("main")].filter(sichtbar).pop() || document.body;
}
/** Hinweisleiste des Beispielprofils ausblenden, damit die Aufnahmen der normalen Arbeit entsprechen. */
function aufraeumen() {
  for (const e of document.querySelectorAll("[role=status]"))
    if (txt(e).startsWith("Sie arbeiten im Beispielprojekt")) e.style.display = "none";
  entmarke();
}
async function arbeitsplatz(t) {
  await klick(t, "header button, nav button, button");
  await sleep(800);
}
async function projekt() {
  const tab = [...document.querySelectorAll("button")].filter(sichtbar).find((b) => txt(b) === "Neubau Einfamilienhaus Mustermann");
  if (!tab) throw new Error("Projekttab fehlt");
  tab.click();
  await sleep(800);
}
async function register(t) {
  await projekt();
  await klick(t, "[role=tab],button");
  await sleep(600);
}
async function bereich(titel) {
  const e = [...document.querySelectorAll(".aktennavigation nav button")]
    .filter(sichtbar)
    .find((b) => txt(b) === titel || txt(b).startsWith(titel));
  if (!e) throw new Error("Bereich fehlt: " + titel);
  e.click();
  await sleep(1500);
}
/** Öffnet in einer Eintragsliste die Zeile, die `zeile` enthält (sonst die erste). */
async function eintrag(zeile) {
  const zeilen = [...haupt().querySelectorAll("tr")].filter(sichtbar);
  const z = zeilen.find((r) => (!zeile || txt(r).includes(zeile)) && finde("Öffnen", "button", r));
  const b = z && finde("Öffnen", "button", z);
  if (!b) throw new Error("Eintrag fehlt: " + zeile);
  b.click();
  await sleep(2500);
}
/** Scrollt die Überschrift bzw. das Element mit dem Text an den oberen Rand. */
async function zeige(t, sel) {
  const e = finde(t, sel || "h1,h2,h3,h4,legend,summary,label,th,strong,p");
  if (!e) throw new Error("nicht gefunden: " + t);
  e.scrollIntoView({ block: "start" });
  await sleep(500);
}
async function aufklappen(t) {
  const s = finde(t, "summary");
  if (!s) throw new Error("nicht gefunden: " + t);
  if (!s.parentElement.open) s.click();
  await sleep(600);
}
function markiere(e, farbe) {
  e = typeof e === "string" ? finde(e) : e;
  if (!e) throw new Error("nicht gefunden");
  e.style.outline = "3px solid " + (farbe || "#d4380d");
  e.style.outlineOffset = "2px";
  e.dataset.hbMarke = "1";
}
function entmarke() {
  document.querySelectorAll("[data-hb-marke]").forEach((e) => {
    e.style.outline = "";
    e.style.outlineOffset = "";
    delete e.dataset.hbMarke;
  });
}
async function taste(key, opts) {
  document.activeElement?.blur?.();
  window.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, ...(opts || {}) }));
  await sleep(500);
}
/** Schließt offene Dialoge der vorigen Szene über „Abbrechen“ bzw. „Schließen“. */
async function dialogeSchliessen() {
  for (const d of document.querySelectorAll("dialog[open]")) {
    const b = finde("Abbrechen", "button", d) || finde("Schließen", "button", d);
    if (b) {
      b.click();
      await sleep(400);
    }
  }
}
/** Klickt einen Knopf im Abschnitt mit der Überschrift `ueberschrift`. */
async function imAbschnitt(ueberschrift, knopf) {
  let p = finde(ueberschrift, "h2,h3,h4,legend,summary");
  while (p && !finde(knopf, "button", p)) p = p.parentElement;
  if (!p) throw new Error("Abschnitt fehlt: " + ueberschrift);
  await klick(finde(knopf, "button", p));
}
/** Öffnet einen Reiter und rückt die Reiterleiste an den oberen Rand. */
async function reiter(t) {
  const e = finde(t, "[role=tab],button");
  if (!e) throw new Error("Reiter fehlt: " + t);
  e.click();
  await sleep(900);
  e.parentElement.scrollIntoView({ block: "start" });
  await sleep(400);
}
/** Wählt in einem Auswahlfeld (aria-label) die Option, deren Text `t` enthält. */
async function waehle(label, t) {
  const s = document.querySelector(`select[aria-label="${label}"]`);
  if (!s) throw new Error("Auswahl fehlt: " + label);
  const o = [...s.options].find((o) => o.text.includes(t));
  if (!o) throw new Error("Option fehlt: " + t);
  s.value = o.value;
  s.dispatchEvent(new Event("change", { bubbles: true }));
  await sleep(800);
}
async function tippe(label, wert) {
  const e = document.querySelector(`input[aria-label="${label}"]`);
  if (!e) throw new Error("Feld fehlt: " + label);
  e.value = wert;
  e.dispatchEvent(new Event("input", { bubbles: true }));
  await sleep(300);
}
/** Öffnet im Aufmaß den Tab des IFC-Modells „Entwurf Architekt“. */
async function ifcModell() {
  await register("Ermittlung");
  await bereich("Aufmaß");
  const t = [...haupt().querySelectorAll("[role=tab]")].filter(sichtbar).find((e) => txt(e) === "Entwurf Architekt");
  if (!t) throw new Error("Modelltab fehlt");
  t.click();
  await sleep(1500);
}
/** Detailreiter der Modellansicht (Eigenschaften, Prüfung, Nacharbeit …). */
async function ifcReiter(t) {
  await klick(t, "[aria-label=Details] [role=tab]");
  await sleep(1500);
}
/** Rollt die rechte Spalte der Modellansicht ans Ende. */
function ifcDetailsEnde() {
  const d = document.querySelector("[aria-label=Details]");
  d.scrollTop = d.scrollHeight;
}
