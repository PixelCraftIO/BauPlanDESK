<script lang="ts">
  import Statusanzeige from "../arbeitsplatz/Statusanzeige.svelte";
  import { ziel as navigationsziel } from "../arbeitsplatz/arbeitscheck";
  import { feldSichtbar, feldFehler } from "./feldDarstellung";
  import Planquellen from "../planaufmass/Planquellen.svelte";
  import { onMount, untrack, tick } from "svelte";
  import {
    api,
    fehlertext,
    type Project,
    type DocumentVersion,
    type GrundstueckKontext,
  } from "../api";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  import Fachfeld from "./Fachfeld.svelte";
  import Druckknopf from "../Druckknopf.svelte";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import { berechnungsApi } from "./api";
  import { abonnieren } from "../projektEreignisse";
  import { projektNavigation } from "../projektNavigation.svelte";
  import type {
    BerechnungsArt,
    BerechnungsVorlage,
    FachPlan,
    FachErgebnis,
    Feld,
    FormTabelle,
    Auswahl,
    Gebaeude,
    NachweisAnsicht,
    Uebernahme,
  } from "../generated/plattform";
  let {
    projekt,
    benutzerId,
    darf,
    darfFreigeben = false,
    modul,
  }: {
    projekt: Project;
    benutzerId: string;
    darf: boolean;
    darfFreigeben?: boolean;
    modul: BerechnungsArt;
  } = $props();
  let vorlage = $state<BerechnungsVorlage | null>(null),
    grund = $state<GrundstueckKontext[]>([]),
    gebaeude = $state<Datensatz<Gebaeude>[]>([]),
    dateien = $state<DocumentVersion[]>([]),
    nachweise = $state<NachweisAnsicht[]>([]),
    raumbuecher = $state<
      Datensatz<import("../generated/plattform").Raumbuch>[]
    >([]);
  let bereich = $state("daten"),
    planQuellenStand = $state(0),
    result = $state<FachErgebnis | null>(null),
    resultSignatur = $state(""),
    busy = $state(false),
    importText = $state(""),
    importOffen = $state(false),
    quellId = $state("");
  // Sprung aus der Projektübersicht: angefragtes Register öffnen (z. B. Quellen).
  const navigation = projektNavigation();
  $effect(() => {
    const a = navigation?.auftrag;
    const r = a?.ziel.register;
    if (a?.ziel.bereich === modul && r && ["daten", "regeln", "quellen", "bericht"].includes(r))
      untrack(() => { bereich = r; });
  });
  const s = untrack(
    () =>
      new FachSitzung<FachPlan>(
        projekt.id,
        modul,
        benutzerId,
        () => {
          const plan: FachPlan = JSON.parse(JSON.stringify(vorlage!.plan));
          // Eindeutige Grundlagen vorbelegen, statt sie erneut abzufragen.
          if (plan.grundstueck_id === null && grund.length === 1)
            plan.grundstueck_id = grund[0].id;
          if (plan.gebaeude_id === null && gebaeude.length === 1 && ["vollgeschosse", "hoehen"].includes(modul)) {
            plan.gebaeude_id = gebaeude[0].objekt;
            plan.gebaeudedaten_uebernehmen = true;
          }
          return plan;
        },
        (p) => p.bezeichnung,
      ),
  );
  /**
   * Stand der verknüpften Quellen: Projektversion (Baugrundstücke) und
   * Revisionen der Gebäude und Raumbücher. Ändert er sich, gilt der Prüfbericht
   * als veraltet, auch wenn die eigenen Eingaben gleich geblieben sind.
   */
  const quellenStand = $derived(
    JSON.stringify([
      projekt.version,
      gebaeude.map((g) => [g.objekt, g.revision]),
      raumbuecher.map((r) => [r.objekt, r.revision]),
      planQuellenStand,
    ]),
  );
  const signatur = (plan: unknown) =>
    JSON.stringify(plan) + "|" + quellenStand;
  const aktuelleVorschau = $derived(
    !!result && !!s.eingabe && signatur(s.eingabe) === resultSignatur,
  );
  const eingabenGleich = $derived(
    !!result &&
      !!s.eingabe &&
      resultSignatur.startsWith(JSON.stringify(s.eingabe) + "|"),
  );
  const z = (n: number | null) =>
    n === null
      ? "Nicht ermittelt"
      : n.toLocaleString("de-DE", { maximumFractionDigits: 4 });
  type Obj = Record<string, unknown>;
  function lesen(v: unknown, path: string): unknown {
    return path
      .split(".")
      .reduce<unknown>(
        (a, k) => (a && typeof a === "object" ? (a as Obj)[k] : undefined),
        v,
      );
  }
  function schreiben(v: unknown, path: string, value: unknown) {
    const keys = path.split(".");
    let p = v as Obj;
    for (const k of keys.slice(0, -1)) p = p[k] as Obj;
    p[keys.at(-1)!] = value;
  }
  function zeilen(t: FormTabelle): Obj[] {
    const a = lesen(s.eingabe?.eingaben, t.pfad);
    return Array.isArray(a) ? (a as Obj[]) : [];
  }
  function name(row: Obj) {
    return String(
      row.bezeichnung ??
        row.raum ??
        row.nutzung ??
        row.typ ??
        row.art ??
        "Position",
    );
  }
  function optionen(f: Feld): Auswahl[] {
    if (!f.referenz) return f.optionen;
    if (f.referenz === "$gebaeudegeschosse") {
      const g = gebaeude.find((g) => g.objekt === s.eingabe?.gebaeude_id);
      if (!g) return [];
      return g.daten.geschosse.length ? g.daten.geschosse.map((s) => ({wert:s.id,titel:s.bezeichnung})) : [{wert:g.objekt,titel:"Einzelgeschoss (aus Gebäudegrundfläche)"}];
    }
    const [path, wertfeld] = f.referenz.split(":");
    const a = lesen(s.eingabe?.eingaben, path);
    if (!Array.isArray(a)) return [];
    const vals = new Map<string, string>();
    for (const row of a as Obj[]) {
      const v = String(row[wertfeld ?? "id"] ?? "");
      if (v) vals.set(v, wertfeld === "nutzung" ? v : name(row));
    }
    return [...vals].map(([wert, titel]) => ({ wert, titel }));
  }
  let bereichElement = $state<HTMLDivElement>();
  let pruefversuch = $state(false),
    alleFelder = $state(false);
  let eingabefehler = $state<{ titel: string; gruppe: string; pfad: string }[]>(
    [],
  );
  $effect(() => {
    void s.id;
    untrack(() => {
      eingabefehler = [];
      pruefversuch = false;
      // Der Prüfbericht gehört zum bisherigen Eintrag.
      result = null;
      resultSignatur = "";
    });
  });
  function formatPruefen() {
    if (!s.eingabe || !vorlage) return true;
    pruefversuch = true;
    const fehler: { titel: string; gruppe: string; pfad: string }[] = [];
    if (!s.eingabe.bezeichnung.trim()) fehler.push({titel:'Bezeichnung',gruppe:'daten',pfad:'$bezeichnung'});
    for (const f of vorlage.felder)
      if (feldFehler(f, lesen(s.eingabe.eingaben, f.name)))
        fehler.push({ titel: f.titel, gruppe: f.gruppe, pfad: f.name });
    for (const t of vorlage.tabellen)
      for (const [i, row] of zeilen(t).entries())
        for (const f of t.felder)
          if (feldFehler(f, row[f.name]))
            fehler.push({
              titel: `${t.titel}, Position ${i + 1}: ${f.titel}`,
              gruppe: t.gruppe,
              pfad: `${t.pfad}[${i}].${f.name}`,
            });
    eingabefehler = fehler;
    if (fehler.length) fehlerOeffnen(fehler[0]);
    return fehler.length === 0;
  }
  function fehlerOeffnen(f: { gruppe: string; pfad: string }) {
    bereich = f.gruppe;
    alleFelder = true;
    void tick().then(() => {
      const block = [
        ...(bereichElement?.querySelectorAll<HTMLElement>("[data-feldpfad]") ??
          []),
      ].find((e) => e.dataset.feldpfad === f.pfad);
      if (!block) return;
      for (let p = block.parentElement; p; p = p.parentElement)
        if (p instanceof HTMLDetailsElement) p.open = true;
      const e = block.querySelector<HTMLElement>("input,select,textarea");
      e?.focus();
      e?.scrollIntoView({ block: "center" });
    });
  }
  function neu(t: FormTabelle) {
    const row = JSON.parse(JSON.stringify(t.leer)) as Obj;
    if ("id" in row) row.id = crypto.randomUUID();
    zeilen(t).push(row);
  }
  function entfernen(t: FormTabelle, index: number) {
    const row = zeilen(t)[index];
    if (s.eingabe)
      s.eingabe.uebernahmen = s.eingabe.uebernahmen.filter(
        (u) => !u.ziel.startsWith(`${t.pfad}.${String(row.id)}.`),
      );
    zeilen(t).splice(index, 1);
    if (s.eingabe?.planbindungen)
      s.eingabe.planbindungen = s.eingabe.planbindungen.filter(b=>!b.ziel.startsWith(`${t.pfad}.${String(row.id)}.`));
  }
  function ziel(t: FormTabelle | null, row: Obj | null, f: Feld) {
    return t ? `${t.pfad}.${String(row?.id)}.${f.name}` : f.name;
  }
  function quelleFuer(path: string) {
    return s.eingabe?.uebernahmen.find((u) => u.ziel === path);
  }
  function planquelleFuer(path: string) {
    return s.eingabe?.planbindungen?.find(b=>b.ziel===path);
  }
  function quellenFuer(f: Feld) {
    return nachweise.flatMap((n) => {
      if (
        n.stand.eingabe.art !== "fach" ||
        !n.aktuell ||
        !n.stand.vollstaendig ||
        n.stand.eingabe.plan.grundstueck_id !== s.eingabe?.grundstueck_id
      )
        return [];
      const r = n.stand.ergebnis as unknown as FachErgebnis;
      return r.kennzahlen
        .filter((k) => k.groesse === f.groesse && k.wert !== null)
        .map((k) => ({
          value: `${n.stand.id}|${k.id}`,
          titel: `${n.stand.bezeichnung} · ${k.bezeichnung}: ${z(k.wert)}`,
          nachweis: n.stand.id,
          k,
        }));
    });
  }
  function binden(
    path: string,
    value: string,
    f: Feld,
    setzen: (v: unknown) => void,
  ) {
    if (!s.eingabe) return;
    s.eingabe.uebernahmen = s.eingabe.uebernahmen.filter(
      (u) => u.ziel !== path,
    );
    const src = quellenFuer(f).find((q) => q.value === value);
    if (src) {
      s.eingabe.uebernahmen.push({
        nachweis_id: src.nachweis,
        kennzahl: src.k.id,
        ziel: path,
      });
      setzen(src.k.wert);
    }
  }
  /** Ladefehler der Auswahllisten; nicht still als leere Liste anzeigen. */
  let quellenFehler = $state<string | null>(null),
    grundFehler = $state<string | null>(null);
  onMount(async () => {
    try {
      const v = await berechnungsApi.vorlagen();
      vorlage = v.find((v) => v.plan.modul === modul) ?? null;
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
  async function quellenLaden() {
    try {
      const [g, d, n] = await Promise.all([
        plattformApi.liste<Gebaeude>(projekt.id, "gebaeude"),
        api.dokumente(projekt.id),
        plattformApi.nachweise(projekt.id),
      ]);
      gebaeude = g;
      dateien = d;
      nachweise = n;
      quellenFehler = null;
    } catch (e) {
      quellenFehler = fehlertext(e);
    }
  }
  async function grundLaden() {
    try {
      grund = await api.grundstueckskontext(projekt.id);
      grundFehler = null;
    } catch (e) {
      grundFehler = fehlertext(e);
    }
  }
  onMount(() => void quellenLaden());
  // Baugrundstücke hängen am Projektstand.
  $effect(() => {
    void projekt.version;
    untrack(() => void grundLaden());
  });
  // Gebäude, Nachweise oder Dokumente anderer Bereiche dieser Akte geändert.
  $effect(() =>
    abonnieren(projekt.id, ["dokumente", "fachobjekte", "nachweise"], (e) => {
      planQuellenStand++;
      void quellenLaden();
      if (e.bereich === "fachobjekte") void raumbuecherLaden();
    }),
  );
  async function pruefen() {
    if (!s.eingabe || busy || !formatPruefen()) return;
    busy = true;
    s.fehler = null;
    try {
      const r = await berechnungsApi.pruefen(
        projekt.id,
        JSON.parse(JSON.stringify(s.eingabe)),
      );
      if (darf) s.eingabe = r.plan;
      result = r.ergebnis;
      resultSignatur = signatur(darf ? r.plan : s.eingabe);
    } catch (e) {
      s.fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  let raumbuchId = $state("");
  async function raumbuecherLaden() {
    if (["wohnflaechen", "din277"].includes(modul)) {
      try {
        raumbuecher = await plattformApi.liste(projekt.id, "raumbuch");
      } catch (e) {
        s.fehler = fehlertext(e);
      }
    }
  }
  onMount(raumbuecherLaden);
  async function raumimport() {
    if (!s.eingabe || !raumbuchId) return;
    busy = true;
    try {
      const r = await berechnungsApi.ausRaumbuch(projekt.id, raumbuchId, modul);
      s.eingabe.gebaeude_id = r.gebaeude_id;
      s.eingabe.fachquellen = r.fachquellen;
      s.eingabe.uebernahmen = [];
      // Abgleich über die stabile Raum-ID: Flächen und Höhen stammen aus dem
      // Raumbuch, bereits erfasste Zuordnungen bleiben erhalten. Manuelle
      // Positionen ohne Raumbuchquelle bleiben stehen; entfallene Räume entfallen.
      const alt = (lesen(s.eingabe.eingaben, "positionen") as Obj[] | undefined) ?? [];
      const neu = (lesen(r.eingaben, "positionen") as Obj[] | undefined) ?? [];
      const ausRaumbuch = (x: Obj) => String(x.quelle ?? "").startsWith("Raumbuch");
      const eigene = modul === "wohnflaechen" ? ["wohnung", "typ", "sonderfaktor", "begruendung"] : ["geschoss", "art", "bereich"];
      let erhalten = 0;
      const abgeglichen = neu.map((n) => {
        const a = alt.find((x) => x.id === n.id);
        // Höhenzone („Raum_Zone“) neu: Raumart und Wohnung vom selben Raum
        // übernehmen, nicht dessen Höhe.
        const raum = String(n.id).split("_")[0];
        const verwandt = !a && String(n.id).includes("_")
          ? alt.find((x) => x.id === raum || String(x.id).startsWith(`${raum}_`))
          : undefined;
        const quelle = a ?? verwandt;
        if (!quelle) return n;
        if (a) erhalten++;
        const z = { ...n };
        for (const k of eigene) if (k !== "begruendung" || a) if (quelle[k] !== undefined && quelle[k] !== "" && quelle[k] !== null) z[k] = quelle[k];
        if (a && z.lichte_hoehe_m == null && a.lichte_hoehe_m != null) z.lichte_hoehe_m = a.lichte_hoehe_m;
        return z;
      });
      const manuell = alt.filter((a) => !neu.some((n) => n.id === a.id) && !ausRaumbuch(a));
      const entfallen = alt.filter((a) => !neu.some((n) => n.id === a.id) && ausRaumbuch(a)).length;
      // PDF-Verknüpfungen bleiben nur an manuellen Positionen; Raumbuchzeilen
      // erhalten ihre Fläche aus dem Raumbuch.
      const manuelleIds = new Set(manuell.map((x) => String(x.id)));
      if (s.eingabe.planbindungen)
        s.eingabe.planbindungen = s.eingabe.planbindungen.filter((b) => !b.ziel.startsWith("positionen.") || manuelleIds.has(b.ziel.split(".")[1]));
      schreiben(s.eingabe.eingaben, "positionen", [...abgeglichen, ...manuell]);
      for (const f of [
        "aufmass_vollstaendig",
        "wohneinheiten_bestaetigt",
        "klassifikation_geprueft",
        "kgf_vollstaendig",
      ]) {
        if (f in (s.eingabe.eingaben as Obj))
          schreiben(s.eingabe.eingaben, f, false);
      }
      s.eingabe.regelkontext.anwendbarkeit_bestaetigt = false;
      s.meldung =
        `Raumfakten übernommen: ${neu.length - erhalten} neu, ${erhalten} aktualisiert (Zuordnungen erhalten)${entfallen ? `, ${entfallen} nicht mehr im Raumbuch und entfernt` : ""}${manuell.length ? `, ${manuell.length} manuelle Positionen beibehalten` : ""}. Raumarten, Wohnungs-/Geschosszuordnung, Höhenzonen und Maßbezug prüfen.`;
    } catch (e) {
      s.fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  const einheitText = (s: string) =>
    ({
      m: "m",
      m2: "m²",
      m3: "m³",
      anzahl: "Anzahl",
      grad: "°",
      prozent: "%",
      l_pro_s: "l/s",
    })[s] ?? s;
  let archivSignatur = "";
  let archivVorgang = crypto.randomUUID();
  async function archivieren() {
    if (!s.eingabe || !s.id || busy) return null;
    busy = true;
    s.fehler = null;
    try {
      if (!(await s.speichern())) return null;
      const r = await berechnungsApi.pruefen(
        projekt.id,
        JSON.parse(JSON.stringify(s.eingabe)),
      );
      if (JSON.stringify(r.plan) !== JSON.stringify(s.eingabe)) {
        s.eingabe = r.plan;
        if (!(await s.speichern())) return null;
      }
      const signatur = JSON.stringify([s.id, s.eingabe]);
      if (signatur !== archivSignatur) {
        archivVorgang = crypto.randomUUID();
        archivSignatur = signatur;
      }
      const n = await plattformApi.nachweisErstellen(
        projekt.id,
        archivVorgang,
        s.eingabe.bezeichnung,
        {
          art: "fach",
          objekt: s.id,
          plan: JSON.parse(JSON.stringify(s.eingabe)),
        },
      );
      nachweise = await plattformApi.nachweise(projekt.id);
      return { art: "nachweis" as const, id: n.id };
    } catch (e) {
      s.fehler = fehlertext(e);
      return null;
    } finally {
      busy = false;
    }
  }
  function regelimport() {
    if (!s.eingabe) return;
    try {
      const r = JSON.parse(importText);
      if (r.modul !== modul || !r.regelkontext || !r.regeln)
        throw Error("Regelpaket passt nicht zu diesem Modul.");
      s.eingabe.regelkontext = {
        ...r.regelkontext,
        anwendbarkeit_bestaetigt: false,
      };
      schreiben(s.eingabe.eingaben, "regeln", r.regeln);
      importText = "";
      importOffen = false;
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
</script>

<div bind:this={bereichElement} data-berechnungsmodul={modul}>
{#if vorlage}
  <Fachliste
    titel={vorlage.titel}
    {s}
    bereich={modul}
    {darf}
    lesendBedienbar
    zusatzBeschaeftigt={() => busy}
    pruefen={formatPruefen}
  >
    {#snippet children(p)}
      <p class="meta">{vorlage!.hinweis}</p>
      <p class="meta">* Erforderliche Zahl. Leere optionale Angaben bleiben unbekannt; 0 ist ein bewusst erfasster Wert.</p>
      {#if eingabefehler.length}<div class="fehlerzusammenfassung" role="alert"><strong>Bitte die Eingaben prüfen.</strong><ul>{#each eingabefehler as f}<li><button type="button" class="link" onclick={()=>fehlerOeffnen(f)}>{f.titel}: Eingabe prüfen.</button></li>{/each}</ul></div>{/if}
      <nav class="fach-tabs" aria-label="Berechnungsbereiche">
        {#each [["daten", "Eingaben"], ["regeln", "Regeln / Grundlagen"], ["quellen", "Quellen / Übernahmen"], ["bericht", "Prüfbericht"]] as [id, t]}<button
            type="button"
            class:aktiv={bereich === id}
            aria-pressed={bereich === id}
            onclick={() => (bereich = id)}>{t}</button
          >{/each}
      </nav>
      <fieldset disabled={!darf || busy}>
        {#if bereich === "daten"}
          <div class="fachraster">
            <label
              data-feldpfad="$bezeichnung">Bezeichnung<span class="pflicht" aria-hidden="true"> *</span><input
                aria-label="Bezeichnung"
                required
                aria-invalid={pruefversuch && !p.bezeichnung.trim() ? 'true' : undefined}
                bind:value={p.bezeichnung}
                maxlength="500"
              /></label
            ><label
              >Baugrundstück<select bind:value={p.grundstueck_id}
                ><option value={null}>Noch nicht zugeordnet</option
                >{#each grund as g}<option value={g.id}>{g.bezeichnung}</option
                  >{/each}</select
              ></label
            ><label
              >Gebäude<select bind:value={p.gebaeude_id}
                ><option value={null}>Kein einzelnes Gebäude</option
                >{#each gebaeude as g}<option value={g.objekt}
                    >{g.daten.bezeichnung}</option
                  >{/each}</select
              ></label
            >
          </div>
          {#if modul === "vollgeschosse"}<section class="positionszeile">
            <h3>Bekannte Gebäudeangaben verwenden</h3>
            <label><input type="checkbox" checked={p.gebaeudedaten_uebernehmen === true} onchange={(e) => { p.gebaeudedaten_uebernehmen = e.currentTarget.checked; result = null; }} />Geschosse, Flächen und bekannte Höhen aus Gebäude/Geschosse übernehmen</label>
            <p class="meta">Beim Prüfen und Speichern werden die Geschoss- und Höhenzonenlisten aus dem ausgewählten Gebäude befüllt und aktualisiert. Eine Höhenzone über die gesamte Fläche entsteht nur bei bestätigter einheitlicher Raumhöhe. Für eigene Höhenzonen diese Übernahme lösen. Änderungen an den Grundlagen setzen die Aufmaßprüfung zurück.</p>
            {#if p.gebaeudedaten_uebernehmen}<button type="button" class="sekundaer" disabled={!p.gebaeude_id} onclick={pruefen}>Gebäudeangaben laden und prüfen</button>{/if}
          </section>{:else if modul === "hoehen"}<section class="positionszeile">
            <h3>Bekannte Gebäudeangaben verwenden</h3>
            <label><input type="checkbox" checked={p.gebaeudedaten_uebernehmen === true} onchange={(e) => { p.gebaeudedaten_uebernehmen = e.currentTarget.checked; result = null; }} />Trauf- und Firsthöhe aus Gebäude/Geschosse übernehmen</label>
            <p class="meta">Beim Prüfen und Speichern entstehen aus Gebäudenullpunkt (m ü. NHN), Traufe und First des ausgewählten Gebäudes Höhenpunkte mit der mittleren Geländehöhe als Bezug. Eigene Punkte bleiben erhalten. Weicht der maßgebende Bezugspunkt der Festsetzung ab, eigene Punkte erfassen. Geänderte Grundlagen setzen die Bezugsprüfung zurück.</p>
            {#if p.gebaeudedaten_uebernehmen}<button type="button" class="sekundaer" disabled={!p.gebaeude_id} onclick={pruefen}>Gebäudehöhen laden und prüfen</button>{/if}
          </section>{/if}
          {#if ["wohnflaechen", "din277"].includes(modul)}<details>
              <summary>Raumfakten aus dem Raumbuch übernehmen</summary>
              <p class="meta">
                Gleicht die Teilflächen über die Raum-ID ab: Flächen und Höhen
                kommen aus dem Raumbuch, bereits gewählte Raumart und
                Wohnungs-/Geschosszuordnung bleiben erhalten. Neue Räume sind
                noch zuzuordnen; manuelle Positionen bleiben stehen.
              </p>
              <label
                >Raumbuch<select bind:value={raumbuchId}
                  ><option value="">Auswählen</option
                  >{#each raumbuecher as r}<option value={r.objekt}
                      >{r.daten.bezeichnung} · Revision {r.revision}</option
                    >{/each}</select
                ></label
              ><button
                type="button"
                class="sekundaer"
                disabled={!raumbuchId}
                onclick={raumimport}
                >Raumfakten übernehmen / aktualisieren</button
              >
            </details>{/if}
          {#if quellenFehler}<p class="fehlertext" role="alert">
              Gebäude, Dokumente oder Nachweise konnten nicht geladen werden: {quellenFehler}
            </p>{/if}
          {#if grundFehler}<p class="fehlertext" role="alert">
              Baugrundstücke konnten nicht geladen werden: {grundFehler}
            </p>
          {:else if !grund.length}<p class="hinweisbox">
              Für einen vollständigen Nachweis zuerst die
              Baugrundstücksermittlung prüfen und speichern. Eingaben können
              bereits erfasst werden.
            </p>{/if}
        {:else if bereich === "regeln"}
          <div class="fachraster">
            <label
              >Regel-/Satzungsbezeichnung<input
                bind:value={p.regelkontext.bezeichnung}
              /></label
            ><label
              >Regelversion<input bind:value={p.regelkontext.version} /></label
            ><label
              >Geltungsbereich / Kommune<textarea
                bind:value={p.regelkontext.geltungsbereich}
              ></textarea></label
            ><label
              >Fundstelle / Rechtsfassung<textarea
                bind:value={p.regelkontext.quelle}
              ></textarea></label
            ><label
              >Gültig ab<input
                type="date"
                bind:value={p.regelkontext.gueltig_ab}
              /></label
            ><label
              >Gültig bis<input
                type="date"
                bind:value={p.regelkontext.gueltig_bis}
              /></label
            >
          </div>
          <p class="meta">
            Geänderte Regelwerte benötigen eine neue Regelversion. Nach
            Änderungen an Eingaben/Grundlagen wird die Bestätigung
            zurückgesetzt: zuerst speichern, danach die Anwendbarkeit
            bestätigen.
          </p>
          <label
            >Prüfvermerk<textarea
              rows="3"
              bind:value={p.regelkontext.pruefvermerk}
            ></textarea></label
          ><label
            ><input
              type="checkbox"
              bind:checked={p.regelkontext.anwendbarkeit_bestaetigt}
              disabled={!darfFreigeben}
            />Anwendbarkeit und Vollständigkeit der Regelgrundlagen durch
            Projektleitung bestätigt</label
          >
          {#if lesen(p.eingaben, "regeln") && !Array.isArray(lesen(p.eingaben, "regeln"))}<button
              type="button"
              class="sekundaer"
              onclick={() => (importOffen = !importOffen)}
              >Eigenes Regelpaket importieren …</button
            >{#if importOffen}<label
                >Regelpaket als JSON<textarea rows="8" bind:value={importText}
                ></textarea></label
              ><button type="button" onclick={regelimport}
                >Regelpaket übernehmen</button
              >{/if}{/if}
        {/if}
        {#if bereich === "daten" || bereich === "regeln"}
          <div class="fachraster">
            {#each vorlage!.felder.filter((f) => f.gruppe === bereich) as f (f.name)}<div
              >
                <Fachfeld
                  feld={f}
                  {pruefversuch}
                  quelle={quelleFuer(f.name) ? "Aus archiviertem Nachweis übernommen · Herkunft unter Quellen / Übernahmen" : ""}
                  wert={lesen(p.eingaben, f.name)}
                  optionen={optionen(f)}
                  disabled={!!quelleFuer(f.name)}
                  onwert={(v) => schreiben(p.eingaben, f.name, v)}
                />{#if f.groesse}{@render uebernahme(f.name, f, (v) =>
                    schreiben(p.eingaben, f.name, v),
                  )}{/if}
              </div>{/each}
          </div>
          {#if ["wohnflaechen","din277"].includes(modul) && bereich === 'daten'}<label class="checkboxzeile"><input type="checkbox" bind:checked={alleFelder} />Auch derzeit nicht verwendete Geometriewerte anzeigen</label><p class="meta">Beim Wechsel der Aufmaßform bleiben vorhandene Werte erhalten. Verwendet werden die zur Aufmaßform gehörenden Werte.</p>{/if}
          {#each vorlage!.tabellen.filter((t) => t.gruppe === bereich) as t}<section
              class="positionsliste"
              aria-label={t.titel}
            >
              <header>
                <h3>{t.titel}</h3>
                <button type="button" class="sekundaer" disabled={modul === "vollgeschosse" && p.gebaeudedaten_uebernehmen === true && t.gruppe === "daten"} onclick={() => neu(t)}
                  >Position hinzufügen</button
                >
              </header>
              {#each zeilen(t) as row, i (row.id ?? i)}<details class="positionszeile">
                  <summary>{i + 1} · {name(row)}</summary>
                  <div class="fachraster">
                    {#each t.felder.filter((f) => f.typ !== "hidden" && (alleFelder || feldSichtbar(modul,t.pfad,f,row))) as f (f.name)}<div>
                        <Fachfeld
                          feld={f}
                          pfad={`${t.pfad}[${i}].${f.name}`}
                          {pruefversuch}
                          quelle={planquelleFuer(ziel(t,row,f)) ? "Aus PDF-Aufmaß verknüpft · Herkunft unter Quellen / Übernahmen" : quelleFuer(ziel(t,row,f)) ? "Aus archiviertem Nachweis übernommen · Herkunft unter Quellen / Übernahmen" : ""}
                          wert={row[f.name]}
                          optionen={optionen(f)}
                          disabled={!!planquelleFuer(ziel(t,row,f)) || !!quelleFuer(ziel(t, row, f)) || (modul === "vollgeschosse" && p.gebaeudedaten_uebernehmen === true && t.gruppe === "daten" && !["sonderentscheidung", "sondergrundlage"].includes(f.name))}
                          onwert={(v) => (row[f.name] = v)}
                        />{#if f.groesse}{@render uebernahme(
                            ziel(t, row, f),
                            f,
                            (v) => (row[f.name] = v),
                          )}{/if}
                      </div>{/each}
                  </div>
                  <button
                    type="button"
                    class="link"
                    disabled={modul === "vollgeschosse" && p.gebaeudedaten_uebernehmen === true && t.gruppe === "daten"} onclick={() => entfernen(t, i)}>Position entfernen</button
                  >
                </details>{:else}<p class="meta">
                  Noch keine Positionen.
                </p>{/each}
            </section>{/each}
        {:else if bereich === "quellen"}
          <Planquellen projektId={projekt.id} plan={p} onplan={neu=>s.eingabe=neu as FachPlan} onbusy={b=>busy=b} />
          <p>
            Verwendete Pläne, Aufmaße und Rechts-/Satzungsbelege mit ihrer
            konkreten Version zuordnen.
          </p>
          <label
            >Projektunterlage<select bind:value={quellId}
              ><option value="">Unterlage auswählen</option
              >{#each dateien as d}<option
                  value={`${d.document_id}|${d.version}`}
                  >{d.file_name} · Version {d.version}</option
                >{/each}</select
            ></label
          ><button
            type="button"
            class="sekundaer"
            disabled={!quellId}
            onclick={() => {
              const [d, v] = quellId.split("|");
              if (
                !p.quellen.some(
                  (q) => q.dokument_id === d && q.version === Number(v),
                )
              )
                p.quellen.push({ dokument_id: d, version: Number(v) });
            }}>Quelle hinzufügen</button
          >
          <ul>
            {#each p.quellen as q, i}<li>
                {dateien.find(
                  (d) =>
                    d.document_id === q.dokument_id && d.version === q.version,
                )?.file_name ?? "Archivierte Unterlage"} · Version {q.version}
                <button
                  type="button"
                  class="link"
                  onclick={() => p.quellen.splice(i, 1)}>Entfernen</button
                >
              </li>{/each}
          </ul>
          <p class="meta">
            Übernahmen werden direkt am passenden Zahlenfeld ausgewählt. Es
            werden nur aktuelle vollständige Fachnachweise derselben
            Grundstückseinheit angeboten. Einheiten und fachliche
            Flächenbegriffe prüft der Rechenkern.
          </p>
          <ul>
            {#each p.uebernahmen as u}<li>
                {nachweise.find((n) => n.stand.id === u.nachweis_id)?.stand
                  .bezeichnung ?? "Quellnachweis"} → {(
                  (
                    nachweise.find((n) => n.stand.id === u.nachweis_id)?.stand
                      .ergebnis as unknown as FachErgebnis
                  )?.kennzahlen ?? []
                ).find((k) => k.id === u.kennzahl)?.bezeichnung ??
                  "Übernommener Fachwert"}
                <span>{nachweise.find(n=>n.stand.id===u.nachweis_id)?.aktuell === true ? "Quelle aktuell" : "Quelle nicht aktuell bestätigt – prüfen"}</span>
                <button type="button" class="link" onclick={()=>navigation?.springen(navigationsziel("nachweise",null,null,u.nachweis_id))}>Quellnachweis ansehen</button><button
                  type="button"
                  class="link"
                  onclick={() =>
                    (p.uebernahmen = p.uebernahmen.filter((x) => x !== u))}
                  >Übernahme lösen</button
                >
              </li>{/each}
          </ul>
        {/if}
      </fieldset>
      <div class="knoepfe links">
        <button type="button" disabled={busy} onclick={pruefen}
          >Berechnung prüfen</button
        ><Druckknopf
          projektId={projekt.id}
          darfAblegen={darf}
          disabled={!darf || busy}
          text="Nachweis archivieren und PDF ausgeben …"
          vorbereiten={archivieren}
        />
      </div>
      {#if result}<section aria-label="Berechnungsergebnis">
          <h3>Prüfbericht</h3>
          <Statusanzeige status={!aktuelleVorschau ? "veraltet" : result.vollstaendig ? "berechnet" : "offen"}/>
          <div class="knoepfe links"><button type="button" class="link" onclick={()=>bereich="quellen"}>Verwendete Quellen prüfen</button><button type="button" class="link" onclick={()=>navigation?.springen(navigationsziel("nachweise"))}>Archiv und Freigaben öffnen</button></div>
          {#if !aktuelleVorschau}<p class="hinweisbox">
              {eingabenGleich
                ? "Verknüpfte Gebäude-, Raumbuch- oder Grundstücksdaten haben sich geändert."
                : "Eingaben geändert."} Vorschau erneut berechnen.
              <button type="button" class="link" disabled={busy} onclick={pruefen}>Jetzt neu berechnen</button>
            </p>{/if}
          <p class:fehlertext={!result.vollstaendig}>
            {result.vollstaendig
              ? "Vollständig ermittelt – fachliche Bürofreigabe erfolgt in den Nachweisständen."
              : "Offene Punkte – noch kein vollständiger Nachweis."}
          </p>
          <ul>
            {#each result.offen as o}<li>{o}</li>{/each}
          </ul>
          <table class="ergebnistabelle">
            <thead
              ><tr><th>Kennzahl</th><th>Wert</th><th>Einheit</th></tr></thead
            ><tbody
              >{#each result.kennzahlen as k}<tr
                  ><td>{k.bezeichnung}</td><td>{z(k.wert)}</td><td
                    >{einheitText(k.einheit)}</td
                  ></tr
                >{/each}</tbody
            >
          </table>
          {#if bereich !== "bericht" && result.tabellen.length}<button type="button" class="link" onclick={()=>bereich="bericht"}>Vollständigen Rechenweg anzeigen ({result.tabellen.length} Tabellen)</button>{/if}
          {#if bereich === "bericht"}{#each result.tabellen as t}<h4>
                {t.titel}
              </h4>
              <div class="tabellen-scroll">
                <table class="ergebnistabelle">
                  <thead
                    ><tr
                      >{#each t.spalten as sp}<th>{sp}</th>{/each}</tr
                    ></thead
                  ><tbody
                    >{#each t.zeilen as row}<tr
                        >{#each row as cell}<td>{cell}</td>{/each}</tr
                      >{/each}</tbody
                  >
                </table>
              </div>{/each}{/if}{#each result.hinweise as h}<p class="meta">
              {h}
            </p>{/each}
        </section>{/if}
    {/snippet}
  </Fachliste>
{:else}<p>Berechnungsmodul wird geladen …</p>
  {#if s.fehler}<p role="alert" class="fehlertext">{s.fehler}</p>{/if}{/if}
{#snippet uebernahme(path: string, f: Feld, setzen: (v: unknown) => void)}
  <label class="uebernahme"
    >Wert aus Nachweis übernehmen<select
      disabled={!darf || busy}
      value={quelleFuer(path)
        ? `${quelleFuer(path)!.nachweis_id}|${quelleFuer(path)!.kennzahl}`
        : ""}
      onchange={(e) => binden(path, e.currentTarget.value, f, setzen)}
      ><option value="">Manuelle Eingabe</option
      >{#each quellenFuer(f) as q}<option value={q.value}>{q.titel}</option
        >{/each}</select
    ></label
  >
{/snippet}

</div>

<style>
  fieldset {
    border: 0;
    padding: 0;
    margin: 0;
    min-width: 0;
  }
  .fach-tabs {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-block: 1rem;
  }
  .fach-tabs button {
    background: var(--panel);
    color: var(--fg);
    border: 1px solid var(--border);
  }
  .fach-tabs button.aktiv {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .fachraster {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 18rem), 1fr));
    gap: 1rem;
    margin-block: 1rem;
  }
  .fachraster label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  input,
  select,
  textarea {
    box-sizing: border-box;
    max-width: 100%;
  }
  .positionsliste {
    margin-block: 1.4rem;
  }
  .positionsliste > header {
    display: flex;
    gap: 1rem;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
  }
  .positionszeile {
    border: 1px solid var(--border);
    padding: 0.7rem;
    margin-block: 0.5rem;
  }
  .positionszeile summary {
    cursor: pointer;
  }
  .uebernahme {
    font-size: 0.8rem;
    margin-top: 0.5rem;
  }
  fieldset {
    min-width: 0;
  }
  li {
    margin-block: 0.4rem;
  }
</style>
