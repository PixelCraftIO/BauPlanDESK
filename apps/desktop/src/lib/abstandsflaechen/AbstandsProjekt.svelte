<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { save } from "@tauri-apps/plugin-dialog";
  import Fachliste from "../Fachliste.svelte";
  import Planquellen from "../planaufmass/Planquellen.svelte";
  import { abonnieren } from "../projektEreignisse";
  import { FachSitzung } from "../FachSitzung.svelte";
  import { plattformApi } from "../plattformApi";
  import { api, fehlertext, type Project, type DocumentVersion } from "../api";
  import type {
    AbstandsPlan,
    AbstandsRegeln,
    AbstandsErgebnis,
    AbstandsWand,
    AbstandsUmriss,
    Dokumentbezug,
    Gebaeude,
    UmrissAbgleich,
  } from "../generated/plattform";
  import Druckknopf from "../Druckknopf.svelte";
  import Dokumentliste from "../verfahren/Dokumentliste.svelte";
  import Dokumentfeld from "../verfahren/Dokumentfeld.svelte";
  import Punktliste from "./Punktliste.svelte";
  import Planbild from "./Planbild.svelte";
  import Hoehenprofil from "./Hoehenprofil.svelte";
  import Abwicklung from "./Abwicklung.svelte";
  import { grundstuecksinformationen } from "../grundstuecksdaten";
  let {
    projekt,
    benutzerId,
    darf,
    darfFreigeben = false,
  }: {
    projekt: Project;
    benutzerId: string;
    darf: boolean;
    darfFreigeben?: boolean;
  } = $props();
  let katalog = $state<AbstandsRegeln[]>([]),
    dateien = $state<DocumentVersion[]>([]),
    bereich = $state("grundlagen"),
    auswahl = $state(""),
    ergebnis = $state<AbstandsErgebnis | null>(null),
    ergebnisStand = $state(""),
    busy = $state(false),
    meldung = $state("");
  function leer(): AbstandsPlan {
    return {
      bezeichnung: "Abstandsflächennachweis",
      regeln: structuredClone(
        $state.snapshot(
          (katalog.find((r) => r.land === projekt.bundesland) ??
            s.eingabe?.regeln)!,
        ),
      ),
      recht: {
        gemeinde: "",
        fassung: "",
        anwendung: "",
        gebiet: projekt.planungsrecht.festgesetzte_werte.baugebiet ?? "WA",
        satzung: "",
        planfestsetzung: "",
        geprueft: false,
        pruefvermerk: "",
      },
      hoehenbezug: "",
      koordinatenbezug: "",
      grundstuecke: [],
      waende: [],
      flaechen: [],
      quellen: [],
      umfang_geprueft: false,
      umfang_vermerk: "",
    };
  }
  const s = untrack(
    () =>
      new FachSitzung<AbstandsPlan>(
        projekt.id,
        "abstandsflaechen",
        benutzerId,
        leer,
        (p) => p.bezeichnung,
      ),
  );
  let quellenGeneration = $state(0);
  $effect(() => abonnieren(projekt.id, ["fachobjekte", "dokumente"], () => { quellenGeneration++; }));
  const stand = $derived(JSON.stringify([s.id, s.eingabe, quellenGeneration]));
  const aktuell = $derived(ergebnisStand === stand);
  const wand = $derived(s.eingabe?.waende.find((w) => w.id === auswahl));
  let vorgang = "",
    vorgangStand = "";
  let gebaeude = $state<import("../plattformApi").Datensatz<Gebaeude>[]>([]);
  // Gleicher Adapter wie im Core, damit auch Altprojekte ihre Flurstücke zeigen.
  const flurstuecke = $derived(grundstuecksinformationen($state.snapshot(projekt.planungsrecht)).flurstuecke);
  let hoehenHinweis = $state("");
  onMount(async () => {
    try {
      [katalog, dateien, gebaeude] = await Promise.all([
        invoke<AbstandsRegeln[]>("abstandsregeln"),
        api.dokumente(projekt.id),
        plattformApi.liste<Gebaeude>(projekt.id, "gebaeude"),
      ]);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
  // Gebäude und Dokumente aus anderen Bereichen ohne Neuöffnen anbieten.
  $effect(() => {
    if (!quellenGeneration) return;
    let gone = false;
    Promise.all([api.dokumente(projekt.id), plattformApi.liste<Gebaeude>(projekt.id, "gebaeude")])
      .then(([d, g]) => { if (!gone) { dateien = d; gebaeude = g; } })
      .catch(() => { /* bisherige Listen bleiben; Fehler zeigt die nächste Prüfung */ });
    return () => { gone = true; };
  });
  async function gebaeudehoehen(id: string | undefined) {
    const g = gebaeude.find((g) => g.objekt === id);
    if (!g) return null;
    const r = await plattformApi.gebaeudeAuswerten(projekt.id, g.objekt, $state.snapshot(g.daten) as Gebaeude);
    return { name: g.daten.bezeichnung, ...r.hoehen_nhn, dach: r.dach ?? null };
  }
  /** Gelände und Traufe (m ü. NHN) des verknüpften Gebäudes als Höhenprofil. */
  async function hoehenUebernehmen(w: AbstandsWand) {
    hoehenHinweis = "";
    try {
      const h = await gebaeudehoehen(w.gebaeude_id);
      if (!h) return;
      if (h.gelaende_m === null || h.traufe_m === null) {
        hoehenHinweis = "Gebäudenullpunkt, mittlere Geländehöhe und Traufhöhe in Gebäude/Geschosse ergänzen.";
        return;
      }
      const l = Math.round(Math.hypot(w.bis.x - w.von.x, w.bis.y - w.von.y) * 1000) / 1000;
      const quelle = `Gebäude ${h.name}, Gebäude/Geschosse (m ü. NHN)`;
      // Eckhöhen eines Umrisses bleiben unberührt.
      if (!w.umriss)
        w.gelaende = [0, l].map((station_m) => ({ station_m, hoehe_m: h.gelaende_m!, quelle: `${quelle}: mittlere Geländehöhe` }));
      w.oberkante = [0, l].map((station_m) => ({ station_m, hoehe_m: h.traufe_m!, quelle: `${quelle}: Traufe` }));
      w.gelaende_a_m = w.gelaende_b_m = w.abschluss_a_m = w.abschluss_b_m = null;
      w.hoehenquelle = `Gebäude ${h.name}: Höhen aus Gebäude/Geschosse (m ü. NHN)`;
      hoehenHinweis = "Höhen übernommen. Gelände entlang der Wand, Giebel und abweichende Wandabschlüsse je Wand prüfen.";
      // Dachhöhe und Neigung aus dem Gebäude; die Anrechnung bestimmt der Core je Dachart.
      const d = h.dach;
      if (d?.form === "flachdach" && w.dachart === "ungeklaert") w.dachart = "flach";
      else if (d && d.form !== "flachdach" && (w.dachart === "giebel" || w.dachart === "dachflaeche")) {
        w.dachhoehe_m = d.dachhoehe_m;
        w.dachneigung_grad = d.neigung_grad;
        // Beim Satteldach haben beide Seiten des Giebels dieselbe Neigung.
        if (w.dachart === "giebel") w.dachneigung_b_grad = d.form === "satteldach" ? d.neigung_grad : null;
        w.dachgrundlage = `Gebäude ${h.name}: ${d.herleitung}`;
        hoehenHinweis += d.form === "satteldach" || w.dachart === "dachflaeche"
          ? " Dachhöhe und Neigung aus dem Gebäude übernommen."
          : " Dachhöhe und Neigung übernommen; Neigung der zweiten Giebelseite prüfen.";
      } else if (!d && (w.dachart === "giebel" || w.dachart === "dachflaeche"))
        hoehenHinweis += " Für Giebel und Dachflächen Dachform und Neigung in Gebäude/Geschosse erfassen.";
    } catch (e) {
      hoehenHinweis = fehlertext(e);
    }
  }
  let umrissHinweise = $state<string[]>([]);
  /** Wände der Umrisse im Core abgleichen (Lage, Außenseite, Eckhöhen). */
  async function umrisseEinbinden() {
    if (!s.eingabe?.umrisse?.length && !s.eingabe?.waende.some((w) => w.umriss)) return true;
    try {
      const r = await invoke<UmrissAbgleich>("abstandsumrisse_einbinden", { plan: $state.snapshot(s.eingabe) });
      if (JSON.stringify(r.plan) !== JSON.stringify($state.snapshot(s.eingabe))) s.eingabe = r.plan;
      umrissHinweise = r.hinweise;
      return true;
    } catch (e) {
      s.fehler = fehlertext(e);
      return false;
    }
  }
  function umrissNeu(p: AbstandsPlan) {
    const u: AbstandsUmriss = {
      id: crypto.randomUUID(),
      bezeichnung: `Gebäude ${(p.umrisse?.length ?? 0) + 1}`,
      grundstueck_id: p.grundstuecke.length === 1 ? p.grundstuecke[0].id : null,
      ecken: [],
      oberkante_m: null,
      quelle: "",
    };
    p.umrisse = [...(p.umrisse ?? []), u];
  }
  async function traufeUebernehmen(u: AbstandsUmriss) {
    hoehenHinweis = "";
    try {
      const h = await gebaeudehoehen(u.gebaeude_id);
      if (h?.traufe_m == null) {
        hoehenHinweis = "Traufhöhe in Gebäude/Geschosse ergänzen.";
        return;
      }
      u.oberkante_m = h.traufe_m;
      if (!u.quelle) u.quelle = `Gebäude ${h.name}: Traufe aus Gebäude/Geschosse (m ü. NHN)`;
    } catch (e) {
      hoehenHinweis = fehlertext(e);
    }
  }
  async function pruefen() {
    if (!s.eingabe) return;
    if (!(await umrisseEinbinden())) return;
    busy = true;
    s.laeuft = true;
    s.fehler = null;
    const q = stand;
    try {
      const r = await invoke<AbstandsErgebnis>("abstandsplan_pruefen", {
        projektId: projekt.id,
        plan: $state.snapshot(s.eingabe),
      });
      if (q === stand) {
        ergebnis = r;
        ergebnisStand = q;
        bereich = "pruefung";
      }
    } catch (e) {
      s.fehler = fehlertext(e);
    } finally {
      busy = false;
      s.laeuft = false;
    }
  }
  async function nachweis() {
    if (!s.eingabe || !s.id) return;
    if (!(await s.speichern())) return;
    s.laeuft = true;
    busy = true;
    s.fehler = null;
    try {
      const q = JSON.stringify([s.id, s.eingabe]);
      if (q !== vorgangStand) {
        vorgang = crypto.randomUUID();
        vorgangStand = q;
      }
      const n = await plattformApi.nachweisErstellen(
        projekt.id,
        vorgang,
        s.eingabe.bezeichnung,
        {
          art: "abstand_projekt",
          objekt: s.id,
          plan: structuredClone($state.snapshot(s.eingabe)),
        },
      );
      meldung =
        "Nachweisstand mit Regeln, Quellen und Geometrie archiviert. Fachliche Freigabe unter Nachweisstände.";
      return n;
    } catch (e) {
      s.fehler = fehlertext(e);
    } finally {
      s.laeuft = false;
      busy = false;
    }
  }
  async function exportieren(format: "json" | "dxf") {
    const n = await nachweis();
    if (!n) return;
    s.laeuft = true;
    try {
      const ziel = await save({
        title: "Archivierten Abstandsflächennachweis exportieren",
        defaultPath: `Abstandsflaechen.${format}`,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      if (ziel)
        await invoke("abstandsnachweis_export", {
          projektId: projekt.id,
          id: n.id,
          format,
          ziel,
        });
    } catch (e) {
      s.fehler = fehlertext(e);
    } finally {
      s.laeuft = false;
    }
  }
  function wandNeu(p: AbstandsPlan) {
    const w: AbstandsWand = {
      id: crypto.randomUUID(),
      gebaeude: "",
      bezeichnung: `Wand ${p.waende.length + 1}`,
      grundstueck_id: p.grundstuecke.length === 1 ? p.grundstuecke[0].id : null,
      von: { x: 0, y: 0 },
      bis: { x: 0, y: 0 },
      aussen: "rechts",
      gelaende_a_m: null,
      gelaende_b_m: null,
      abschluss_a_m: null,
      abschluss_b_m: null,
      dachart: "ungeklaert",
      dachhoehe_m: null,
      dachneigung_grad: null,
      dachfaktor_manuell: null,
      hoehenquelle: "",
      dachgrundlage: "",
      fall: "regel",
      grenzkennung: "",
      nutzung_geprueft: false,
      sonderansatz: null,
      polygon_manuell: null,
    };
    p.waende.push(w);
    auswahl = w.id;
  }
  async function eigeneRegeln(e: Event, p: AbstandsPlan) {
    try {
      const f = (e.currentTarget as HTMLInputElement).files?.[0];
      if (!f) return;
      if (f.size > 256 * 1024) throw Error("Regelpaket höchstens 256 KB");
      p.regeln = JSON.parse(await f.text());
      p.recht.geprueft = false;
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  const z = (n: number | null | undefined) =>
    n == null ? "—" : n.toLocaleString("de-DE", { maximumFractionDigits: 4 });
</script>

<p class="meta">
  Projektbezogener Nachweis mit getrennten Grundstückseinheiten.
  Grundbuchdateien sind optional. Rechenergebnisse ersetzen keine fachliche
  Prüfung der anwendbaren Vorschriften.
</p>
{#if !katalog.some((r) => r.land === projekt.bundesland)}<p role="status">
    Für dieses Bundesland ist kein Pilotpaket vorbereitet. Einen Plan mit
    eigenem fachlich geprüftem Regelpaket verwenden; die einfache
    Tiefenvorprüfung bleibt separat verfügbar.
  </p>
  <label
    >Eigenen Landeskatalog laden<input
      type="file"
      accept=".json"
      onchange={async (e) => {
        try {
          const f = e.currentTarget.files?.[0];
          if (f && f.size <= 256 * 1024) {
            const r: AbstandsRegeln = JSON.parse(await f.text());
            if (r.land !== projekt.bundesland)
              throw Error("Bundesland passt nicht");
            katalog = [...katalog, r];
          }
        } catch (e) {
          s.fehler = fehlertext(e);
        }
      }}
    /></label
  >{/if}
{#if meldung}<p role="status">{meldung}</p>{/if}
<Fachliste
  titel="Abstandsflächen im Projekt"
  {s}
  bereich="abstand"
  darf={darf &&
    (katalog.some((r) => r.land === projekt.bundesland) || !!s.eingabe)}
  lesendBedienbar
  zusatzBeschaeftigt={() => busy}
>
  {#snippet children(p: AbstandsPlan)}
    <nav aria-label="Abstandsflächenbereiche">
      {#each [["grundlagen", "Grundlagen"], ["umrisse", "Gebäudeumrisse"], ["waende", "Wände / Bauteile"], ["lageplan", "Lageplan / Grenzen"], ["sicherung", "Sonderfälle / Sicherungen"], ["pruefung", "Prüfbericht"]] as [id, t]}<button
          type="button"
          class:sekundaer={bereich !== id}
          aria-pressed={bereich === id}
          onclick={() => (bereich = id)}>{t}</button
        >{/each}
    </nav>
    <fieldset disabled={!darf || busy} class="eingaben">
      {#if bereich === "grundlagen"}
        <h3>Rechts- und Datengrundlagen</h3>
        <div class="raster">
          <label>Bezeichnung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Bezeichnung" required bind:value={p.bezeichnung} /></label
          ><label>Gemeinde<input bind:value={p.recht.gemeinde} /></label><label
            >Konkrete LBO-Fassung<input bind:value={p.recht.fassung} /></label
          ><label
            >Baugebiet<select bind:value={p.recht.gebiet}
              >{#each ["WA", "WR", "WB", "WS", "MI", "MD", "MDW", "MU", "MK", "SO", "GE", "GI"] as g}<option
                  >{g}</option
                >{/each}</select
            ></label
          >
        </div>
        <label
          >Anwendbarkeit / Übergangsrecht<textarea
            bind:value={p.recht.anwendung}
            rows="2"
          ></textarea></label
        ><label
          >Örtliche Satzung: Ergebnis und Quelle der Prüfung<textarea
            bind:value={p.recht.satzung}
            rows="2"
          ></textarea></label
        ><label
          >Planfestsetzungen / Grenzbebauung: Prüfung<textarea
            bind:value={p.recht.planfestsetzung}
            rows="2"
          ></textarea></label
        >
        <div class="raster">
          <label
            >Gelände-/Höhenbezug<input
              bind:value={p.hoehenbezug}
              placeholder="Maßgebliches Gelände, Höhenbezug und Herkunft"
            /></label
          ><label
            >Koordinatenbezug<input
              bind:value={p.koordinatenbezug}
              placeholder="Lokales System oder Referenzsystem; Einheit Meter"
            /></label
          >
        </div>
        <h4>Regelpaket {p.regeln.id} · {p.regeln.version}</h4>
        <p class="hinweisbox">{p.regeln.hinweis}</p>
        <label
          >Eigenes versioniertes Regelpaket laden (JSON)<input
            type="file"
            accept=".json,application/json"
            onchange={(e) => eigeneRegeln(e, p)}
          /></label
        >
        <details>
          <summary>Konfigurierte Tiefen-/Dachregeln ansehen</summary>
          <pre>{JSON.stringify(p.regeln, null, 2)}</pre>
        </details>
        <label
          >Prüfvermerk zur Rechtsanwendung<textarea
            rows="3"
            bind:value={p.recht.pruefvermerk}
          ></textarea></label
        ><label
          ><input
            type="checkbox"
            disabled={!darfFreigeben}
            bind:checked={p.recht.geprueft}
          />Rechtsfassung, lokale Regeln und sämtliche verwendeten
          Regelbedingungen fachlich geprüft</label
        >
        <h4>Originalquellen und Planversionen</h4>
        <Dokumentliste {dateien} bind:werte={p.quellen} />
        <Planquellen projektId={projekt.id} plan={p} onplan={neu=>s.eingabe=neu as AbstandsPlan} onbusy={b=>busy=b}/>
      {:else if bereich === "umrisse"}
        <h3>Gebäudeumrisse</h3>
        <p class="meta">
          Eckpunkte des Gebäudes in Umlaufrichtung. Jede Seite wird zu einer
          Wand; die Außenseite ergibt sich aus der Umlaufrichtung. Geländehöhen
          an den Ecken gelten für beide angrenzenden Seiten. Zwischenpunkte,
          Wandoberkante, Dach und Sonderfälle je Seite unter „Wände / Bauteile“
          ergänzen.
        </p>
        <button type="button" class="sekundaer" onclick={() => umrissNeu(p)}>Gebäudeumriss hinzufügen</button>
        {#each p.umrisse ?? [] as u, i (u.id)}<fieldset>
            <legend>U{i + 1} · {u.bezeichnung}</legend>
            <div class="raster">
              <label>Bezeichnung<input bind:value={u.bezeichnung} /></label>
              {#if gebaeude.length}<label
                  >Gebäude aus Gebäude/Geschosse<select
                    value={u.gebaeude_id ?? ""}
                    onchange={(e) => {
                      const g = gebaeude.find((g) => g.objekt === e.currentTarget.value);
                      u.gebaeude_id = g?.objekt;
                      if (g) u.bezeichnung = g.daten.bezeichnung;
                    }}
                    ><option value="">Nicht verknüpft</option
                    >{#each gebaeude as g (g.objekt)}<option value={g.objekt}>{g.daten.bezeichnung}</option
                      >{/each}</select
                  ></label
                >{/if}
              <label
                >Grundstückseinheit<select bind:value={u.grundstueck_id}
                  ><option value={null}>Nur Tiefenvorprüfung / noch ungeklärt</option
                  >{#each p.grundstuecke as g}<option value={g.id}>{g.bezeichnung}</option>{/each}</select
                ></label
              >
              <label
                >Vorgabe Wandoberkante (m, z. B. Traufe)<input
                  type="number"
                  step="any"
                  bind:value={u.oberkante_m}
                /></label
              >
            </div>
            {#if u.gebaeude_id}<button type="button" class="sekundaer" onclick={() => traufeUebernehmen(u)}
                >Traufe aus Gebäude übernehmen</button
              >{#if hoehenHinweis}<p class="meta" role="status">{hoehenHinweis}</p>{/if}{/if}
            <label>Quelle der Umriss- und Höhenangaben<input bind:value={u.quelle} /></label>
            <table class="ergebnistabelle">
              <thead><tr><th>Ecke</th><th>X (m)</th><th>Y (m)</th><th>Gelände (m)</th><th></th></tr></thead>
              <tbody
                >{#each u.ecken as e, k}<tr
                    ><td>{k + 1}</td><td
                      ><input aria-label={`U${i + 1} Ecke ${k + 1} X`} type="number" step="any" bind:value={e.x} /></td
                    ><td
                      ><input aria-label={`U${i + 1} Ecke ${k + 1} Y`} type="number" step="any" bind:value={e.y} /></td
                    ><td
                      ><input
                        aria-label={`U${i + 1} Ecke ${k + 1} Gelände`}
                        type="number"
                        step="any"
                        value={e.gelaende_m ?? ""}
                        oninput={(ev) => {
                          const v = ev.currentTarget.value;
                          e.gelaende_m = v === "" ? undefined : Number(v);
                        }}
                      /></td
                    ><td
                      ><button type="button" class="link" onclick={() => (u.ecken = u.ecken.filter((_, n) => n !== k))}
                        >Ecke entfernen</button
                      ></td
                    ></tr
                  >{/each}</tbody
              >
            </table>
            <div class="knoepfe links">
              <button type="button" class="sekundaer" onclick={() => (u.ecken = [...u.ecken, { x: 0, y: 0 }])}
                >Ecke hinzufügen</button
              ><button type="button" class="sekundaer" onclick={umrisseEinbinden} disabled={u.ecken.length < 3}
                >Seiten als Wände übernehmen</button
              ><button
                type="button"
                class="link"
                onclick={async () => {
                  p.umrisse = (p.umrisse ?? []).filter((x) => x.id !== u.id);
                  await umrisseEinbinden();
                }}>Umriss mit seinen Wänden entfernen</button
              >
            </div>
            <Punktliste
              werte={u.ecken.map((e) => ({ x: e.x, y: e.y }))}
              label={`U${i + 1}: Ecken aus DXF-Kontur ersetzen`}
              {dateien}
              projektId={projekt.id}
              nurImport
              onwerte={(punkte) => (u.ecken = punkte.map((q) => ({ x: q.x, y: q.y })))}
              onquelle={(d) => {
                if (!p.quellen.some((q) => q.dokument_id === d.dokument_id && q.version === d.version))
                  p.quellen.push(d);
              }}
            />
          </fieldset>{/each}
        {#each umrissHinweise as h}<p class="hinweisbox" role="status">{h}</p>{/each}
        <Planbild plan={p} ergebnis={aktuell ? ergebnis : null} bind:auswahl />
      {:else if bereich === "waende"}
        <h3>Außenwände und Abschnitte</h3>
        <button type="button" class="sekundaer" onclick={() => wandNeu(p)}
          >Wand hinzufügen</button
        >
        <table class="ergebnistabelle">
          <thead
            ><tr
              ><th>Nr.</th><th>Gebäude</th><th>Wand</th><th>Fall</th><th
              ></th></tr
            ></thead
          ><tbody
            >{#each p.waende as w, i}<tr
                ><td>W{i + 1}</td><td>{w.gebaeude}</td><td
                  >{w.bezeichnung}{#if w.umriss}<span class="meta"> · aus Umriss</span>{/if}</td
                ><td>{w.fall}</td><td
                  ><button
                    type="button"
                    class="link"
                    onclick={() => (auswahl = w.id)}>Bearbeiten</button
                  ><button
                    type="button"
                    class="link"
                    onclick={() => {
                      p.waende = p.waende.filter((a) => a.id !== w.id);
                      // Verknüpfungen der entfernten Wand entfallen mit ihr.
                      if (p.planbindungen)
                        p.planbindungen = p.planbindungen.filter((b) => !b.ziel.startsWith(`waende.${w.id}.`));
                    }}
                    >Entfernen</button
                  ></td
                ></tr
              >{/each}</tbody
          >
        </table>
        {#if wand}{@const w = wand}
          <h4>{w.bezeichnung}</h4>
          {#if w.umriss}<p class="meta">
              Seite {w.umriss.seite + 1} des Umrisses
              „{(p.umrisse ?? []).find((u) => u.id === w.umriss?.umriss_id)?.bezeichnung ?? "?"}“: Lage,
              Außenseite, Gebäude, Grundstück und Eckhöhen werden unter
              „Gebäudeumrisse“ geführt.
            </p>{/if}
          <div class="raster">
            {#if gebaeude.length}<label
                >Gebäude aus Gebäude/Geschosse<select
                  disabled={!!w.umriss}
                  value={w.gebaeude_id ?? ""}
                  onchange={(e) => {
                    const g = gebaeude.find((g) => g.objekt === e.currentTarget.value);
                    w.gebaeude_id = g?.objekt;
                    if (g) w.gebaeude = g.daten.bezeichnung;
                  }}
                  ><option value="">Nicht verknüpft</option
                  >{#each gebaeude as g (g.objekt)}<option value={g.objekt}
                      >{g.daten.bezeichnung}</option
                    >{/each}</select
                ></label
              >{/if}
            <label>Gebäude<input bind:value={w.gebaeude} readonly={!!w.gebaeude_id || !!w.umriss} /></label>{#if w.gebaeude_id && !w.umriss}<div>
                <button type="button" class="sekundaer" onclick={() => hoehenUebernehmen(w)}
                  >Gelände und Traufe aus Gebäude übernehmen</button
                >{#if hoehenHinweis}<p class="meta" role="status">{hoehenHinweis}</p>{/if}
              </div>{/if}<label
              >Wandbezeichnung<input bind:value={w.bezeichnung} /></label
            ><label
              >Grundstückseinheit<select disabled={!!w.umriss} bind:value={w.grundstueck_id}
                ><option value={null}
                  >Nur Tiefenvorprüfung / noch ungeklärt</option
                >{#each p.grundstuecke as g}<option value={g.id}
                    >{g.bezeichnung}</option
                  >{/each}</select
              ></label
            ><label
              >Außenseite in Richtung Anfang → Ende<select disabled={!!w.umriss} bind:value={w.aussen}
                ><option value="rechts">Rechts</option><option value="links"
                  >Links</option
                ></select
              ></label
            >
          </div>
          <div class="raster">
            <label
              >Anfang X (m)<input
                type="number"
                step="any"
                readonly={!!w.umriss}
                bind:value={w.von.x}
              /></label
            ><label
              >Anfang Y (m)<input
                type="number"
                step="any"
                readonly={!!w.umriss}
                bind:value={w.von.y}
              /></label
            ><label
              >Ende X (m)<input
                type="number"
                step="any"
                readonly={!!w.umriss}
                bind:value={w.bis.x}
              /></label
            ><label
              >Ende Y (m)<input
                type="number"
                step="any"
                readonly={!!w.umriss}
                bind:value={w.bis.y}
              /></label
            >
          </div>
          {#if w.gebaeude_id && w.umriss}<div>
              <button type="button" class="sekundaer" onclick={() => hoehenUebernehmen(w)}
                >Traufe aus Gebäude als Wandoberkante übernehmen</button
              >{#if hoehenHinweis}<p class="meta" role="status">{hoehenHinweis}</p>{/if}
            </div>{/if}
          <p class="meta">
            Wandhöhenansatz des Regelpakets: {p.regeln.wandhoehe === "mittel_kanten"
              ? "im Mittel gemessene Wandhöhe aus den Wandhöhen an den Kanten je Wandteil"
              : p.regeln.wandhoehe === "mittel_profil"
                ? "flächengemittelte Wandhöhe je Wandteil"
                : p.regeln.wandhoehe === "punktweise"
                  ? "Wandhöhe an jeder Stelle, Abstandsfläche mit wechselnder Tiefe"
                  : "größte Wandhöhe je Wandteil (konservativ)"}.
          </p>
          {#key w.id}<Hoehenprofil
              wand={w}
              planGelaende={p.planbindungen?.some(b=>b.ziel.startsWith(`waende.${w.id}.gelaende_`))??false}
              planOberkante={p.planbindungen?.some(b=>b.ziel.startsWith(`waende.${w.id}.abschluss_`))??false}
              {dateien}
              projektId={projekt.id}
              onquelle={(d) => {
                if (!p.quellen.some((q) => q.dokument_id === d.dokument_id && q.version === d.version))
                  p.quellen.push(d);
              }}
            />{/key}
          <label
            >Gelände-/Höhenherleitung und Planquelle<textarea
              bind:value={w.hoehenquelle}
              readonly={p.planbindungen?.some(b=>b.ziel.startsWith(`waende.${w.id}.`))}
              rows="2"
            ></textarea></label
          >
          <label
            >Dach-/Giebelansatz<select bind:value={w.dachart}
              ><option value="ungeklaert">Noch ungeklärt</option><option
                value="flach">Flachdach / kein zusätzlicher Anteil</option
              ><option value="dachflaeche"
                >Dachfläche nach konfigurierter Neigungsregel</option
              ><option value="giebel"
                >{p.regeln.giebel ? "Giebelfläche nach Regelpaket" : "Giebel – fachlich hergeleiteter Anteil"}</option
              ><option value="manuell"
                >Besonderer Dachanteil mit Grundlage</option
              ></select
            ></label
          >
          {#if w.dachart !== "flach" && w.dachart !== "ungeklaert"}<div
              class="raster"
            >
              <label
                >{w.dachart === "giebel" ? "Giebelhöhe (Traufe bis First, m)" : "Dach-/Giebelhöhe (m)"}<input
                  type="number"
                  step="any"
                  min="0"
                  bind:value={w.dachhoehe_m}
                  readonly={p.planbindungen?.some(b=>b.ziel===`waende.${w.id}.dachhoehe_m`)}
                /></label
              >{#if w.dachart === "dachflaeche"}<label
                  >Dachneigung (Grad)<input
                    type="number"
                    min="0"
                    max="90"
                    step="any"
                    bind:value={w.dachneigung_grad}
                    readonly={p.planbindungen?.some(b=>b.ziel===`waende.${w.id}.dachneigung_grad`)}
                  /></label
                >{#if p.regeln.gauben}<label
                    >Gesamtlänge Gauben/Dachaufbauten auf dieser Dachfläche (m)<input
                      type="number"
                      min="0"
                      step="any"
                      value={w.gauben_laenge_m ?? ""}
                      oninput={(e) => {
                        const v = e.currentTarget.value;
                        w.gauben_laenge_m = v === "" ? undefined : Number(v);
                      }}
                    /></label
                  >{/if}{:else if w.dachart === "giebel" && p.regeln.giebel}<label
                  >Neigung Dachseite 1 (Grad)<input
                    type="number"
                    min="0"
                    max="90"
                    step="any"
                    bind:value={w.dachneigung_grad}
                    readonly={p.planbindungen?.some(b=>b.ziel===`waende.${w.id}.dachneigung_grad`)}
                  /></label
                ><label
                  >Neigung Dachseite 2 (Grad)<input
                    type="number"
                    min="0"
                    max="90"
                    step="any"
                    value={w.dachneigung_b_grad ?? ""}
                    readonly={p.planbindungen?.some(b=>b.ziel===`waende.${w.id}.dachneigung_b_grad`)}
                    oninput={(e) => {
                      const v = e.currentTarget.value;
                      w.dachneigung_b_grad = v === "" ? undefined : Number(v);
                    }}
                  /></label
                >{:else}<label
                  >Anrechnungsfaktor<input
                    type="number"
                    min="0"
                    step="any"
                    bind:value={w.dachfaktor_manuell}
                  /></label
                ><label
                  >Rechtliche / geometrische Herleitung<input
                    bind:value={w.dachgrundlage}
                  /></label
                >{/if}
            </div>{/if}
          <label
            >Bauteil-/Wandfall<select bind:value={w.fall}
              >{#each [["regel", "Regelwand"], ["garage", "Garage"], ["carport", "Carport"], ["nebengebaeude", "Nebengebäude ohne Aufenthaltsräume"], ["vorbau", "Vorbau / Balkon / Erker"], ["grenzbebauung", "Grenzbebauung"], ["bestand", "Bestand / Änderung"], ["sonderbau", "Sonderbau"], ["sonstiges", "Sonstiger Sonderfall"]] as [id, t]}<option
                  value={id}>{t}</option
                >{/each}</select
            ></label
          >
          <label
            >Tiefenfall<select
              value={w.tiefenfall ?? "regel"}
              onchange={(e) => {
                const v = e.currentTarget.value as NonNullable<AbstandsWand["tiefenfall"]>;
                w.tiefenfall = v === "regel" ? undefined : v;
              }}
              ><option value="regel">Regeltiefe</option><option value="oeffentliche_flaeche"
                >Zu öffentlicher Verkehrs-, Grün- oder Wasserfläche</option
              ><option value="wohngebaeude_gk12"
                >Wohngebäude GK 1/2, höchstens drei oberirdische Geschosse</option
              ></select
            ></label
          >{#if w.tiefenfall}<label
              >Begründung des Tiefenfalls<input
                value={w.tiefenfall_grundlage ?? ""}
                oninput={(e) => (w.tiefenfall_grundlage = e.currentTarget.value)}
              /></label
            >{/if}
          {#if w.fall === "garage" || w.fall === "carport" || w.fall === "nebengebaeude"}<label
              >Gemeinsame Grenzkennung<input
                bind:value={w.grenzkennung}
              /></label
            ><label
              ><input
                type="checkbox"
                bind:checked={w.nutzung_geprueft}
              />Gesamte Anlage auf Nutzung, Höhe und weitere
              Privilegierungsbedingungen geprüft; erfasste Längen je
              Anlage/Grenze ohne Doppelzählung</label
            >{/if}
          {#if w.fall !== "regel"}<p class="meta">
              Sonderfall ohne passenden automatisierten Regeltatbestand bleibt
              offen. Einen wirksamen besonderen Ansatz mit Beleg können Sie
              unter Sonderfälle / Sicherungen erfassen.
            </p>{/if}
        {/if}
      {:else if bereich === "lageplan"}
        <h3>Grundstücksgrenzen</h3>
        <p class="meta">
          Maßgebliche rechtliche Grundstückseinheiten und Grenzen, unabhängig
          von GRZ-/GFZ-Flächenabzügen. Nicht automatisch über interne
          Flurstücksgrenzen zusammenfassen.
        </p>
        <button
          type="button"
          class="sekundaer"
          onclick={() =>
            p.grundstuecke.push({
              id: crypto.randomUUID(),
              bezeichnung: "",
              flurstueck_ids: [],
              grenze: [],
              innenringe: [],
              grundlage: "",
            })}>Grundstückseinheit hinzufügen</button
        >
        {#each p.grundstuecke as g, i}<fieldset>
            <legend>G{i + 1}</legend><label
              >Grundstücksbezeichnung<input bind:value={g.bezeichnung} /></label
            ><label
              >Grenzherkunft und rechtliche Zuordnung<textarea
                bind:value={g.grundlage}
                rows="2"
              ></textarea></label
            >
            <div class="knoepfe links">
              {#each flurstuecke as f (f.id)}<label
                  ><input
                    type="checkbox"
                    checked={g.flurstueck_ids.includes(f.id)}
                    onchange={(e) =>
                      (g.flurstueck_ids = e.currentTarget.checked
                        ? [...g.flurstueck_ids, f.id]
                        : g.flurstueck_ids.filter((id) => id !== f.id))}
                  />{f.gemarkung} · {f.nummer}</label
                >{/each}
              {#each g.flurstueck_ids.filter((id) => !flurstuecke.some((f) => f.id === id)) as id (id)}<label
                  class="veraltet"
                  ><input
                    type="checkbox"
                    checked
                    onchange={() =>
                      (g.flurstueck_ids = g.flurstueck_ids.filter((x) => x !== id))}
                  />Nicht mehr erfasstes Flurstück – Haken entfernen</label
                >{/each}
            </div>
            <p class="meta">
              Flurstück-IDs: {g.flurstueck_ids.join(", ") ||
                "Über die dokumentierte Grundstückszuordnung festlegen"}
            </p>
            <Punktliste
              bind:werte={g.grenze}
              label={`G${i + 1} Grenze`}
              {dateien}
              projektId={projekt.id}
              onquelle={(d) => {
                if (
                  !p.quellen.some(
                    (q) =>
                      q.dokument_id === d.dokument_id &&
                      q.version === d.version,
                  )
                )
                  p.quellen.push(d);
              }}
            />
            <details>
              <summary>Rechtlich ausgeschlossene Innenringe</summary>
              <p class="meta">
                Nur tatsächlich nicht zugehörige Grundstücksflächen erfassen,
                keine rechnerischen GRZ-/GFZ-Abzüge.
              </p>
              <button
                type="button"
                class="sekundaer"
                onclick={() => {
                  g.innenringe ??= [];
                  g.innenringe.push([]);
                }}>Innenring hinzufügen</button
              >{#each g.innenringe ?? [] as _, j}<Punktliste
                  bind:werte={g.innenringe[j]}
                  label={`G${i + 1} Innenring ${j + 1}`}
                /><button
                  type="button"
                  class="link"
                  onclick={() =>
                    (g.innenringe = g.innenringe.filter((_, n) => n !== j))}
                  >Innenring entfernen</button
                >{/each}
            </details>
            <button
              type="button"
              class="link"
              onclick={() =>
                (p.grundstuecke = p.grundstuecke.filter((a) => a.id !== g.id))}
              >Grundstückseinheit entfernen</button
            >
          </fieldset>{/each}
        <Planbild plan={p} ergebnis={aktuell ? ergebnis : null} bind:auswahl />
      {:else if bereich === "sicherung"}
        <h3>Sicherungsflächen, öffentliche Teilflächen und Baukörper</h3>
        <p class="meta">
          Nur die fachlich geprüfte zulässige Teilfläche einer öffentlichen
          Fläche erfassen (z.B. soweit anwendbar bis zur Mitte).
          Nachbarzustimmung allein ersetzt keine erforderliche Sicherung. Andere
          Gebäude als Baukörper erfassen; die eigene Ausgangswand nicht als
          Hindernis eintragen.
        </p>
        <button
          type="button"
          class="sekundaer"
          disabled={!p.grundstuecke.length}
          onclick={() =>
            p.flaechen.push({
              id: crypto.randomUUID(),
              grundstueck_id: p.grundstuecke[0].id,
              art: "sicherung",
              bezeichnung: "",
              polygon: [],
              grundlage: "",
              wirksam: false,
              beleg: null,
            })}>Zusätzliche Fläche erfassen</button
        >
        {#each p.flaechen as f, i}<fieldset>
            <legend>F{i + 1}</legend>
            <div class="raster">
              <label
                >Flächenbezeichnung<input bind:value={f.bezeichnung} /></label
              ><label
                >Flächenart<select bind:value={f.art}
                  ><option value="sicherung">Rechtliche Sicherungsfläche</option
                  ><option value="oeffentliche_flaeche"
                    >Zulässige öffentliche Teilfläche</option
                  ><option value="baukoerper">Anderer Baukörper</option></select
                ></label
              ><label
                >Begünstigte Grundstückseinheit<select
                  bind:value={f.grundstueck_id}
                  >{#each p.grundstuecke as g}<option value={g.id}
                      >{g.bezeichnung}</option
                    >{/each}</select
                ></label
              >
            </div>
            <label
              >Geltungsumfang / rechtliche Grundlage<textarea
                bind:value={f.grundlage}
                rows="2"
              ></textarea></label
            ><Dokumentfeld
              {dateien}
              bind:wert={f.beleg}
              label="Sicherungs-/Planbeleg"
            /><label
              ><input type="checkbox" bind:checked={f.wirksam} />Wirksamkeit und
              Anwendungsumfang fachlich geprüft</label
            ><Punktliste
              bind:werte={f.polygon}
              label={`F${i + 1} Umriss`}
              {dateien}
              projektId={projekt.id}
              onquelle={(d) => {
                if (
                  !p.quellen.some(
                    (q) =>
                      q.dokument_id === d.dokument_id &&
                      q.version === d.version,
                  )
                )
                  p.quellen.push(d);
              }}
            /><button
              type="button"
              class="link"
              onclick={() =>
                (p.flaechen = p.flaechen.filter((a) => a.id !== f.id))}
              >Fläche entfernen</button
            >
          </fieldset>{/each}
        <h4>Besondere Wandansätze und Entscheidungen</h4>
        {#each p.waende as w}<details>
            <summary>{w.gebaeude} / {w.bezeichnung}</summary
            >{#if w.sonderansatz}{@const a = w.sonderansatz}<label
                >Belegte besondere Tiefe (m)<input
                  type="number"
                  min="0"
                  step="any"
                  bind:value={a.tiefe_m}
                /></label
              ><label
                >Entscheidung / Rechtsgrundlage und Geltungsumfang<textarea
                  rows="3"
                  bind:value={a.grundlage}
                ></textarea></label
              ><Dokumentfeld
                {dateien}
                bind:wert={a.beleg}
                label="Entscheidungs-/Sondernachweis"
              /><label
                ><input type="checkbox" bind:checked={a.wirksam} />Genehmigt
                bzw. fachlich wirksam und für diese Wand anwendbar</label
              ><button
                type="button"
                class="link"
                onclick={() => (w.sonderansatz = null)}
                >Besonderen Ansatz entfernen</button
              ><label
                ><input
                  type="checkbox"
                  checked={w.polygon_manuell !== null}
                  onchange={(e) =>
                    (w.polygon_manuell = e.currentTarget.checked ? [] : null)}
                />Eigenes dokumentiertes Abstandsflächenpolygon</label
              >{#if w.polygon_manuell}<Punktliste
                  bind:werte={w.polygon_manuell}
                  label="Besonderes Abstandsflächenpolygon"
                />{/if}{:else}<button
                type="button"
                class="sekundaer"
                onclick={() =>
                  (w.sonderansatz = {
                    tiefe_m: 0,
                    grundlage: "",
                    wirksam: false,
                    beleg: null,
                  })}>Besonderen Ansatz erfassen</button
              >{/if}
          </details>{/each}
      {:else}
        <h3>Prüfumfang und Ergebnisse</h3>
        <label
          >Vermerk zu Vollständigkeit, Ecken, Geländeprofilen, Grenztypen und
          Sonderfällen<textarea rows="3" bind:value={p.umfang_vermerk}
          ></textarea></label
        ><label
          ><input
            type="checkbox"
            disabled={!darfFreigeben}
            bind:checked={p.umfang_geprueft}
          />Alle maßgeblichen Wände, Geländeabschnitte, Ecken und besonderen
          Verhältnisse fachlich geprüft</label
        >
      {/if}
    </fieldset>
    <div class="knoepfe links">
      <button type="button" disabled={busy || s.laeuft} onclick={pruefen}
        >Abstandsflächen berechnen</button
      ><button
        type="button"
        class="sekundaer"
        disabled={!darf || busy || s.laeuft}
        onclick={nachweis}>Nachweisstand archivieren</button
      ><Druckknopf
        projektId={projekt.id}
        darfAblegen={darf}
        disabled={!darf || busy || s.laeuft}
        text="Nachweis archivieren und PDF ausgeben …"
        vorbereiten={async () => {
          const n = await nachweis();
          return n ? { art: "nachweis", id: n.id } : null;
        }}
      /><button
        type="button"
        class="sekundaer"
        disabled={!darf || busy || s.laeuft}
        onclick={() => exportieren("json")}
        >Nachweis als JSON exportieren</button
      ><button
        type="button"
        class="sekundaer"
        disabled={!darf || busy || s.laeuft}
        onclick={() => exportieren("dxf")}>Geometrie als DXF exportieren</button
      >
    </div>
    {#if ergebnis && ergebnisStand.startsWith(JSON.stringify( [s.id], ).slice(0, -1))}{#if !aktuell}<p
          role="status"
          class="hinweisbox"
        >
          Ergebnis veraltet – Eingaben geändert. Erneut berechnen.
        </p>{/if}
      <section aria-label="Ergebnis Abstandsflächen">
        <h3>Berechnungsergebnis</h3>
        <p>
          {ergebnis.konfliktfrei
            ? "Geometrisch eingehalten; fachliche Freigabe gesondert erforderlich"
            : "Offene Prüfungen oder geometrische Konflikte vorhanden"}
        </p>
        {#each ergebnis.offen as h}<p class="hinweisbox">{h}</p>{/each}
        <table class="ergebnistabelle">
          <thead
            ><tr
              ><th>Wand</th><th>H (m)</th><th>Tiefe (m)</th><th>Fläche (m²)</th
              ><th>Außerhalb (m²)</th><th>Status</th></tr
            ></thead
          ><tbody
            >{#each ergebnis.waende as w}<tr
                ><td
                  ><button
                    type="button"
                    class="link"
                    onclick={() => {
                      auswahl = w.id;
                      bereich = "waende";
                    }}>{w.bezeichnung}</button
                  ></td
                ><td>{z(w.h_m)}</td><td>{z(w.tiefe_m)}</td><td
                  >{z(w.flaeche_m2)}</td
                ><td>{z(w.ausserhalb_m2)}</td><td>{w.status}</td></tr
              >{/each}</tbody
          >
        </table>
        {#each ergebnis.waende as w}<details>
            <summary>{w.bezeichnung} – Herleitung und offene Punkte</summary
            ><Abwicklung ergebnis={w} />{#each w.herleitung as h}<p class:hinweisbox={h.startsWith("Hinweis")}>{h}</p>{/each}{#each w.offen as h}<p
                class="hinweisbox"
              >
                {h}
              </p>{/each}
          </details>{/each}{#each ergebnis.ueberdeckungen as u}<p class:hinweisbox={!u.zulaessig}>
            Überdeckung {ergebnis.waende.find((w) => w.id === u.wand_a)?.bezeichnung} /
            {ergebnis.waende.find((w) => w.id === u.wand_b)?.bezeichnung}: {z(u.flaeche_m2)} m²{u.winkel_grad != null
              ? `, Winkel ${z(u.winkel_grad)}°`
              : ""} – {u.zulaessig ?? "Zulässigkeit gesondert prüfen"}
          </p>{/each}<Planbild
          plan={p}
          ergebnis={aktuell ? ergebnis : null}
          bind:auswahl
        />
      </section>{/if}
  {/snippet}
</Fachliste>

<style>
  .veraltet { color: var(--warning-text); font-style: italic; }
  nav {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    margin: 1rem 0;
  }
  .eingaben {
    border: 0;
    padding: 0;
    min-width: 0;
  }
  .eingaben :global(label) {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin: 0.5rem 0;
  }
  .eingaben :global(label:has(input[type="checkbox"])) {
    flex-direction: row;
    align-items: start;
  }
  .eingaben :global(fieldset) {
    margin: 1rem 0;
    min-width: 0;
  }
  pre {
    white-space: pre-wrap;
    max-height: 24rem;
    overflow: auto;
  }
  h4 {
    margin-top: 1.2rem;
  }
</style>
