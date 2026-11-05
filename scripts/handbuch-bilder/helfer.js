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

/**
 * Legt im Beispielprojekt die eigene Zeichnung „Entwurf Erdgeschoss“ (Blatt in
 * Metern) an, falls sie fehlt: Wände mit Fenstern und Türen, Maßketten,
 * Raumstempel aus dem Raumbuch EG, Planzeichen, ein Abbruch-Schuppen und die
 * Geschoss-Außenkante als Markierung. Gibt die Objekt-ID zurück.
 */
async function beispielzeichnung(NAME = "Entwurf Erdgeschoss") {
  const inv = window.__TAURI_INTERNALS__.invoke;
  const p = (await inv("projekte_auflisten")).find((x) => x.name === "Neubau Einfamilienhaus Mustermann");
  const da = (await inv("fachobjekte", { projektId: p.id, art: "planaufmass" })).find((f) => f.daten.bezeichnung === NAME);
  if (da) return da.objekt;
  const rb = (await inv("fachobjekte", { projektId: p.id, art: "raumbuch" })).find((f) => /Erdgeschoss/.test(f.daten.bezeichnung));
  const id = () => crypto.randomUUID();
  const ebene = (bezeichnung, darstellung) => ({ id: id(), bezeichnung, sichtbar: true, gesperrt: false, darstellung });
  const wand = ebene("Wände", { stift_mm: 0.5, schraffur: "mauerwerk", bauzustand: "neubau" });
  const abbruch = ebene("Abbruch", { stift_mm: 0.35, schraffur: "diagonal", bauzustand: "abbruch" });
  const mass = ebene("Bemaßung", { stift_mm: 0.18 });
  const text = ebene("Beschriftung", { stift_mm: 0.25 });
  const frei = ebene("Freianlagen", { stift_mm: 0.25, farbe: "#2f7d32" });
  const aufmass = ebene("Aufmaß", {});
  const W = (von, bis, dicke_m, oeffnungen = [], e = wand) => ({ id: id(), ebene_id: e.id, art: "wand", von, bis, dicke_m, oeffnungen });
  const F = (lage_m, breite_m, hoehe_m = 1.385, bruestung_m = 0.885) => ({ art: "fenster", lage_m, breite_m, hoehe_m, bruestung_m, links: false });
  const T = (lage_m, breite_m, links = false) => ({ art: "tuer", lage_m, breite_m, hoehe_m: 2.135, links });
  const M = (von, bis, abstand) => ({ id: id(), ebene_id: mass.id, art: "masskette", von, bis, abstand });
  const elemente = [
    W([0, 0], [12, 0], 0.365, [F(1.0, 2.01), F(4.3, 1.51), F(8.9, 1.26)]),
    W([12, 0], [12, 9.5], 0.365, [F(1.1, 1.26), F(7.4, 1.26)]),
    W([12, 9.5], [0, 9.5], 0.365, [F(1.6, 1.51), T(5.6, 1.135, true)]),
    W([0, 9.5], [0, 0], 0.365, [F(1.7, 1.01), F(5.4, 2.26, 2.26, 0)]),
    W([7, 0], [7, 9.5], 0.24, [T(4.4, 0.885), T(7.6, 0.885, true)]),
    W([0, 5], [7, 5], 0.24, [T(4.6, 0.885)]),
    W([3.5, 5], [3.5, 9.5], 0.115, [T(1.4, 0.76)]),
    W([7, 3.6], [12, 3.6], 0.115, [T(1.2, 0.885, true)]),
    W([7, 6.9], [12, 6.9], 0.115),
    W([15, 0.5], [18, 0.5], 0.175, [], abbruch),
    W([18, 0.5], [18, 3.5], 0.175, [], abbruch),
    W([18, 3.5], [15, 3.5], 0.175, [], abbruch),
    W([15, 3.5], [15, 0.5], 0.175, [], abbruch),
    M([0, 0], [12, 0], -1.4),
    M([0, 0], [7, 0], -0.8),
    M([7, 0], [12, 0], -0.8),
    M([0, 9.5], [0, 0], -1.4),
    M([0, 9.5], [0, 5], -0.8),
    M([0, 5], [0, 0], -0.8),
    { id: id(), ebene_id: text.id, art: "text", punkt: [-4.5, -2.6], inhalt: "Grundriss Erdgeschoss", hoehe: 0.5, drehung_grad: 0 },
    { id: id(), ebene_id: text.id, art: "text", punkt: [15, 4.4], inhalt: "Schuppen (Abbruch)", hoehe: 0.3, drehung_grad: 0 },
    { id: id(), ebene_id: frei.id, art: "symbol", punkt: [-3, 2], symbol: "baum_bestand", groesse: 3, drehung_grad: 0 },
    { id: id(), ebene_id: frei.id, art: "symbol", punkt: [16.5, 8], symbol: "baum_neu", groesse: 2.5, drehung_grad: 0 },
    { id: id(), ebene_id: frei.id, art: "symbol", punkt: [5.25, 10.6], symbol: "hauseingang", groesse: 1, drehung_grad: 0 },
  ];
  // Raumstempel an den Raummitten, nach Raumname zugeordnet.
  const lage = [
    [/Wohnen/, [1.5, 2.5]],
    [/Küche/, [8.0, 1.8]],
    [/Arbeit/, [8.0, 8.2]],
    [/Diele|Flur/, [3.8, 6.4]],
    [/Technik|Hausanschl|WC|Bad|Dusch/, [0.8, 7.3]],
  ];
  const belegt = new Set();
  for (const r of rb?.daten.raeume ?? []) {
    const l = lage.find(([re], i) => re.test(r.bezeichnung) && !belegt.has(i));
    if (!l) continue;
    belegt.add(lage.indexOf(l));
    elemente.push({ id: id(), ebene_id: text.id, art: "raumstempel", punkt: l[1], raumbuch_id: rb.objekt, raum_id: r.id, hoehe: 0.3 });
  }
  const s = 0.1825;
  const plan = {
    bezeichnung: NAME,
    seite: 1,
    kalibrierung: { art: "einheit", einheit: "meter" },
    hoehenbezug: null,
    ebenen: [wand, abbruch, mass, text, frei, aufmass],
    markierungen: [
      {
        id: id(),
        ebene_id: aufmass.id,
        bezeichnung: "Außenkante EG",
        bedeutung: "geschoss_aussenkante",
        geometrie: { art: "polygon", punkte: [[-s, -s], [12 + s, -s], [12 + s, 9.5 + s], [-s, 9.5 + s]] },
        grundstueck_id: null,
        gebaeude_id: null,
        geschoss_id: null,
      },
    ],
    elemente,
  };
  // Haus mit Abstand zum Blattrand: alle Punkte um (6 m, 6 m) verschieben.
  const v = (q) => [q[0] + 6, q[1] + 6];
  for (const e of elemente) for (const k of ["von", "bis", "punkt"]) if (e[k]) e[k] = v(e[k]);
  for (const m of plan.markierungen) m.geometrie.punkte = m.geometrie.punkte.map(v);
  const neu = id();
  await inv("fachobjekt_speichern", { projektId: p.id, art: "planaufmass", id: neu, basis: 0, daten: plan });
  return neu;
}

/** Öffnet im Aufmaß den Tab bzw. Listeneintrag mit dem Namen `name`. */
async function aufmassTab(name) {
  await register("Ermittlung");
  await bereich("Aufmaß");
  let t = [...haupt().querySelectorAll("[role=tab]")].filter(sichtbar).find((e) => txt(e).startsWith(name));
  if (!t) {
    const liste = [...haupt().querySelectorAll("[role=tab]")].filter(sichtbar).find((e) => txt(e) === "Aufmaßliste");
    if (liste) {
      liste.click();
      await sleep(1200);
    }
    await eintrag(name);
    t = [...haupt().querySelectorAll("[role=tab]")].filter(sichtbar).find((e) => txt(e).startsWith(name));
  }
  if (t) t.click();
  await sleep(1500);
}

/** Zoomt die Zeichenfläche um den Faktor `f` um den Bildschirmpunkt (x, y) wie Strg+Mausrad. */
async function zoomAuf(x, y, f) {
  const ziel = document.elementFromPoint(x, y);
  ziel.dispatchEvent(new WheelEvent("wheel", { deltaY: -Math.log(f) / 0.0015, ctrlKey: true, clientX: x, clientY: y, bubbles: true, cancelable: true }));
  await sleep(1200);
}

/** Zoomt so, dass die Planbeschriftung `t` (z. B. „Grundriss EG“) links oben stehen bleibt. */
async function zoomAufText(t, f) {
  const e = [...document.querySelectorAll("svg text")].filter(sichtbar).find((x) => x.textContent.trim() === t);
  if (!e) throw new Error("Beschriftung fehlt: " + t);
  const r = e.getBoundingClientRect();
  await zoomAuf(r.left - 8, r.top - 8, f);
}

/** Schaltet den Zeicheneditor in den Vollbildmodus, falls er es nicht schon ist. */
async function vollbildAn() {
  if (!finde("Vollbild beenden")) await klick("Vollbild");
  await sleep(1200);
}

/** Wählt in der Auswahl, deren erste Option mit `erste` beginnt, die Option mit Text `t`. */
async function waehleText(erste, t) {
  const s = [...document.querySelectorAll("select")].filter(sichtbar).find((x) => x.options[0]?.text.startsWith(erste));
  if (!s) throw new Error("Auswahl fehlt: " + erste);
  s.value = [...s.options].find((o) => o.text.startsWith(t) || o.text.includes(t)).value;
  s.dispatchEvent(new Event("change", { bubbles: true }));
  await sleep(1200);
}

/** Klickt mit der Maus an Bildschirmpunkt (x, y) auf die Zeichenfläche (wählt die oberste Markierung). */
async function planKlick(x, y) {
  const e = document.elementFromPoint(x, y);
  const o = { clientX: x, clientY: y, bubbles: true, cancelable: true, button: 0, pointerId: 1, pointerType: "mouse", isPrimary: true };
  e.dispatchEvent(new PointerEvent("pointerdown", o));
  await sleep(60);
  e.dispatchEvent(new PointerEvent("pointerup", o));
  e.dispatchEvent(new MouseEvent("click", o));
  await sleep(1000);
}

/** Klappt die Palette des Zeicheneditors mit dem Titel `t` auf und rückt sie nach oben. */
// Paletten der rechten Spalte: Reiter wählen (ohne Auswahl zeigt „Eigenschaften“ das Blatt), dann zum Abschnitt rollen.
const PALETTENREITER = { "Aufgaben": "Aufgaben", "Objekte": "Objekte", "Eigenschaften": "Eigenschaften", "Blatt und Einrichtung": "Eigenschaften", "Prüfen und Ausgabe": "Eigenschaften" };
async function palette(t) {
  const r = [...document.querySelectorAll("[role=tab]")].filter(sichtbar).find((x) => txt(x) === (PALETTENREITER[t] ?? t));
  if (r) r.click();
  await sleep(500);
  if (PALETTENREITER[t] === "Eigenschaften" && t !== "Eigenschaften") { const b = [...document.querySelectorAll("button")].filter(sichtbar).find((x) => txt(x) === "Zum Blatt"); if (b) b.click(); }
  await sleep(500);
  const s = [...document.querySelectorAll("section.palette > .palettenkopf")].filter(sichtbar).find((x) => txt(x).toUpperCase().startsWith(t.toUpperCase()));
  if (!s) throw new Error("Palette fehlt: " + t);
  await sleep(300);
  s.scrollIntoView({ block: "start" });
  await sleep(400);
}
