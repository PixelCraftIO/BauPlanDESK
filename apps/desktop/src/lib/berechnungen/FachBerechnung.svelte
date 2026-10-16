<script lang="ts">
  import { onMount, untrack } from "svelte";
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
    nachweise = $state<NachweisAnsicht[]>([]);
  let bereich = $state("daten"),
    result = $state<FachErgebnis | null>(null),
    resultSignatur = $state(""),
    busy = $state(false),
    importText = $state(""),
    importOffen = $state(false),
    quellId = $state("");
  const s = untrack(
    () =>
      new FachSitzung<FachPlan>(
        projekt.id,
        modul,
        benutzerId,
        () => JSON.parse(JSON.stringify(vorlage!.plan)),
        (p) => p.bezeichnung,
      ),
  );
  const aktuelleVorschau = $derived(
    !!result && !!s.eingabe && JSON.stringify(s.eingabe) === resultSignatur,
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
  }
  function ziel(t: FormTabelle | null, row: Obj | null, f: Feld) {
    return t ? `${t.pfad}.${String(row?.id)}.${f.name}` : f.name;
  }
  function quelleFuer(path: string) {
    return s.eingabe?.uebernahmen.find((u) => u.ziel === path);
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
  onMount(async () => {
    try {
      const [v, g, d, n] = await Promise.all([
        berechnungsApi.vorlagen(),
        plattformApi.liste<Gebaeude>(projekt.id, "gebaeude"),
        api.dokumente(projekt.id),
        plattformApi.nachweise(projekt.id),
      ]);
      vorlage = v.find((v) => v.plan.modul === modul) ?? null;
      gebaeude = g;
      dateien = d;
      nachweise = n;
      try {
        grund = await api.grundstueckskontext(projekt.id);
      } catch {
        grund = [];
      }
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
  async function pruefen() {
    if (!s.eingabe || busy) return;
    busy = true;
    s.fehler = null;
    try {
      const r = await berechnungsApi.pruefen(
        projekt.id,
        JSON.parse(JSON.stringify(s.eingabe)),
      );
      if (darf) s.eingabe = r.plan;
      result = r.ergebnis;
      resultSignatur = JSON.stringify(darf ? r.plan : s.eingabe);
    } catch (e) {
      s.fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  let raumbuecher = $state<
    Datensatz<import("../generated/plattform").Raumbuch>[]
  >([]);
  let raumbuchId = $state("");
  onMount(async () => {
    if (["wohnflaechen", "din277"].includes(modul)) {
      try {
        raumbuecher = await plattformApi.liste(projekt.id, "raumbuch");
      } catch (e) {
        s.fehler = fehlertext(e);
      }
    }
  });
  async function raumimport() {
    if (!s.eingabe || !raumbuchId) return;
    busy = true;
    try {
      const r = await berechnungsApi.ausRaumbuch(projekt.id, raumbuchId, modul);
      s.eingabe.gebaeude_id = r.gebaeude_id;
      s.eingabe.fachquellen = r.fachquellen;
      s.eingabe.uebernahmen = [];
      schreiben(
        s.eingabe.eingaben,
        "positionen",
        lesen(r.eingaben, "positionen"),
      );
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
        "Raumfakten übernommen. Raumarten, Wohnungs-/Geschosszuordnung, Höhenzonen und Maßbezug ausdrücklich prüfen.";
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

{#if vorlage}
  <Fachliste
    titel={vorlage.titel}
    {s}
    bereich={modul}
    {darf}
    lesendBedienbar
    zusatzBeschaeftigt={() => busy}
  >
    {#snippet children(p)}
      <p class="meta">{vorlage!.hinweis}</p>
      <nav class="fach-tabs" aria-label="Berechnungsbereiche">
        {#each [["daten", "Eingaben"], ["regeln", "Regeln / Grundlagen"], ["quellen", "Quellen / Übernahmen"], ["bericht", "Prüfbericht"]] as [id, t]}<button
            type="button"
            class:aktiv={bereich === id}
            onclick={() => (bereich = id)}>{t}</button
          >{/each}
      </nav>
      <fieldset disabled={!darf || busy}>
        {#if bereich === "daten"}
          <div class="fachraster">
            <label
              >Bezeichnung<input
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
          </section>{/if}
          {#if ["wohnflaechen", "din277"].includes(modul)}<details>
              <summary>Raumfakten aus dem Raumbuch übernehmen</summary>
              <p class="meta">
                Ersetzt die Teilflächenliste. Raumart und
                Wohnungs-/Geschosszuordnung werden bewusst nicht automatisch
                festgelegt.
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
                >Raumfakten übernehmen (Teilflächen ersetzen)</button
              >
            </details>{/if}
          {#if !grund.length}<p class="hinweisbox">
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
            {#each vorlage!.felder.filter((f) => f.gruppe === bereich) as f}<div
              >
                <Fachfeld
                  feld={f}
                  wert={lesen(p.eingaben, f.name)}
                  optionen={optionen(f)}
                  disabled={!!quelleFuer(f.name)}
                  onwert={(v) => schreiben(p.eingaben, f.name, v)}
                />{#if f.groesse}{@render uebernahme(f.name, f, (v) =>
                    schreiben(p.eingaben, f.name, v),
                  )}{/if}
              </div>{/each}
          </div>
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
              {#each zeilen(t) as row, i}<details class="positionszeile">
                  <summary>{i + 1} · {name(row)}</summary>
                  <div class="fachraster">
                    {#each t.felder.filter((f) => f.typ !== "hidden") as f}<div>
                        <Fachfeld
                          feld={f}
                          wert={row[f.name]}
                          optionen={optionen(f)}
                          disabled={!!quelleFuer(ziel(t, row, f)) || (modul === "vollgeschosse" && p.gebaeudedaten_uebernehmen === true && t.gruppe === "daten" && !["sonderentscheidung", "sondergrundlage"].includes(f.name))}
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
                  "Übernommener Fachwert"}<button
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
          {#if !aktuelleVorschau}<p class="hinweisbox">
              Eingaben geändert. Vorschau erneut berechnen.
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
