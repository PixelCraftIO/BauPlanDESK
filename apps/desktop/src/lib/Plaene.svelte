<script lang="ts">
  import { editorAnmelden } from "./editoren";
  import { untrack, onDestroy } from "svelte";
  import { ask, save } from "@tauri-apps/plugin-dialog";
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
  import { planApi } from "./planaufmass/api";
  import type { CadProjektaufmass } from "./generated/plattform";
  import { standardFang, type Fangoptionen } from "./planaufmass/fang";

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
    /** Gemessene Planfläche bei den Flurstücksinformationen prüfen. */
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
  let messungen = $state<
    {
      nr: number;
      id: string;
      bezeichnung: string;
      art: string;
      ergebnis: Messung;
      punkte: Punkt[];
      geschlossen: boolean;
    }[]
  >([]);
  let viewer = $state<PlanViewer>();
  let laufendeNr = 0;
  let punktfang=$state(true),fangoptionen=$state<Fangoptionen>({...standardFang}),fangHinweis=$state("");
  let planGeneration=0,listeGeneration=0;
  onDestroy(()=>{planGeneration++;listeGeneration++;anfrage++});
  let cadRevision=$state(0),cadGespeichert=$state(""),cadLaeuft=$state(false),cadHinweis=$state("");
  const cadEingabe=$derived<CadProjektaufmass|null>(geladen?{dokument:{dokument_id:geladen.dokument.document_id,version:geladen.dokument.version},einheit,markierungen:messungen.map(m=>({id:m.id,bezeichnung:m.bezeichnung,punkte:m.punkte,geschlossen:m.geschlossen}))}:null);
  const cadSignatur=$derived(JSON.stringify(cadEingabe));
  const cadGeaendert=$derived(!!geladen&&cadGespeichert!==""&&cadSignatur!==cadGespeichert);
  async function cadSpeichern():Promise<boolean>{
    if(!cadEingabe||!darf||cadLaeuft)return false;
    const a=JSON.parse(cadSignatur) as CadProjektaufmass,stand=cadSignatur,generation=planGeneration,pid=projekt.id,basis=cadRevision;
    cadLaeuft=true;cadHinweis="";
    try{const f=await planApi.cadSpeichern(pid,a,basis);if(generation!==planGeneration)return false;cadRevision=f.revision;cadGespeichert=stand;cadHinweis=`Projektaufmaß gespeichert · Revision ${f.revision}`;return true}
    catch(e){onfehler(fehlertext(e));return false}finally{cadLaeuft=false}
  }
  async function cadExportieren(){
    if(!cadEingabe||!geladen)return;
    if(cadGeaendert&&(!darf||!(await ask("Für die CSV-Ausgabe das Projektaufmaß zuerst speichern?",{title:"CAD-Aufmaß ausgeben",okLabel:"Speichern und ausgeben",cancelLabel:"Abbrechen"}))||!(await cadSpeichern())))return;
    const d=cadEingabe.dokument,revision=cadRevision,ziel=await save({defaultPath:`${geladen.dokument.file_name.replace(/\.[^.]+$/,"")}-aufmass.csv`,filters:[{name:"Messwerte CSV",extensions:["csv"]}]});if(!ziel)return;
    try{await planApi.cadExportieren(projekt.id,d,revision,ziel);cadHinweis=`CSV ausgegeben: ${ziel}`}catch(e){onfehler(fehlertext(e))}
  }
  let baufenster = $state<Punkt[]>([]);
  let vorhaben = $state<Punkt[]>([]);
  // Baugrundstück des Vorhabenumrisses; bei mehreren Einheiten erforderlich.
  let grundSchluessel = $state<string | null>(null);
  let grundkontexte = $state<import("./api").GrundstueckKontext[]>([]);
  $effect(() => {
    void projekt.version;
    untrack(() => {
      api
        .grundstueckskontext(projekt.id)
        .then((g) => (grundkontexte = g))
        .catch(() => (grundkontexte = []));
    });
  });
  let gespeichertSignatur = $state("");
  let grenzpruefung = $state<BaugrenzenPruefung | null>(null);
  let ergebnisSignatur = $state("");
  let geoLaeuft = $state(false);
  let geoHinweis = $state<string | null>(null);
  const signatur = $derived(
    JSON.stringify([
      geladen?.dokument.document_id,
      geladen?.dokument.version,
      einheit,
      baufenster,
      vorhaben,
      grundSchluessel,
    ]),
  );
  const geoGeaendert = $derived(
    (baufenster.length > 0 || vorhaben.length > 0) &&
      signatur !== gespeichertSignatur,
  );
  const geometrie = $derived<BaugrenzenGeometrie | null>(
    geladen && baufenster.length >= 3 && vorhaben.length >= 3
      ? {
          dokument_id: geladen.dokument.document_id,
          dokument_version: geladen.dokument.version,
          einheit,
          baufenster,
          vorhaben,
          grundstueck_schluessel: grundSchluessel,
        }
      : null,
  );
  const aktuellesGrenzergebnis = $derived(
    grenzpruefung && ergebnisSignatur === signatur ? grenzpruefung : null,
  );

  export async function verwerfenErlaubt(): Promise<boolean> {
    return (
      (!geoGeaendert&&!cadGeaendert) ||
      (await ask(
        "Projektaufmaß oder Baugrenzenzuordnung wurden noch nicht gespeichert. Änderungen verwerfen?",
        {
          title: "Baugrenzen",
          kind: "warning",
          okLabel: "Verwerfen",
          cancelLabel: "Abbrechen",
        },
      ))
    );
  }

  async function planWechsel(id: string) {
    if (await verwerfenErlaubt()) dokumentId = id;
  }

  async function grenzePruefen(speichern = false) {
    if (!geometrie) return;
    geoLaeuft = true;
    geoHinweis = null;
    const stand = signatur;
    const g = structuredClone($state.snapshot(geometrie));
    try {
      const r = await api.baugrenzenPruefen(g);
      grenzpruefung = r;
      ergebnisSignatur = stand;
      if (speichern) {
        const p = await api.projektAktualisieren({
          id: projekt.id,
          expected_version: projekt.version,
          name: projekt.name,
          bundesland: projekt.bundesland,
          grundstuecksflaeche_m2: projekt.grundstuecksflaeche_m2,
          // Die Lage im Baufenster bewertet der Core aus diesen Umrissen
          // (§ 23 BauNVO); die gemessene Fläche ist keine Festsetzung.
          planungsrecht: {
            ...structuredClone($state.snapshot(projekt.planungsrecht)),
            baugrenzen: g,
          },
        });
        onprojekt(p);
        gespeichertSignatur = stand;
        geoHinweis =
          "Umrisse dem Projekt zugeordnet; die Lage im Baufenster erscheint im GRZ-Nachweis.";
      }
    } catch (e) {
      grenzpruefung = null;
      onfehler(fehlertext(e));
    } finally {
      geoLaeuft = false;
    }
  }

  // Jede Messung wird in core-lib gerechnet; hier nur die jeweils neueste Antwort anzeigen.
  let anfrage = 0;
  async function liveMessen(punkte: Punkt[], geschlossen: boolean) {
    const nr = ++anfrage;
    const generation=planGeneration,unit=einheit;
    if (punkte.length < 2) {
      live = null;
      return;
    }
    try {
      const m = await api.aufmass(punkte, geschlossen, unit);
      if (nr === anfrage&&generation===planGeneration&&unit===einheit) live = m;
    } catch {
      if (nr === anfrage) live = null;
    }
  }

  async function fertig(punkte: Punkt[], geschlossen: boolean) {
    if(laedt||cadLaeuft)return;
    anfrage++;
    live = null;
    const generation=planGeneration,unit=einheit,tool=werkzeug;
    try {
      const ergebnis = await api.aufmass(punkte, geschlossen, unit);
      if(generation!==planGeneration||unit!==einheit)return;
      const art =
        tool === "auswahl"
          ? "Fläche (gewählt)"
          : geschlossen
            ? "Fläche"
            : "Länge";
      messungen = [
        { nr: ++laufendeNr, id:crypto.randomUUID(),bezeichnung:`Messung ${laufendeNr}`, art, ergebnis, punkte, geschlossen },
        ...messungen,
      ];
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  // Einheit geändert: bisherige Messungen neu rechnen.
  async function einheitGeaendert() {
    anfrage++;const generation=planGeneration,unit=einheit,stand=messungen;
    try {
      if(!cadEingabe)return;const werte=await planApi.cadMesswerte(JSON.parse(cadSignatur));
      if(generation!==planGeneration||unit!==einheit||messungen!==stand)return;
      messungen=stand.map((m,i)=>({...m,ergebnis:werte[i]}));
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  async function listeLaden() {
    const generation=++listeGeneration;
    try {
      const alle = await api.dokumente(projektId);
      if(generation!==listeGeneration)return;
      const neueste = new Map<string, DocumentVersion>();
      for (const v of alle.filter((d) => istPlan(d.file_name)))
        if ((neueste.get(v.document_id)?.version ?? 0) < v.version)
          neueste.set(v.document_id, v);
      plaene = [...neueste.values()].sort((a, b) =>
        a.file_name.localeCompare(b.file_name, "de"),
      );
      if (!dokumentId && plaene.length) dokumentId = plaene[0].document_id;
      const aktuell = geladen
        ? neueste.get(geladen.dokument.document_id)
        : null;
      if (geladen && aktuell && aktuell.version !== geladen.dokument.version) {
        if (geoGeaendert||cadGeaendert)
          geoHinweis =
            "Eine neuere Planversion ist verfügbar. Bitte die aktuelle Zuordnung sichern oder verwerfen und den Plan neu laden.";
        else await planLaden(geladen.dokument.document_id);
      }
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  async function planLaden(id: string) {
    const generation=++planGeneration;anfrage++;
    laedt = true;
    try {
      const p = await api.cadPlan(id);
      if(generation!==planGeneration||id!==dokumentId)return;
      geladen = p;
      einheit = p.plan.einheit ?? "meter";
      sichtbar = p.plan.ebenen.map((e) => e.sichtbar);
      messungen = [];
      cadRevision=0;cadGespeichert=JSON.stringify({dokument:{dokument_id:p.dokument.document_id,version:p.dokument.version},einheit,markierungen:[]});cadHinweis="";
      live = null;
      grenzpruefung = null;
      geoHinweis = null;
      const g = untrack(() => projekt.planungsrecht.baugrenzen);
      if (
        g &&
        g.dokument_id === p.dokument.document_id &&
        g.dokument_version === p.dokument.version
      ) {
        baufenster = structuredClone($state.snapshot(g.baufenster));
        vorhaben = structuredClone($state.snapshot(g.vorhaben));
        einheit = g.einheit;
        grundSchluessel = g.grundstueck_schluessel ?? null;
      } else {
        baufenster = [];
        vorhaben = [];
        grundSchluessel = null;
        if (g?.dokument_id === p.dokument.document_id)
          geoHinweis =
            "Die gespeicherte Baugrenzenzuordnung gehört zu einer älteren Planversion. Bitte Umrisse neu zuordnen.";
      }
      gespeichertSignatur = JSON.stringify([
        p.dokument.document_id,
        p.dokument.version,
        einheit,
        baufenster,
        vorhaben,
        grundSchluessel,
      ]);
      const f=await planApi.cadLaden(projektId,{dokument_id:p.dokument.document_id,version:p.dokument.version});
      if(generation!==planGeneration)return;
      if(f){const a=f.daten,werte=await planApi.cadMesswerte(a);if(generation!==planGeneration)return;einheit=a.einheit;cadRevision=f.revision;messungen=a.markierungen.map((m,i)=>({...m,nr:++laufendeNr,art:m.geschlossen?"Fläche":"Strecke",ergebnis:werte[i]}));cadGespeichert=JSON.stringify(a);cadHinweis=`Gespeichertes Projektaufmaß · Revision ${f.revision}`;}
    } catch (e) {
      if(generation!==planGeneration)return;
      geladen = null;
      onfehler(fehlertext(e));
    } finally {
      if(generation===planGeneration)laedt = false;
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
    const benutzt = new Set([
      ...geladen.plan.linien.map((l) => l.ebene),
      ...geladen.plan.texte.map((t) => t.ebene),
    ]);
    return geladen.plan.ebenen
      .map((e, i) => ({ ...e, i }))
      .filter((e) => benutzt.has(e.i));
  });

  const WERKZEUGE: {
    id: Werkzeug;
    text: string;
    taste: string;
    tipp: string;
  }[] = [
    {
      id: "verschieben",
      text: "Verschieben",
      taste: "M",
      tipp: "Ziehen oder Leertaste verschiebt; Strg+Mausrad zoomt.",
    },
    {
      id: "laenge",
      text: "Strecke",
      taste: "S",
      tipp: "Punkte anklicken, Doppelklick oder Enter beendet.",
    },
    {
      id: "flaeche",
      text: "Polygon",
      taste: "P",
      tipp: "Eckpunkte anklicken, Doppelklick oder Enter schließt die Fläche.",
    },
    {
      id: "auswahl",
      text: "Auswählen",
      taste: "V",
      tipp: "In eine geschlossene Linie klicken, z. B. die Grundstücksgrenze.",
    },
  ];

  function taste(e: KeyboardEvent) {
    if (
      !aktiv ||
      !geladen ||
      (e.target as HTMLElement).closest("input, select, textarea, dialog")
    )
      return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const w = WERKZEUGE.find(
      (w) => w.taste.toLowerCase() === e.key.toLowerCase(),
    );
    if (w) werkzeug = w.id;
    else if (e.key === "Escape") {
      viewer?.abbrechen();
      live = null;
    } else if (e.key === "Enter") viewer?.abschliessen();
    else if (e.key === "Backspace") viewer?.letztenPunktEntfernen();
    else if (e.key === "Home") viewer?.einpassen();
    else return;
    e.preventDefault();
  }

  const z = (n: number, stellen = 2) =>
    n.toLocaleString("de-DE", {
      minimumFractionDigits: stellen,
      maximumFractionDigits: stellen,
    });
  const einheitKurz: Record<Laengeneinheit, string> = {
    millimeter: "mm",
    zentimeter: "cm",
    dezimeter: "dm",
    meter: "m",
    kilometer: "km",
    zoll: "in",
    fuss: "ft",
  };

  export function hatAenderungen() {
    return geoGeaendert||cadGeaendert;
  }
  export function istBeschaeftigt() {
    return geoLaeuft || laedt||cadLaeuft;
  }
  export async function speichernBeimSchliessen(): Promise<boolean> {
    if (istBeschaeftigt()) return false;
    if(cadGeaendert&&!(await cadSpeichern()))return false;
    if(geoGeaendert)await grenzePruefen(true);
    return !hatAenderungen();
  }
  editorAnmelden("plaene", {
    hatAenderungen,
    istBeschaeftigt,
    speichern: speichernBeimSchliessen,
  });
</script>

<svelte:window onkeydown={taste} />

{#if plaene.length === 0}
  <p class="meta">
    Im Projekt liegen noch keine DXF-Pläne. Importieren Sie einen Plan unter
    „Dokumente“. DWG-Dateien bitte vorher im CAD-Programm als DXF exportieren.
  </p>
{:else}
  <div class="planleiste">
    <label class="zeile"
      >Plan
      <select
        aria-label="Plan"
        value={dokumentId ?? ""}
        disabled={geoLaeuft}
        onchange={(e) => planWechsel(e.currentTarget.value)}
      >
        {#each plaene as p (p.document_id)}<option value={p.document_id}
            >{p.file_name} (V{p.version})</option
          >{/each}
      </select>
    </label>
    <div class="werkzeuge" role="toolbar" aria-label="Werkzeuge">
      <label><input type="checkbox" bind:checked={punktfang}/>Punktfang</label>
      <details><summary>Fangoptionen</summary>{#each [["endpunkte","Endpunkte"],["mittelpunkte","Mittelpunkte"],["kanten","Kanten"],["schnittpunkte","Schnittpunkte"]] as [id,titel]}<label><input type="checkbox" checked={fangoptionen[id as keyof Fangoptionen]} onchange={e=>fangoptionen={...fangoptionen,[id]:e.currentTarget.checked}}/>{titel}</label>{/each}</details>
      {#each WERKZEUGE as w}
        <button
          class:aktiv={werkzeug === w.id}
          aria-pressed={werkzeug === w.id}
          title="{w.tipp} (Taste {w.taste})"
          onclick={() => (werkzeug = w.id)}>{w.text}</button
        >
      {/each}
      <button
        class="sekundaer"
        disabled={geoLaeuft}
        onclick={async () => {
          if (dokumentId && (await verwerfenErlaubt()))
            await planLaden(dokumentId);
        }}>Plan neu laden</button
      >
      <button
        class="sekundaer"
        title="Ganzen Plan zeigen (Pos1)"
        onclick={() => viewer?.einpassen()}>Einpassen</button
      >
    </div>
    <label class="zeile"
      >Einheit
      <select
        aria-label="Einheit"
        bind:value={einheit}
        onchange={einheitGeaendert}
        disabled={geoLaeuft}
      >
        {#each EINHEITEN as e}<option value={e.code}>{e.name}</option>{/each}
      </select>
    </label>
  </div>

  {#if geladen}
    <div class="planbereich">
      <div class="planspalte">
        <PlanViewer
          bind:this={viewer}
          {punktfang} {fangoptionen} onfanghinweis={text=>fangHinweis=text}
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
            <strong class="live"
              >Länge {z(live.laenge_m)} m{live.flaeche_m2 !== null
                ? ` · Fläche ${z(live.flaeche_m2)} m²`
                : ""}</strong
            >
          {/if}
          {#if cursor}<span class="koordinaten"
              >x {z(cursor[0], 3)} · y {z(cursor[1], 3)}
              {einheitKurz[einheit]}</span
            >{/if}
        </div>
      </div>

      <aside class="planseite">
        {#if geladen.plan.einheit === null}
          <p class="hinweisbox">
            Die Datei nennt keine Einheit. Bitte oben die Einheit wählen, in der
            gezeichnet wurde, bevor Sie messen.
          </p>
        {/if}

        <h3>Baugrenzenprüfung</h3>
        <p class="meta">
          Geschlossene Umrisse mit „Fläche wählen“ auswählen oder mit „Fläche“
          zeichnen, dann unten als Baufenster und Vorhaben zuordnen.
        </p>
        <p class="meta">
          Baufenster: {baufenster.length >= 3
            ? "zugeordnet (blau)"
            : "fehlt"}<br />Vorhaben: {vorhaben.length >= 3
            ? "zugeordnet (violett)"
            : "fehlt"}
        </p>
        {#if grundkontexte.length > 1}<label
            >Baugrundstück des Vorhabenumrisses<select
              aria-label="Baugrundstück des Vorhabenumrisses"
              bind:value={grundSchluessel}
              ><option value={null}>Noch nicht zugeordnet</option
              >{#each grundkontexte as k (k.schluessel)}<option value={k.schluessel}
                  >{k.bezeichnung}</option
                >{/each}</select
            ></label
          >
          {#if !grundSchluessel}<p class="meta">
              Bei mehreren Baugrundstücken erscheint die Lage im Baufenster nur
              beim zugeordneten Grundstück.
            </p>{/if}{/if}
        <div class="aktionen">
          <button
            onclick={() => grenzePruefen()}
            disabled={!geometrie || geoLaeuft}>Baugrenze prüfen</button
          >
          <button
            class="sekundaer"
            onclick={() => grenzePruefen(true)}
            disabled={!darf || !geometrie || geoLaeuft || !geoGeaendert}
            >Zuordnung speichern</button
          >
        </div>
        {#if geoGeaendert}<p class="meta">Ungespeicherte Zuordnung</p>{/if}
        {#if geoHinweis}<p class="hinweisbox" role="status">
            {geoHinweis}
          </p>{/if}
        {#if grenzpruefung && !aktuellesGrenzergebnis}<p class="hinweisbox">
            Zuordnung oder Einheit geändert. Bitte neu prüfen.
          </p>{/if}
        {#if aktuellesGrenzergebnis}
          <dl class="grenzdaten">
            <dt>Baufenster</dt>
            <dd>{z(aktuellesGrenzergebnis.baufenster_m2)} m²</dd>
            <dt>Vorhaben</dt>
            <dd>{z(aktuellesGrenzergebnis.vorhaben_m2)} m²</dd>
            <dt>Außerhalb</dt>
            <dd class:verstoss={!aktuellesGrenzergebnis.eingehalten}>
              {z(aktuellesGrenzergebnis.ausserhalb_m2)} m² ({z(
                aktuellesGrenzergebnis.ausserhalb_prozent,
              )} %)
            </dd>
            <dt>Größter Überstand an Eckpunkten</dt>
            <dd>{z(aktuellesGrenzergebnis.eckpunkt_ueberstand_m)} m</dd>
          </dl>
          <p class:fehlertext={!aktuellesGrenzergebnis.eingehalten}>
            {aktuellesGrenzergebnis.eingehalten
              ? "Umriss liegt innerhalb des Baufensters."
              : "Umriss überschreitet das Baufenster (rote Fläche)."}
          </p>
          <p class="meta">
            Geometrischer Flächenvergleich. Der Eckpunktabstand erfasst nicht
            alle möglichen Überstände an Kanten. Ausnahmen, Befreiungen oder
            zulässige Bauteilüberschreitungen sind gesondert zu beurteilen.
          </p>
        {/if}
        {#if fangHinweis}<p class="hinweisbox" role="status">{fangHinweis}</p>{/if}
        <h3>Messungen</h3>
        <p class="meta" role="status">{cadGeaendert?"Projektaufmaß geändert · noch nicht gespeichert":cadHinweis||"Neue Messungen sind zunächst temporär."}</p>
        <button type="button" disabled={!darf||cadLaeuft||!cadGeaendert} onclick={()=>void cadSpeichern()}>{cadLaeuft?"Speichere Projektaufmaß …":"Projektaufmaß speichern"}</button>
        <button type="button" class="sekundaer" disabled={cadLaeuft||(!cadRevision&&!cadGeaendert)} onclick={()=>void cadExportieren()}>Messwerte CSV …</button>
        {#each messungen as m (m.nr)}
          <div class="messung">
            <div>
              <span class="meta">{m.nr}. {m.art}</span><label>Bezeichnung<input bind:value={m.bezeichnung} disabled={!darf||cadLaeuft} required/></label>
              {#if m.ergebnis.flaeche_m2 !== null}
                <strong>{z(m.ergebnis.flaeche_m2)} m²</strong>
                <span class="meta">Umfang {z(m.ergebnis.laenge_m)} m</span>
                {#if m.ergebnis.selbstschneidend}<span class="fehlertext"
                    >Umriss überschneidet sich, Fläche ungültig.</span
                  >{/if}
              {:else}
                <strong>{z(m.ergebnis.laenge_m)} m</strong>
              {/if}
            </div>
            <div class="aktionen">
              {#if m.ergebnis.flaeche_m2 !== null && !m.ergebnis.selbstschneidend}
                <button
                  class="link"
                  disabled={!darf || geoLaeuft}
                  onclick={() =>
                    onflaeche(Math.round(m.ergebnis.flaeche_m2! * 100) / 100)}
                  >Bei Flurstücksinformationen prüfen</button
                >
              {/if}
              {#if m.ergebnis.flaeche_m2 !== null && !m.ergebnis.selbstschneidend && m.punkte.length >= 3}
                <button
                  class="link"
                  disabled={geoLaeuft}
                  onclick={() => {
                    baufenster = structuredClone($state.snapshot(m.punkte));
                    geoHinweis = null;
                  }}>Als Baufenster</button
                >
                <button
                  class="link"
                  disabled={geoLaeuft}
                  onclick={() => {
                    vorhaben = structuredClone($state.snapshot(m.punkte));
                    geoHinweis = null;
                  }}>Als Vorhaben</button
                >
              {/if}
              <button
                class="link"
                onclick={() =>
                  (messungen = messungen.filter((x) => x.nr !== m.nr))}
                >Entfernen</button
              >
            </div>
          </div>
        {:else}
          <p class="meta">
            Noch keine Messung. Wählen Sie „Länge“, „Fläche“ oder „Fläche
            wählen“.
          </p>
        {/each}

        <h3>Ebenen</h3>
        <div class="knoepfe links">
          <button
            class="link"
            onclick={() => (sichtbar = sichtbar.map(() => true))}>Alle</button
          >
          <button
            class="link"
            onclick={() => (sichtbar = sichtbar.map(() => false))}>Keine</button
          >
        </div>
        <ul class="ebenen">
          {#each ebenen as e (e.i)}
            <li>
              <label>
                <input type="checkbox" bind:checked={sichtbar[e.i]} />
                <span class="farbfeld" style:background={e.farbe ?? "var(--fg)"}
                ></span>
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
