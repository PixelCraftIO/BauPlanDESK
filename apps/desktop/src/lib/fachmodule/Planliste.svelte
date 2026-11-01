<script lang="ts">
  import { untrack } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    type Project,
    type DocumentVersion,
  } from "../api";
  import type {
    Planart,
    Planeintrag,
    Planliste,
    Planzeile,
  } from "../generated/plattform";
  import { plattformApi } from "../plattformApi";
  import { ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  import { editorAnmelden } from "../editoren";
  let {
    projekt,
    darf,
  }: { projekt: Project; benutzerId: string; darf: boolean } = $props();
  const ARTEN: [Planart, string, string][] = [
    ["lageplan", "LP", "Lageplan"],
    ["grundriss", "GR", "Grundriss"],
    ["schnitt", "SN", "Schnitt"],
    ["ansicht", "AN", "Ansicht"],
    ["detail", "DT", "Detail"],
    ["entwaesserung", "EW", "Entwässerungsplan"],
    ["freiflaechen", "FF", "Freiflächenplan"],
    ["brandschutz", "BS", "Brandschutzplan"],
    ["sonstiges", "SO", "Sonstiger Plan"],
  ];
  const artName = (a: Planart) => ARTEN.find(([k]) => k === a)?.[2] ?? a;
  const STATUS: Record<string, string> = {
    entwurf: "Entwurf",
    eingereicht: "Eingereicht",
    nachgereicht: "Nachgereicht",
    genehmigt: "Genehmigt",
    ersetzt: "Ersetzt",
  };
  const heute = () => {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  };
  const datum = (d: string) =>
    new Date(d + "T12:00:00").toLocaleDateString("de-DE");
  let liste = $state<Planliste | null>(null),
    dateien = $state<DocumentVersion[]>([]),
    fehler = $state<string | null>(null),
    meldung = $state<string | null>(null),
    laeuft = $state(false),
    eingabe = $state<Planeintrag | null>(null),
    original = $state<string>(""),
    basis = $state(0),
    neuerIndex = $state({ index: "", aenderung: "" }),
    neuerVersand = $state({
      empfaenger: "",
      index: "",
      weg: "E-Mail",
      bemerkung: "",
      datum: heute(),
    });
  const geaendert = $derived(!!eingabe && JSON.stringify(eingabe) !== original);
  const zeile = $derived(
    liste?.zeilen.find((z) => z.eintrag.dokument_id === eingabe?.dokument_id) ??
      null,
  );
  const frei = $derived(
    dateien.filter(
      (d) =>
        /\.(pdf|dxf|ifc)$/i.test(d.file_name) &&
        !liste?.zeilen.some((z) => z.eintrag.dokument_id === d.document_id),
    ),
  );
  const naechste = (a: Planart) =>
    Math.max(
      0,
      ...(liste?.zeilen
        .filter((z) => z.eintrag.planart === a)
        .map((z) => z.eintrag.nummer) ?? []),
    ) + 1;
  /** Nächster Büroindex: a, b, … z, aa … (wie der bisherige, sonst klein). */
  function folgeindex(z: Planzeile | null) {
    const letzter = eingabe?.revisionen.at(-1)?.index ?? z?.index?.index;
    if (!letzter) return "a";
    if (/^\d+$/.test(letzter))
      return String(Number(letzter) + 1).padStart(letzter.length, "0");
    const gross = letzter === letzter.toUpperCase();
    const s = letzter.toLowerCase().split("");
    let i = s.length - 1;
    while (i >= 0 && s[i] === "z") s[i--] = "a";
    if (i < 0) s.unshift("a");
    else s[i] = String.fromCharCode(s[i].charCodeAt(0) + 1);
    const r = s.join("");
    return gross ? r.toUpperCase() : r;
  }
  async function laden() {
    laeuft = true;
    fehler = null;
    try {
      [liste, dateien] = await Promise.all([
        api.planliste(projekt.id),
        api.dokumente(projekt.id),
      ]);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  ansichtsAbfrage(() => projekt.id, ["dokumente", "fachobjekte"], laden);
  function oeffnen(z: Planzeile) {
    if (geaendert) {
      meldung = "Bitte zuerst speichern oder verwerfen.";
      return;
    }
    eingabe = JSON.parse(JSON.stringify(z.eintrag));
    original = JSON.stringify(eingabe);
    basis = z.revision;
    meldung = null;
    neuerIndex = { index: folgeindex(z), aenderung: "" };
  }
  let neuDokument = $state("");
  function aufnehmen() {
    const d = dateien.find((d) => d.document_id === neuDokument);
    if (!d || geaendert) return;
    const art: Planart = /lage/i.test(d.file_name)
      ? "lageplan"
      : /schnitt/i.test(d.file_name)
        ? "schnitt"
        : /ansicht/i.test(d.file_name)
          ? "ansicht"
          : "grundriss";
    eingabe = {
      dokument_id: d.document_id,
      planart: art,
      geschoss: "",
      nummer: naechste(art),
      titel: d.file_name.replace(/\.[^.]+$/, ""),
      massstab: art === "lageplan" ? 500 : 100,
      plannummer: "",
      revisionen: [],
      verteiler: [],
    };
    original = "";
    basis = 0;
    neuerIndex = { index: "a", aenderung: "Erstausgabe" };
    neuDokument = "";
  }
  async function speichern() {
    if (!eingabe) return false;
    laeuft = true;
    fehler = null;
    try {
      const f = await plattformApi.speichern<Planeintrag>(
        projekt.id,
        "planliste",
        eingabe.dokument_id,
        basis,
        eingabe,
      );
      eingabe = f.daten;
      original = JSON.stringify(f.daten);
      basis = f.revision;
      meldung = `Gespeichert als ${f.daten.plannummer}.`;
      await laden();
      return true;
    } catch (e) {
      fehler = fehlertext(e);
      return false;
    } finally {
      laeuft = false;
    }
  }
  /** Plankopf (Projekt, Bauherr, Plannummer, Index) auf die aktuelle Dateiversion stempeln. */
  async function stempeln() {
    if (!eingabe || !zeile?.aktuelle_version) return;
    if (!zeile.dateiname.toLowerCase().endsWith(".pdf")) {
      fehler = "Der Plankopf lässt sich nur auf PDF-Pläne setzen.";
      return;
    }
    const ziel = await save({
      defaultPath: `${eingabe.plannummer || "Plan"} mit Plankopf.pdf`.replace(/[<>:"/\\|?*]/g, "_"),
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (!ziel) return;
    laeuft = true;
    try {
      await api.plankopfStempeln(projekt.id, { dokument_id: eingabe.dokument_id, version: zeile.aktuelle_version, seite: 1 }, ziel);
      meldung = `Plan mit Plankopf gespeichert: ${ziel}. Das Original im Projekt bleibt unverändert.`;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  function indexSetzen() {
    if (!eingabe || !neuerIndex.index.trim()) return;
    const version =
      zeile?.aktuelle_version ??
      Math.max(
        0,
        ...dateien
          .filter((d) => d.document_id === eingabe!.dokument_id)
          .map((d) => d.version),
      );
    eingabe.revisionen = [
      ...eingabe.revisionen,
      {
        index: neuerIndex.index.trim(),
        datum: heute(),
        aenderung: neuerIndex.aenderung.trim(),
        version,
      },
    ];
    neuerIndex = { index: folgeindex(zeile), aenderung: "" };
  }
  function versandErfassen() {
    if (!eingabe || !neuerVersand.empfaenger.trim()) return;
    eingabe.verteiler = [
      ...eingabe.verteiler,
      {
        id: crypto.randomUUID(),
        empfaenger: neuerVersand.empfaenger.trim(),
        datum: neuerVersand.datum,
        index: neuerVersand.index,
        weg: neuerVersand.weg,
        bemerkung: neuerVersand.bemerkung,
      },
    ];
    neuerVersand = { ...neuerVersand, empfaenger: "", bemerkung: "" };
  }
  editorAnmelden("planliste", {
    hatAenderungen: () => geaendert,
    istBeschaeftigt: () => laeuft,
    speichern,
  });
  $effect(() => {
    if (eingabe && !neuerVersand.index)
      untrack(() => (neuerVersand.index = eingabe?.revisionen.at(-1)?.index ?? ""));
  });
</script>

<section class="planliste">
  <h2>Planliste</h2>
  <p class="meta">
    Plannummer nach Büroschema <code>{liste?.schema.muster ?? "…"}</code> mit
    Projektkennzeichen „{liste?.kennzeichen ?? ""}“ (Projektorganisation). Der
    Büroindex wird hier fortgeschrieben; der Behördenindex A, B, C … ergibt
    sich aus den versendeten Abgaben der Verfahrensakte.
  </p>
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if meldung}<p class="meta" role="status">{meldung}</p>{/if}
  <div class="tabellen-scroll">
    <table class="ergebnistabelle">
      <thead
        ><tr
          ><th>Plannummer</th><th>Titel</th><th>Planart</th><th>Maßstab</th><th
            >Büroindex</th
          ><th>DMS</th><th>Behörde</th><th>Zuletzt weitergegeben</th></tr
        ></thead
      >
      <tbody>
        {#each liste?.zeilen ?? [] as z (z.eintrag.dokument_id)}<tr
            class:auswahl={eingabe?.dokument_id === z.eintrag.dokument_id}
            ><td
              ><button type="button" class="link" onclick={() => oeffnen(z)}
                >{z.eintrag.plannummer}</button
              ></td
            ><td>{z.eintrag.titel}</td><td>{artName(z.eintrag.planart)}</td><td
              >{z.eintrag.massstab ? `1:${z.eintrag.massstab}` : "—"}</td
            ><td
              >{z.index
                ? `${z.index.index} · ${datum(z.index.datum)}`
                : "—"}</td
            ><td
              >{z.aktuelle_version ? `V${z.aktuelle_version}` : "entfernt"}{#if z.ohne_index}<br
                /><span class="warnung">neuere Version ohne Index</span>{/if}</td
            ><td
              >{#each z.behoerde as b}{b.bezeichnung}: {b.planindex ?? "–"} · {STATUS[
                  b.status
                ]}<br />{:else}—{/each}</td
            ><td
              >{z.versand[0]
                ? `${z.versand[0].empfaenger} · ${datum(z.versand[0].datum)}`
                : "—"}</td
            ></tr
          >{:else}<tr
            ><td colspan="8" class="meta"
              >{laeuft
                ? "Wird geladen …"
                : "Noch keine Pläne in der Liste."}</td
            ></tr
          >{/each}
      </tbody>
    </table>
  </div>
  {#if darf}
    <div class="knoepfe links">
      <label
        >Plan aus den Projektdateien aufnehmen<select bind:value={neuDokument}
          ><option value="">Datei wählen …</option
          >{#each frei as d (d.document_id)}<option value={d.document_id}
              >{d.file_name} · V{d.version}</option
            >{/each}</select
        ></label
      >
      <button
        class="sekundaer"
        disabled={!neuDokument || geaendert}
        onclick={aufnehmen}>Aufnehmen</button
      >
    </div>
  {/if}

  {#if eingabe}
    <fieldset disabled={!darf || laeuft} class="planeintrag">
      <legend>{eingabe.plannummer || "Neuer Plan"}</legend>
      <div class="raster">
        <label
          >Titel<span class="pflicht" aria-hidden="true"> *</span><input
            bind:value={eingabe.titel}
            required
            maxlength="300"
          /></label
        ><label
          >Planart<select
            bind:value={eingabe.planart}
            onchange={() =>
              basis === 0 && eingabe && (eingabe.nummer = naechste(eingabe.planart))}
            >{#each ARTEN as [k, kz, t]}<option value={k}>{t} ({kz})</option
              >{/each}</select
          ></label
        ><label
          >Geschoss / Bereich<input
            bind:value={eingabe.geschoss}
            maxlength="30"
            placeholder="EG, OG1, KG …"
          /></label
        ><label
          >Laufende Nummer<input
            type="number"
            min="1"
            max="9999"
            bind:value={eingabe.nummer}
          /></label
        ><label
          >Maßstab 1:<input
            type="number"
            min="1"
            max="100000"
            value={eingabe.massstab ?? ""}
            onchange={(e) =>
              eingabe &&
              (eingabe.massstab = e.currentTarget.value
                ? Number(e.currentTarget.value)
                : null)}
          /></label
        >
      </div>
      <h3>Büroindex</h3>
      <table class="ergebnistabelle">
        <thead
          ><tr
            ><th>Index</th><th>Datum</th><th>Änderung</th><th>Version</th></tr
          ></thead
        >
        <tbody>
          {#each eingabe.revisionen as r}<tr
              ><td>{r.index}</td><td>{datum(r.datum)}</td><td>{r.aenderung}</td
              ><td>V{r.version}</td></tr
            >{:else}<tr><td colspan="4" class="meta">Noch kein Index.</td></tr
            >{/each}
        </tbody>
      </table>
      <div class="raster">
        <label
          >Neuer Index<input bind:value={neuerIndex.index} maxlength="10" /></label
        ><label
          >Änderung<input
            bind:value={neuerIndex.aenderung}
            maxlength="2000"
            placeholder="z. B. Dachneigung auf 38° angepasst"
          /></label
        >
      </div>
      <button type="button" class="sekundaer" onclick={indexSetzen}
        >Index für die aktuelle Dateiversion setzen</button
      >
      <button type="button" class="sekundaer" disabled={laeuft || geaendert || !zeile?.aktuelle_version} title={geaendert ? "Zuerst speichern" : "Plankopf unten rechts auf Seite 1 setzen"} onclick={() => void stempeln()}
        >Plankopf stempeln …</button
      >
      <h3>Planverteiler</h3>
      <table class="ergebnistabelle">
        <thead
          ><tr
            ><th>Datum</th><th>Empfänger</th><th>Index</th><th>Weg</th><th
              >Bemerkung</th
            ></tr
          ></thead
        >
        <tbody>
          {#each [...eingabe.verteiler, ...(zeile?.versand.filter((v) => !eingabe?.verteiler.some((e) => e.id === v.id)) ?? [])].sort( (a, b) => b.datum.localeCompare(a.datum), ) as v (v.id)}<tr
              ><td>{datum(v.datum)}</td><td>{v.empfaenger}</td><td
                >{v.index || "—"}</td
              ><td>{v.weg}</td><td>{v.bemerkung}</td></tr
            >{:else}<tr
              ><td colspan="5" class="meta">Noch nicht weitergegeben.</td></tr
            >{/each}
        </tbody>
      </table>
      <p class="meta">
        Abgaben an die Behörde erscheinen automatisch aus der Verfahrensakte.
      </p>
      <div class="raster">
        <label
          >Empfänger<input
            bind:value={neuerVersand.empfaenger}
            maxlength="300"
            placeholder="Bauherr, Statikbüro, Prüfingenieur …"
          /></label
        ><label
          >Datum<input
            type="date"
            max={heute()}
            bind:value={neuerVersand.datum}
          /></label
        ><label
          >Index<select bind:value={neuerVersand.index}
            ><option value="">ohne Index</option
            >{#each eingabe.revisionen as r}<option value={r.index}
                >{r.index}</option
              >{/each}</select
          ></label
        ><label
          >Weg<select bind:value={neuerVersand.weg}
            >{#each ["E-Mail", "Post", "Plattform", "Persönlich", "Sonstiges"] as w}<option
                >{w}</option
              >{/each}</select
          ></label
        ><label
          >Bemerkung<input
            bind:value={neuerVersand.bemerkung}
            maxlength="2000"
          /></label
        >
      </div>
      <button type="button" class="sekundaer" onclick={versandErfassen}
        >Weitergabe erfassen</button
      >
      <div class="knoepfe links">
        <button onclick={speichern} disabled={!geaendert}>Speichern</button>
        <button
          class="sekundaer"
          disabled={laeuft}
          onclick={() => {
            eingabe = null;
            original = "";
            meldung = null;
          }}>{geaendert ? "Verwerfen" : "Schließen"}</button
        >
      </div>
    </fieldset>
  {/if}
</section>

<style>
  .warnung {
    color: var(--warning-text);
  }
  tr.auswahl td {
    background: var(--auswahl);
  }
  .planeintrag {
    margin-top: var(--space-3);
  }
</style>
