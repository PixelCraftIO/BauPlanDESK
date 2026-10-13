<script lang="ts">
  import { untrack } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import {
    CODELISTE_ANLAGEN,
    MASSNAHMEN,
    alsBeteiligter,
    anschriftZeile,
    api,
    fehlertext,
    istKonflikt,
    kundenname,
    leereAnschrift,
    mindestens,
    type Bauantrag,
    type Benutzer,
    type Beteiligter,
    type Codeliste,
    type DocumentVersion,
    type GespeicherterBauantrag,
    type Kunde,
    type Project,
  } from "./api";
  import AnschriftFelder from "./AnschriftFelder.svelte";
  import BeteiligterFelder from "./BeteiligterFelder.svelte";

  let {
    projekt,
    benutzer,
    exportAusloeser,
    onbuero,
    onbauherren,
    onfehler,
    onhinweis,
  }: {
    projekt: Project;
    benutzer: Benutzer;
    exportAusloeser: number;
    /** Dialog „Büroangaben“ öffnen. */
    onbuero: () => void;
    /** Dialog „Bauherren“ öffnen. */
    onbauherren: () => void;
    onfehler: (m: string) => void;
    onhinweis: (m: string) => void;
  } = $props();

  const darf = $derived(benutzer.aktiv && mindestens(benutzer.rolle, "bearbeitung"));

  let gespeichert = $state<GespeicherterBauantrag | null>(null);
  let antrag = $state<Bauantrag | null>(null);
  let dokumente = $state<DocumentVersion[]>([]);
  let anlagenarten = $state<Codeliste | null>(null);
  let bauherren = $state<Kunde[]>([]);
  let kunden = $state<Kunde[]>([]);
  let laeuft = $state(false);

  const geaendert = $derived(
    !!antrag && !!gespeichert && JSON.stringify(antrag) !== JSON.stringify(gespeichert.antrag),
  );
  const gebuehrenBauherr = $derived(antrag?.gebuehrenpflichtiger == null);

  function uebernehmen(g: GespeicherterBauantrag) {
    gespeichert = g;
    antrag = structuredClone(g.antrag);
  }

  /** Die Liste enthält alle Versionen; je Dokument zählt die neueste. */
  // Die Anlagen-Codeliste ist zweistufig: volle Hunderter sind Oberbegriffe,
  // die übrigen Codes werden darunter eingerückt.
  function einrueckung(code: string): string {
    return /^\d+$/.test(code) && !code.endsWith("00") ? "   " : "";
  }

  function neuesteVersionen(alle: DocumentVersion[]): DocumentVersion[] {
    const neueste = new Map<string, DocumentVersion>();
    for (const v of alle) {
      const bisher = neueste.get(v.document_id);
      if (!bisher || v.version > bisher.version) neueste.set(v.document_id, v);
    }
    return [...neueste.values()].sort((a, b) => a.file_name.localeCompare(b.file_name, "de"));
  }

  async function laden() {
    try {
      const [g, d, listen, b] = await Promise.all([
        api.bauantrag(projekt.id),
        api.dokumente(projekt.id),
        api.xbauCodelisten(),
        api.bauherren(projekt.id),
      ]);
      uebernehmen(g);
      bauherren = b;
      dokumente = neuesteVersionen(d);
      anlagenarten = listen.find((l) => l.uri === CODELISTE_ANLAGEN) ?? null;
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  $effect(() => {
    void projekt.id;
    untrack(laden);
  });

  // Bauherren geändert (die Projektversion steigt): Liste und fehlende
  // Angaben neu laden, ohne ungespeicherte Eingaben zu verwerfen.
  let geladeneVersion = untrack(() => projekt.version);
  $effect(() => {
    const v = projekt.version;
    if (v === geladeneVersion) return;
    geladeneVersion = v;
    untrack(async () => {
      try {
        const [g, b] = await Promise.all([api.bauantrag(projekt.id), api.bauherren(projekt.id)]);
        bauherren = b;
        if (!geaendert) uebernehmen(g);
        else if (gespeichert) gespeichert = { ...gespeichert, fehlende_angaben: g.fehlende_angaben };
      } catch (e) {
        onfehler(fehlertext(e));
      }
    });
  });

  async function kundenLaden() {
    if (kunden.length === 0) kunden = await api.kunden().catch(() => []);
  }

  async function speichern(): Promise<boolean> {
    if (!antrag || !gespeichert) return false;
    laeuft = true;
    try {
      uebernehmen(await api.bauantragSpeichern(projekt.id, antrag, gespeichert.version));
      return true;
    } catch (e) {
      if (istKonflikt(e)) {
        onfehler("Der Bauantrag wurde inzwischen an einem anderen Arbeitsplatz geändert. Die aktuelle Fassung wurde geladen; bitte Änderungen erneut eintragen.");
        await laden();
      } else {
        onfehler(fehlertext(e));
      }
      return false;
    } finally {
      laeuft = false;
    }
  }

  async function exportieren() {
    if (geaendert && !(await speichern())) return;
    if (gespeichert && gespeichert.fehlende_angaben.length > 0) {
      onfehler(`Für den XBau-Export fehlen noch: ${gespeichert.fehlende_angaben.join(", ")}.`);
      return;
    }
    const datum = new Date().toISOString().slice(0, 10);
    const ziel = await save({
      title: "Bauantrag als XBau exportieren",
      defaultPath: `Bauantrag ${projekt.name.replace(/[\\/:*?"<>|]/g, "_")} ${datum}.zip`,
      filters: [{ name: "XBau-Paket (ZIP)", extensions: ["zip"] }],
    });
    if (!ziel) return;
    laeuft = true;
    try {
      await api.xbauExportieren(projekt.id, ziel);
      onhinweis(`Bauantrag exportiert: ${ziel}`);
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      laeuft = false;
    }
  }

  // „Datei → Bauantrag als XBau exportieren …“
  let letzterAusloeser = untrack(() => exportAusloeser);
  $effect(() => {
    if (exportAusloeser > letzterAusloeser) {
      letzterAusloeser = exportAusloeser;
      untrack(() => {
        // Erst exportieren, wenn der Antrag geladen ist.
        if (antrag) exportieren();
        else laden().then(exportieren);
      });
    }
  });

  const neueId = () => crypto.randomUUID();
  const neuePerson = (): Beteiligter => ({
    art: "person",
    id: neueId(),
    vorname: "",
    familienname: "",
    anschrift: leereAnschrift(),
  });

  function massnahmeUmschalten(code: string, an: boolean) {
    if (!antrag) return;
    antrag.massnahmen = an
      ? [...antrag.massnahmen, code].sort()
      : antrag.massnahmen.filter((m) => m !== code);
  }

  const anlageGewaehlt = (id: string) => antrag?.anlagen.find((a) => a.dokument === id);

  function anlageUmschalten(d: DocumentVersion, an: boolean) {
    if (!antrag) return;
    antrag.anlagen = an
      ? [...antrag.anlagen, { dokument: d.document_id, anlageart: "", bezeichnung: "" }]
      : antrag.anlagen.filter((a) => a.dokument !== d.document_id);
  }

  const dokumentName = (id: string) => dokumente.find((d) => d.document_id === id)?.file_name ?? "(gelöscht)";
</script>

{#if antrag && gespeichert}
  <div class="bauantrag">
    <div class="knoepfe links">
      <button onclick={speichern} disabled={!darf || !geaendert || laeuft}>Speichern</button>
      <button class="sekundaer" onclick={() => antrag && gespeichert && (antrag = structuredClone(gespeichert.antrag))} disabled={!geaendert || laeuft}>Verwerfen</button>
      <button onclick={exportieren} disabled={laeuft}>Als XBau exportieren …</button>
      <button class="sekundaer" onclick={onbuero}>Büroangaben …</button>
    </div>
    <p class="meta">
      XBau 2.7, Nachricht „Beantragung einer Baugenehmigung“ (0200). Das Paket enthält die Nachricht und die ausgewählten Bauvorlagen.
      {#if gespeichert.geaendert_am}Zuletzt gespeichert {new Date(gespeichert.geaendert_am).toLocaleString("de-DE")} von {gespeichert.geaendert_von}.{/if}
    </p>
    {#if gespeichert.fehlende_angaben.length > 0}
      <div class="hinweisbox" role="status">
        <strong>Für den Export fehlt noch{geaendert ? " (Stand der letzten Speicherung)" : ""}:</strong>
        <ul>{#each gespeichert.fehlende_angaben as f}<li>{f}</li>{/each}</ul>
      </div>
    {/if}

    <fieldset disabled={!darf}>
      <legend>Bauvorhaben</legend>
      <label class="voll">Bezeichnung
        <input bind:value={antrag.bezeichnung} placeholder={projekt.name} />
      </label>
      <div class="massnahmen" role="group" aria-label="Art der Maßnahme">
        <span class="meta">Art der Maßnahme</span>
        {#each MASSNAHMEN as m}
          <label><input type="checkbox" checked={antrag.massnahmen.includes(m.code)} onchange={(e) => massnahmeUmschalten(m.code, e.currentTarget.checked)} /> {m.name}</label>
        {/each}
      </div>
      <label>Anzahl der Gebäude
        <input type="number" min="0" step="1" bind:value={antrag.anzahl_gebaeude} />
      </label>
    </fieldset>

    <fieldset disabled={!darf}>
      <legend>Bauaufsichtsbehörde</legend>
      <div class="anschrift">
        <label class="breit">Name <input bind:value={antrag.bauaufsicht.name} placeholder="z. B. Landratsamt …" /></label>
        <label class="breit">Behördenkennung (DVDV) <input bind:value={antrag.bauaufsicht.kennung} placeholder="z. B. bab:091620000" /></label>
      </div>
    </fieldset>

    <fieldset disabled={!darf}>
      <legend>Bauherren</legend>
      {#each bauherren as k (k.id)}
        <div>{kundenname(k)} <span class="meta">{anschriftZeile(k.anschrift)}</span></div>
      {:else}
        <p class="meta">Noch kein Bauherr zugeordnet.</p>
      {/each}
      <button type="button" class="sekundaer" onclick={onbauherren}>Bauherren aus der Kundendatenbank wählen …</button>
      <label class="zeile">
        <input
          type="checkbox"
          checked={gebuehrenBauherr}
          onchange={(e) => antrag && (antrag.gebuehrenpflichtiger = e.currentTarget.checked ? null : neuePerson())}
        />
        Der (erste) Bauherr zahlt die Gebühren
      </label>
      {#if antrag.gebuehrenpflichtiger}
        <div class="eintrag">
          <span class="meta">Gebührenpflichtig</span>
          <BeteiligterFelder bind:wer={antrag.gebuehrenpflichtiger} gesperrt={!darf} />
        </div>
        <label>Aus der Kundendatenbank übernehmen
          <select
            value=""
            onfocus={kundenLaden}
            onchange={(e) => {
              const k = kunden.find((x) => x.id === e.currentTarget.value);
              if (k && antrag) antrag.gebuehrenpflichtiger = alsBeteiligter(k);
              e.currentTarget.value = "";
            }}
          >
            <option value="">– Kunde wählen –</option>
            {#each kunden as k (k.id)}<option value={k.id}>{kundenname(k)}</option>{/each}
          </select>
        </label>
      {/if}
    </fieldset>

    <fieldset disabled={!darf}>
      <legend>Baugrundstück</legend>
      <AnschriftFelder bind:anschrift={antrag.grundstueck.anschrift} gesperrt={!darf} />
      <table class="ergebnistabelle">
        <thead><tr><th>Gemarkung (Name oder Nummer)</th><th>Flur</th><th>Flurstück Zähler</th><th>Nenner</th><th></th></tr></thead>
        <tbody>
          {#each antrag.grundstueck.flurstuecke as f, i}
            <tr>
              <td><input bind:value={f.gemarkung} aria-label="Gemarkung" /></td>
              <td><input bind:value={f.flur} size="4" aria-label="Flur" /></td>
              <td><input bind:value={f.zaehler} size="6" aria-label="Zähler" /></td>
              <td><input bind:value={f.nenner} size="4" aria-label="Nenner" /></td>
              <td><button type="button" class="sekundaer" onclick={() => antrag && antrag.grundstueck.flurstuecke.splice(i, 1)}>Entfernen</button></td>
            </tr>
          {/each}
        </tbody>
      </table>
      <button type="button" class="sekundaer" onclick={() => antrag?.grundstueck.flurstuecke.push({ gemarkung: "", flur: "", zaehler: "", nenner: "" })}>Flurstück hinzufügen</button>
    </fieldset>

    <fieldset disabled={!darf}>
      <legend>Anlagen (Bauvorlagen aus den Dokumenten)</legend>
      {#if !anlagenarten}
        <p class="fehlertext">Die Codeliste der Anlagenarten fehlt. Ohne sie kann die Art der Anlage nur als Code eingetragen werden, und der Export ist nicht möglich.</p>
      {/if}
      {#if dokumente.length === 0}
        <p class="meta">Noch keine Dokumente im Projekt. Bauvorlagen zuerst unter „Dokumente“ ablegen.</p>
      {:else}
        <table class="ergebnistabelle">
          <thead><tr><th></th><th>Dokument</th><th>Art der Anlage</th><th>Bezeichnung für die Behörde</th></tr></thead>
          <tbody>
            {#each dokumente as d (d.document_id)}
              {@const a = anlageGewaehlt(d.document_id)}
              <tr>
                <td><input type="checkbox" checked={!!a} onchange={(e) => anlageUmschalten(d, e.currentTarget.checked)} aria-label="{d.file_name} beifügen" /></td>
                <td>{d.file_name} <span class="meta">(Version {d.version})</span></td>
                <td>
                  {#if a}
                    {#if anlagenarten}
                      <select class="anlageart" bind:value={a.anlageart} aria-label="Art der Anlage">
                        <option value="">– bitte wählen –</option>
                        {#each anlagenarten.eintraege as e}<option value={e.code} title={e.beschreibung}>{einrueckung(e.code)}{e.code} {e.beschreibung}</option>{/each}
                      </select>
                    {:else}
                      <input bind:value={a.anlageart} size="6" placeholder="Code" aria-label="Art der Anlage (Code)" />
                    {/if}
                  {/if}
                </td>
                <td>{#if a}<input bind:value={a.bezeichnung} placeholder={d.file_name} aria-label="Bezeichnung" />{/if}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
      {#each antrag.anlagen.filter((a) => !dokumente.some((d) => d.document_id === a.dokument)) as a}
        <p class="fehlertext">Ausgewählte Anlage {dokumentName(a.dokument)} ist nicht mehr im Projekt.</p>
      {/each}
    </fieldset>
  </div>
{:else}
  <p class="meta">Wird geladen …</p>
{/if}
