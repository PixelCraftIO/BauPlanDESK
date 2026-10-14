<script lang="ts">
  import { untrack } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import {
    api,
    EINHEITEN,
    fehlertext,
    istPlan,
    type CadPlan,
    type Project,
    type BaugrenzenGeometrie,
    type BaugrenzenPruefung,
    type DocumentVersion,
    type Laengeneinheit,
    type Messung,
    type Punkt,
  } from "./api";
  import PlanViewer, { type Werkzeug } from "./PlanViewer.svelte";

  let {
    projekt,
    aktiv,
    darf,
    onprojekt,
    dokumentId = $bindable(null),
    onflaeche,
    onfehler,
  }: {
    projekt: Project;
    aktiv: boolean;
    darf: boolean;
    onprojekt: (p: Project) => void;
    dokumentId: string | null;
    /** Gemessene Fläche als Grundstücksfläche übernehmen. */
    onflaeche: (m2: number) => void;
    onfehler: (m: string) => void;
  } = $props();

  const projektId = $derived(projekt.id);
  let plaene = $state<DocumentVersion[]>([]);
  let geladen = $state<CadPlan | null>(null);
  let laedt = $state(false);
  let einheit = $state<Laengeneinheit>("meter");
  let sichtbar = $state<boolean[]>([]);
  let werkzeug = $state<Werkzeug>("verschieben");
  let live = $state<Messung | null>(null);
  let cursor = $state<Punkt | null>(null);
  let messungen = $state<{ nr: number; art: string; ergebnis: Messung; punkte: Punkt[]; geschlossen: boolean }[]>([]);
  let viewer = $state<PlanViewer>();
  let laufendeNr = 0;
  let baufenster = $state<Punkt[]>([]);
  let vorhaben = $state<Punkt[]>([]);
  let gespeichertSignatur = $state("");
  let grenzpruefung = $state<BaugrenzenPruefung | null>(null);
  let ergebnisSignatur = $state("");
  let geoLaeuft = $state(false);
  let geoHinweis = $state<string | null>(null);
  const signatur = $derived(JSON.stringify([geladen?.dokument.document_id, geladen?.dokument.version, einheit, baufenster, vorhaben]));
  const geoGeaendert = $derived((baufenster.length > 0 || vorhaben.length > 0) && signatur !== gespeichertSignatur);
  const geometrie = $derived<BaugrenzenGeometrie | null>(geladen && baufenster.length >= 3 && vorhaben.length >= 3 ? {
    dokument_id: geladen.dokument.document_id, dokument_version: geladen.dokument.version, einheit, baufenster, vorhaben,
  } : null);
  const aktuellesGrenzergebnis = $derived(grenzpruefung && ergebnisSignatur === signatur ? grenzpruefung : null);

  export async function verwerfenErlaubt(): Promise<boolean> {
    return !geoGeaendert || await ask("Die Zuordnung von Baufenster und Vorhaben wurde nicht gespeichert. Verwerfen?", { title: "Baugrenzen", kind: "warning", okLabel: "Verwerfen", cancelLabel: "Abbrechen" });
  }

  async function planWechsel(id: string) {
    if (await verwerfenErlaubt()) dokumentId = id;
  }

  async function grenzePruefen(speichern = false) {
    if (!geometrie) return;
    geoLaeuft = true; geoHinweis = null;
    const stand = signatur;
    const g = structuredClone($state.snapshot(geometrie));
    try {
      const r = await api.baugrenzenPruefen(g);
      grenzpruefung = r;
      ergebnisSignatur = stand;
      if (speichern) {
        const p = await api.projektAktualisieren({
          id: projekt.id, expected_version: projekt.version, name: projekt.name, bundesland: projekt.bundesland,
          grundstuecksflaeche_m2: projekt.grundstuecksflaeche_m2,
          planungsrecht: {
            ...structuredClone($state.snapshot(projekt.planungsrecht)), baugrenzen: g,
            festgesetzte_werte: { ...structuredClone($state.snapshot(projekt.planungsrecht.festgesetzte_werte)), ueberbaubare_flaeche_m2: r.baufenster_m2 },
          },
        });
        onprojekt(p);
        gespeichertSignatur = stand;
        geoHinweis = "Umrisse dem Projekt zugeordnet; Baufensterfläche in die Festsetzungen übernommen.";
      }
    } catch(e) { grenzpruefung = null; onfehler(fehlertext(e)); }
    finally { geoLaeuft = false; }
  }


  // Jede Messung wird in core-lib gerechnet; hier nur die jeweils neueste Antwort anzeigen.
  let anfrage = 0;
  async function liveMessen(punkte: Punkt[], geschlossen: boolean) {
    const nr = ++anfrage;
    if (punkte.length < 2) {
      live = null;
      return;
    }
    try {
      const m = await api.aufmass(punkte, geschlossen, einheit);
      if (nr === anfrage) live = m;
    } catch {
      if (nr === anfrage) live = null;
    }
  }

  async function fertig(punkte: Punkt[], geschlossen: boolean) {
    anfrage++;
    live = null;
    try {
      const ergebnis = await api.aufmass(punkte, geschlossen, einheit);
      const art = werkzeug === "auswahl" ? "Fläche (gewählt)" : geschlossen ? "Fläche" : "Länge";
      messungen = [{ nr: ++laufendeNr, art, ergebnis, punkte, geschlossen }, ...messungen];
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  // Einheit geändert: bisherige Messungen neu rechnen.
  async function einheitGeaendert() {
    try {
      messungen = await Promise.all(
        messungen.map(async (m) => ({ ...m, ergebnis: await api.aufmass(m.punkte, m.geschlossen, einheit) })),
      );
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  async function listeLaden() {
    try {
      const alle = await api.dokumente(projektId);
      const neueste = new Map<string, DocumentVersion>();
      for (const v of alle.filter((d) => istPlan(d.file_name)))
        if ((neueste.get(v.document_id)?.version ?? 0) < v.version) neueste.set(v.document_id, v);
      plaene = [...neueste.values()].sort((a, b) => a.file_name.localeCompare(b.file_name, "de"));
      if (!dokumentId && plaene.length) dokumentId = plaene[0].document_id;
      const aktuell = geladen ? neueste.get(geladen.dokument.document_id) : null;
      if (geladen && aktuell && aktuell.version !== geladen.dokument.version) {
        if (geoGeaendert) geoHinweis = "Eine neuere Planversion ist verfügbar. Bitte die aktuelle Zuordnung sichern oder verwerfen und den Plan neu laden.";
        else await planLaden(geladen.dokument.document_id);
      }
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  async function planLaden(id: string) {
    laedt = true;
    try {
      const p = await api.cadPlan(id);
      geladen = p;
      einheit = p.plan.einheit ?? "meter";
      sichtbar = p.plan.ebenen.map((e) => e.sichtbar);
      messungen = [];
      live = null;
      grenzpruefung = null;
      geoHinweis = null;
      const g = untrack(() => projekt.planungsrecht.baugrenzen);
      if (g && g.dokument_id === p.dokument.document_id && g.dokument_version === p.dokument.version) {
        baufenster = structuredClone($state.snapshot(g.baufenster));
        vorhaben = structuredClone($state.snapshot(g.vorhaben));
        einheit = g.einheit;
      } else {
        baufenster = []; vorhaben = [];
        if (g?.dokument_id === p.dokument.document_id) geoHinweis = "Die gespeicherte Baugrenzenzuordnung gehört zu einer älteren Planversion. Bitte Umrisse neu zuordnen.";
      }
      gespeichertSignatur = JSON.stringify([p.dokument.document_id, p.dokument.version, einheit, baufenster, vorhaben]);
    } catch (e) {
      geladen = null;
      onfehler(fehlertext(e));
    } finally {
      laedt = false;
    }
  }

  $effect(() => {
    void projektId;
    if (aktiv) untrack(listeLaden);
  });

  $effect(() => {
    if (dokumentId) planLaden(dokumentId);
  });

  // Nur Ebenen mit Inhalt anzeigen.
  const ebenen = $derived.by(() => {
    if (!geladen) return [];
    const benutzt = new Set([...geladen.plan.linien.map((l) => l.ebene), ...geladen.plan.texte.map((t) => t.ebene)]);
    return geladen.plan.ebenen.map((e, i) => ({ ...e, i })).filter((e) => benutzt.has(e.i));
  });

  const WERKZEUGE: { id: Werkzeug; text: string; taste: string; tipp: string }[] = [
    { id: "verschieben", text: "Verschieben", taste: "V", tipp: "Ziehen verschiebt, Mausrad zoomt." },
    { id: "laenge", text: "Länge", taste: "L", tipp: "Punkte anklicken, Doppelklick oder Enter beendet." },
    { id: "flaeche", text: "Fläche", taste: "F", tipp: "Eckpunkte anklicken, Doppelklick oder Enter schließt die Fläche." },
    { id: "auswahl", text: "Fläche wählen", taste: "W", tipp: "In eine geschlossene Linie klicken, z. B. die Grundstücksgrenze." },
  ];

  function taste(e: KeyboardEvent) {
    if (!aktiv || !geladen || (e.target as HTMLElement).closest("input, select, textarea, dialog")) return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const w = WERKZEUGE.find((w) => w.taste.toLowerCase() === e.key.toLowerCase());
    if (w) werkzeug = w.id;
    else if (e.key === "Escape") { viewer?.abbrechen(); live = null; }
    else if (e.key === "Enter") viewer?.abschliessen();
    else if (e.key === "Backspace") viewer?.letztenPunktEntfernen();
    else if (e.key === "Home") viewer?.einpassen();
    else return;
    e.preventDefault();
  }

  const z = (n: number, stellen = 2) => n.toLocaleString("de-DE", { minimumFractionDigits: stellen, maximumFractionDigits: stellen });
  const einheitKurz: Record<Laengeneinheit, string> = {
    millimeter: "mm", zentimeter: "cm", dezimeter: "dm", meter: "m", kilometer: "km", zoll: "in", fuss: "ft",
  };
</script>

<svelte:window onkeydown={taste} />

{#if plaene.length === 0}
  <p class="meta">Im Projekt liegen noch keine DXF-Pläne. Importieren Sie einen Plan unter „Dokumente“. DWG-Dateien bitte vorher im CAD-Programm als DXF exportieren.</p>
{:else}
  <div class="planleiste">
    <label class="zeile">Plan
      <select value={dokumentId ?? ""} disabled={geoLaeuft} onchange={(e) => planWechsel(e.currentTarget.value)}>
        {#each plaene as p (p.document_id)}<option value={p.document_id}>{p.file_name} (V{p.version})</option>{/each}
      </select>
    </label>
    <div class="werkzeuge" role="toolbar" aria-label="Werkzeuge">
      {#each WERKZEUGE as w}
        <button class:aktiv={werkzeug === w.id} aria-pressed={werkzeug === w.id} title="{w.tipp} (Taste {w.taste})" onclick={() => (werkzeug = w.id)}>{w.text}</button>
      {/each}
      <button class="sekundaer" disabled={geoLaeuft} onclick={async () => { if (dokumentId && await verwerfenErlaubt()) await planLaden(dokumentId); }}>Plan neu laden</button>
      <button class="sekundaer" title="Ganzen Plan zeigen (Pos1)" onclick={() => viewer?.einpassen()}>Einpassen</button>
    </div>
    <label class="zeile">Einheit
      <select bind:value={einheit} onchange={einheitGeaendert} disabled={geoLaeuft}>
        {#each EINHEITEN as e}<option value={e.code}>{e.name}</option>{/each}
      </select>
    </label>
  </div>

  {#if geladen}
    <div class="planbereich">
      <div class="planspalte">
        <PlanViewer
          bind:this={viewer}
          plan={geladen.plan}
          {sichtbar}
          {werkzeug}
          baugrenzen={geometrie}
          grenzpruefung={aktuellesGrenzergebnis}
          onaufmass={liveMessen}
          onfertig={fertig}
          oncursor={(p) => (cursor = p)}
        />
        <div class="planstatus meta">
          <span>{WERKZEUGE.find((w) => w.id === werkzeug)?.tipp}</span>
          {#if live}
            <strong class="live">Länge {z(live.laenge_m)} m{live.flaeche_m2 !== null ? ` · Fläche ${z(live.flaeche_m2)} m²` : ""}</strong>
          {/if}
          {#if cursor}<span class="koordinaten">x {z(cursor[0], 3)} · y {z(cursor[1], 3)} {einheitKurz[einheit]}</span>{/if}
        </div>
      </div>

      <aside class="planseite">
        {#if geladen.plan.einheit === null}
          <p class="hinweisbox">Die Datei nennt keine Einheit. Bitte oben die Einheit wählen, in der gezeichnet wurde, bevor Sie messen.</p>
        {/if}

        <h3>Baugrenzenprüfung</h3>
        <p class="meta">Geschlossene Umrisse mit „Fläche wählen“ auswählen oder mit „Fläche“ zeichnen, dann unten als Baufenster und Vorhaben zuordnen.</p>
        <p class="meta">Baufenster: {baufenster.length >= 3 ? "zugeordnet (blau)" : "fehlt"}<br />Vorhaben: {vorhaben.length >= 3 ? "zugeordnet (violett)" : "fehlt"}</p>
        <div class="aktionen">
          <button onclick={() => grenzePruefen()} disabled={!geometrie || geoLaeuft}>Baugrenze prüfen</button>
          <button class="sekundaer" onclick={() => grenzePruefen(true)} disabled={!darf || !geometrie || geoLaeuft || !geoGeaendert}>Zuordnung speichern</button>
        </div>
        {#if geoGeaendert}<p class="meta">Ungespeicherte Zuordnung</p>{/if}
        {#if geoHinweis}<p class="hinweisbox" role="status">{geoHinweis}</p>{/if}
        {#if grenzpruefung && !aktuellesGrenzergebnis}<p class="hinweisbox">Zuordnung oder Einheit geändert. Bitte neu prüfen.</p>{/if}
        {#if aktuellesGrenzergebnis}
          <dl class="grenzdaten">
            <dt>Baufenster</dt><dd>{z(aktuellesGrenzergebnis.baufenster_m2)} m²</dd>
            <dt>Vorhaben</dt><dd>{z(aktuellesGrenzergebnis.vorhaben_m2)} m²</dd>
            <dt>Außerhalb</dt><dd class:verstoss={!aktuellesGrenzergebnis.eingehalten}>{z(aktuellesGrenzergebnis.ausserhalb_m2)} m² ({z(aktuellesGrenzergebnis.ausserhalb_prozent)} %)</dd>
            <dt>Größter Überstand an Eckpunkten</dt><dd>{z(aktuellesGrenzergebnis.eckpunkt_ueberstand_m)} m</dd>
          </dl>
          <p class:fehlertext={!aktuellesGrenzergebnis.eingehalten}>{aktuellesGrenzergebnis.eingehalten ? "Umriss liegt innerhalb des Baufensters." : "Umriss überschreitet das Baufenster (rote Fläche)."}</p>
          <p class="meta">Geometrischer Flächenvergleich. Der Eckpunktabstand erfasst nicht alle möglichen Überstände an Kanten. Ausnahmen, Befreiungen oder zulässige Bauteilüberschreitungen sind gesondert zu beurteilen.</p>
        {/if}
        <h3>Messungen</h3>
        {#each messungen as m (m.nr)}
          <div class="messung">
            <div>
              <span class="meta">{m.nr}. {m.art}</span>
              {#if m.ergebnis.flaeche_m2 !== null}
                <strong>{z(m.ergebnis.flaeche_m2)} m²</strong>
                <span class="meta">Umfang {z(m.ergebnis.laenge_m)} m</span>
                {#if m.ergebnis.selbstschneidend}<span class="fehlertext">Umriss überschneidet sich, Fläche ungültig.</span>{/if}
              {:else}
                <strong>{z(m.ergebnis.laenge_m)} m</strong>
              {/if}
            </div>
            <div class="aktionen">
              {#if m.ergebnis.flaeche_m2 !== null && !m.ergebnis.selbstschneidend}
                <button class="link" disabled={!darf || geoLaeuft} onclick={() => onflaeche(Math.round(m.ergebnis.flaeche_m2! * 100) / 100)}>Als Grundstücksfläche übernehmen</button>
              {/if}
              {#if m.ergebnis.flaeche_m2 !== null && !m.ergebnis.selbstschneidend && m.punkte.length >= 3}
                <button class="link" disabled={geoLaeuft} onclick={() => { baufenster = structuredClone($state.snapshot(m.punkte)); geoHinweis = null; }}>Als Baufenster</button>
                <button class="link" disabled={geoLaeuft} onclick={() => { vorhaben = structuredClone($state.snapshot(m.punkte)); geoHinweis = null; }}>Als Vorhaben</button>
              {/if}
              <button class="link" onclick={() => (messungen = messungen.filter((x) => x.nr !== m.nr))}>Entfernen</button>
            </div>
          </div>
        {:else}
          <p class="meta">Noch keine Messung. Wählen Sie „Länge“, „Fläche“ oder „Fläche wählen“.</p>
        {/each}

        <h3>Ebenen</h3>
        <div class="knoepfe links">
          <button class="link" onclick={() => (sichtbar = sichtbar.map(() => true))}>Alle</button>
          <button class="link" onclick={() => (sichtbar = sichtbar.map(() => false))}>Keine</button>
        </div>
        <ul class="ebenen">
          {#each ebenen as e (e.i)}
            <li>
              <label>
                <input type="checkbox" bind:checked={sichtbar[e.i]} />
                <span class="farbfeld" style:background={e.farbe ?? "var(--fg)"}></span>
                {e.name}
              </label>
            </li>
          {/each}
        </ul>

        {#if geladen.plan.hinweise.length}
          <h3>Hinweise zum Import</h3>
          <ul class="hinweise meta">
            {#each geladen.plan.hinweise as h}<li>{h}</li>{/each}
          </ul>
        {/if}
      </aside>
    </div>
  {:else if laedt}
    <p class="meta">Plan wird geladen …</p>
  {/if}
{/if}
