<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Project, GrundstueckKontext } from "../api";
  import { api, fehlertext } from "../api";
  import type {
    Gebaeude,
    NachweisAnsicht,
    FachErgebnis,
  } from "../generated/plattform";
  import { plattformApi } from "../plattformApi";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  import GrundlagenZahl from "./GrundlagenZahl.svelte";
  import GeschossMasse from "./GeschossMasse.svelte";
  import type { GebaeudeGrundlagen } from "../generated/plattform";
  let {
    projekt,
    benutzerId,
    darf,
  }: { projekt: Project; benutzerId: string; darf: boolean } = $props();
  const s = untrack(
    () =>
      new FachSitzung<Gebaeude>(
        projekt.id,
        "gebaeude",
        benutzerId,
        () => ({
          bezeichnung: "",
          grundstueck_ids: [],
          geschosse: [],
          quelle: null,
        }),
        (g) => g.bezeichnung,
      ),
  );
  let nachweise = $state<NachweisAnsicht[]>([]);
  let vorschau = $state<GebaeudeGrundlagen | null>(null);
  let vorschauFehler = $state("");
  const hoehenfelder = [
    ["nullpunkt_m_nhn", "Gebäudenullpunkt ±0,00 (m ü. NHN)"],
    ["gelaende_m", "Maßgebende mittlere Geländehöhe zu ±0,00 (m)"],
    ["fundamentsohle_m", "Fundamentsohle zu ±0,00 (m)"],
    ["traufe_m", "Traufhöhe zu ±0,00 (m)"],
    ["first_m", "Firsthöhe zu ±0,00 (m)"],
  ] as const;
  const z = (n: number | null) => n === null ? "Noch unbekannt" : n.toLocaleString("de-DE", {maximumFractionDigits: 3});
  $effect(() => {
    const daten = JSON.stringify(s.eingabe);
    const id = s.id;
    vorschau = null;
    vorschauFehler = "";
    if (!s.eingabe || !id) return;
    let abgebrochen = false;
    const timer = setTimeout(async () => {
      try {
        const r = await plattformApi.gebaeudeAuswerten(s.projektId, id, JSON.parse(daten));
        if (!abgebrochen) vorschau = r;
      } catch (e) { if (!abgebrochen) vorschauFehler = fehlertext(e); }
    }, 300);
    return () => { abgebrochen = true; clearTimeout(timer); };
  });
  async function quellstaende() {
    try {
      nachweise = await plattformApi.nachweise(projekt.id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  onMount(quellstaende);
  function quellen(geschossId: string) {
    return nachweise.flatMap((n) => {
      if (
        !n.aktuell ||
        !n.stand.vollstaendig ||
        n.stand.eingabe.art !== "fach" ||
        n.stand.eingabe.plan.modul !== "vollgeschosse" ||
        n.stand.eingabe.plan.gebaeude_id !== s.id
      )
        return [];
      const input = n.stand.eingabe.plan.eingaben as {
        geschosse: { id: string; gebaeude_geschoss_id: string | null }[];
      };
      const r = n.stand.ergebnis as unknown as FachErgebnis;
      return r.kennzahlen
        .filter(
          (k) =>
            k.id.startsWith("geschoss:") &&
            input.geschosse.some(
              (s) =>
                `geschoss:${s.id}` === k.id &&
                s.gebaeude_geschoss_id === geschossId,
            ),
        )
        .map((k) => ({
          nachweis: n.stand.id,
          kennzahl: k.id,
          wert: k.wert,
          titel: `${n.stand.bezeichnung} · ${k.bezeichnung}`,
        }));
    });
  }
  let grund = $state<GrundstueckKontext[]>([]);
  onMount(async () => {
    try {
      grund = await api.grundstueckskontext(projekt.id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
</script>

<Fachliste titel="Gebäude und Geschosse" {s} bereich="gebaeude" {darf}>
  {#snippet zusatz(g)}<span>{z(g.geometrie?.grundflaeche_m2 ?? null)}{g.geometrie?.grundflaeche_m2 !== undefined ? " m² Grundfläche" : ""}</span><br /><span class="meta">{g.geschosse.length ? `${g.geschosse.length} Geschosse erfasst` : "Einzelgeschoss aus Grundfläche"}</span>{/snippet}
  {#snippet children(g)}
    <label
      >Gebäudebezeichnung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Gebäudebezeichnung"
        bind:value={g.bezeichnung}
        required
        maxlength="500"
      /></label
    >
    <p class="meta">Erfassen Sie zunächst, was bereits bekannt ist. Flächen und Höhen können später ergänzt werden; leere Felder bleiben unbekannt.</p>
    <section class="grundlagen-block" aria-label="Gebäudegrundfläche">
      <h3>Gebäudegrundfläche</h3>
      <div class="raster">
        <label>Gebäudetyp für GRZ<select value={g.geometrie?.typ ?? ""} onchange={(e) => { (g.geometrie ??= {}).typ = (e.currentTarget.value || undefined) as NonNullable<Gebaeude["geometrie"]>["typ"]; }}>
          <option value="">Noch nicht zugeordnet</option><option value="hauptanlage">Hauptgebäude / Wohnhaus</option><option value="garage">Garage</option><option value="nebenanlage14">Nebenanlage (§ 14)</option><option value="unterirdisch">Unterirdische Anlage</option>
        </select></label>
        <GrundlagenZahl titel="Gebäudegrundfläche, horizontale Projektion (m²)" min={0} wert={g.geometrie?.grundflaeche_m2} onwert={(v) => { (g.geometrie ??= {}).grundflaeche_m2 = v; }} />
      </div>
      <p class="meta">Ohne einzeln erfasste Geschosse wird diese Grundfläche zugleich als Fläche eines vorläufigen Einzelgeschosses verwendet. Bei mehreren Geschossen bleibt sie der einmalige Gebäudeumgriff für die GRZ.</p>
    </section>
    <details class="grundlagen-block"><summary>Höhen und Bezugspunkt ergänzen</summary>
      <div class="raster">{#each hoehenfelder as [key, titel]}<GrundlagenZahl {titel} wert={g.geometrie?.[key]} onwert={(v) => { (g.geometrie ??= {})[key] = v; }} />{/each}</div>
      <label>Herkunft der Flächen und Höhen<textarea maxlength="4000" value={g.geometrie?.herkunft ?? ""} placeholder="z. B. Vorentwurf, Aufmaß oder Vermessung mit Datum" oninput={(e) => { (g.geometrie ??= {}).herkunft = e.currentTarget.value; }}></textarea></label>
      <p class="meta">Fundamentsohle und Fußbodenhöhe sind unterschiedliche Angaben. Welches Gelände und welche Deckenlage für eine Vollgeschossprüfung maßgebend sind, wird im Nachweis anhand der geltenden Vorschrift geprüft.</p>
    </details>
    <fieldset>
      <legend>Zugeordnete Baugrundstücke</legend>{#each grund as k (k.id)}<label
          ><input
            type="checkbox"
            checked={g.grundstueck_ids.includes(k.id)}
            onchange={(e) => {
              g.grundstueck_ids = e.currentTarget.checked
                ? [...g.grundstueck_ids, k.id]
                : g.grundstueck_ids.filter((id) => id !== k.id);
            }}
          />{k.bezeichnung}</label
        >{:else}<p class="meta">
          Zunächst Baugrundstücke ermitteln. Die Zuordnung kann anschließend
          ergänzt werden.
        </p>{/each}
    </fieldset>
    <h3>Geschosse</h3>
    <button type="button" class="sekundaer" onclick={quellstaende}
      >Vollgeschossnachweise aktualisieren</button
    >
    {#if !g.geschosse.length}<section class="einzelgeschoss">
      <h4>Einzelgeschoss · aus Gebäudegrundfläche</h4>
      <p class="meta">Für eine eingeschossige Garage ist kein zusätzliches EG erforderlich. Die Vollgeschosseinstufung bleibt zunächst offen.</p>
      <details><summary>Bekannte Höhen des Einzelgeschosses</summary><GeschossMasse einzelgeschoss masse={g.geometrie?.einzelgeschoss} onmasse={(m) => { (g.geometrie ??= {}).einzelgeschoss = m; }} /></details>
    </section>{/if}
    {#each g.geschosse as geschoss, i (geschoss.id)}<fieldset>
        <legend>Geschoss {i + 1}</legend>
        <div class="raster">
          <label
            >Bezeichnung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Bezeichnung"
              bind:value={geschoss.bezeichnung}
              required
            /></label
          ><label
            >Einstufung<select
              aria-label="Einstufung"
              bind:value={geschoss.vollgeschoss}
              disabled={!!geschoss.einstufungsquelle}
              ><option value={null}>Noch zu prüfen</option><option value={true}
                >Vollgeschoss</option
              ><option value={false}>Nichtvollgeschoss</option></select
            ></label
          ><label
            >Grundlage der Einstufung<input
              bind:value={geschoss.grundlage}
              required={geschoss.vollgeschoss !== null}
              placeholder="Anwendbare Vorschrift und Herleitung"
            /></label
          >
        </div>
        <GeschossMasse masse={geschoss.geometrie} onmasse={(m) => { geschoss.geometrie = m; }} geschosse={g.geschosse} id={geschoss.id} />
        <label
          >Aus Vollgeschossnachweis übernehmen<select
            value={geschoss.einstufungsquelle
              ? `${geschoss.einstufungsquelle.nachweis_id}|${geschoss.einstufungsquelle.kennzahl}`
              : ""}
            onchange={(e) => {
              const q = quellen(geschoss.id).find(
                (q) => `${q.nachweis}|${q.kennzahl}` === e.currentTarget.value,
              );
              geschoss.einstufungsquelle = q
                ? { nachweis_id: q.nachweis, kennzahl: q.kennzahl }
                : undefined;
              if (q) geschoss.vollgeschoss = q.wert === 1;
            }}
            ><option value="">Manuelle Einstufung</option
            >{#each quellen(geschoss.id) as q}<option
                value={`${q.nachweis}|${q.kennzahl}`}>{q.titel}</option
              >{/each}</select
          ></label
        >
        {#if geschoss.einstufungsquelle}<p class="meta">
            Die GFZ-/Geschossigkeitsberechnung prüft diesen Quellnachweis bei
            jeder Verwendung auf Aktualität.
          </p>{/if}
        <button
          type="button"
          class="link"
          onclick={() => g.geschosse.splice(i, 1)}>Geschoss entfernen</button
        >
      </fieldset>{/each}
    <button
      type="button"
      class="sekundaer"
      onclick={() =>
        g.geschosse.push({
          id: crypto.randomUUID(),
          bezeichnung: "",
          vollgeschoss: null,
          grundlage: "",
        })}>Geschoss hinzufügen</button
    >
    {#if vorschauFehler}<p role="alert" class="fehlertext">{vorschauFehler}</p>{/if}
    {#if vorschau}<section class="grundlagen-auswertung" aria-label="Abgeleitete Gebäudegrundlagen">
      <h3>Aus den bekannten Angaben</h3>
      <p>Gebäudegrundfläche: <strong>{z(vorschau.grundflaeche_m2)}{vorschau.grundflaeche_m2 !== null ? " m²" : ""}</strong> · Summe der Geschossflächen: <strong>{z(vorschau.geschossflaechensumme_m2)}{vorschau.geschossflaechensumme_m2 !== null ? " m²" : ""}</strong></p>
      <p class="meta">Die Geschossflächensumme ist noch keine anrechenbare GFZ-Fläche. Hierfür werden Vollgeschosseinstufung und Planungsrecht im Berechnungsbereich geprüft.</p>
      <div class="tabellen-scroll"><table class="ergebnistabelle"><thead><tr><th>Ebene</th><th>Fläche (m²)</th><th>OK Decke zu ±0,00 (m)</th><th>Decke über Gelände (m)</th></tr></thead><tbody>
      {#each vorschau.geschosse as s}<tr><th scope="row">{s.bezeichnung}{s.implizit ? " (vorläufig)" : ""}</th><td>{z(s.flaeche_m2)}</td><td>{z(s.decke_oberkante_m)}</td><td>{z(s.decke_ueber_gelaende_m)}</td></tr>{/each}
      </tbody></table></div>
      {#each vorschau.geschosse.filter((s) => s.decke_oberkante_m !== null) as s}<p class="meta">{s.bezeichnung}: {s.herleitung}.</p>{/each}
      {#if vorschau.offen.length}<details><summary>Noch offene Grundlagen ({vorschau.offen.length})</summary><ul>{#each vorschau.offen as o}<li>{o}</li>{/each}</ul></details>{/if}
    </section>{/if}
    <p class="meta">
      Gebäude und Geschosse besitzen feste Identitäten. Die
      Vollgeschosseinstufung wird mit ihrer fachlichen Grundlage erfasst.
    </p>
  {/snippet}
</Fachliste>

<style>
  .grundlagen-block { margin-block: 1.3rem; padding-block: 1rem; border-block-start: 1px solid var(--border); }
  summary { cursor: pointer; font-weight: 600; }
  .einzelgeschoss { margin-block: 1rem; padding: 1rem; border-inline-start: 3px solid var(--accent); background: var(--accent-soft); }
  .einzelgeschoss h4 { margin-top: 0; }
  .grundlagen-auswertung { margin-block: 1.5rem; padding-block: 1rem; border-block: 1px solid var(--border); font-variant-numeric: tabular-nums; }
  .tabellen-scroll { overflow-x: auto; }
</style>
