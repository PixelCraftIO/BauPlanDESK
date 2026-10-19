<script lang="ts">
  import { editorAnmelden } from "./editoren";
  import { projektNavigation } from "./projektNavigation.svelte";
  import { untrack } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import {
    CODELISTE_ANLAGEN,
    MASSNAHMEN,
    alsBeteiligter,
    anschriftZeile,
    api,
    entwurfsverfasserName,
    beteiligtenrolleName,
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
    type Entwurfsverfasser,
    type GespeicherterBauantrag,
    type Kunde,
    type Project,
    type Projektbeteiligter,
  } from "./api";
  import AnschriftFelder from "./AnschriftFelder.svelte";
  import { abonnieren, bueroAbonnieren } from "./projektEreignisse";
  import BeteiligterFelder from "./BeteiligterFelder.svelte";

  let {
    projekt,
    benutzer,
    onbuero,
    onbauherren,
    onbeteiligte,
    onfehler,
  }: {
    projekt: Project;
    benutzer: Benutzer;
    /** Dialog „Büroangaben“ öffnen. */
    onbuero: () => void;
    /** Dialog „Bauherren“ öffnen. */
    onbauherren: () => void;
    /** Dialog „Weitere Beteiligte“ öffnen. */
    onbeteiligte: () => void;
    onfehler: (m: string) => void;
  } = $props();

  const darf = $derived(
    benutzer.aktiv && mindestens(benutzer.rolle, "bearbeitung"),
  );

  const navigation = projektNavigation();
  let gespeichert = $state<GespeicherterBauantrag | null>(null);
  let antrag = $state<Bauantrag | null>(null);
  let dokumente = $state<DocumentVersion[]>([]);
  let anlagenarten = $state<Codeliste | null>(null);
  let bauherren = $state<Kunde[]>([]);
  let beteiligte = $state<Projektbeteiligter[]>([]);
  let kunden = $state<Kunde[]>([]);
  let entwurfsverfasser = $state<Entwurfsverfasser[]>([]);
  let laeuft = $state(false);
  /** Ladefehler je Auswahlliste: nicht still als leere Liste anzeigen. */
  let ladefehler = $state<{
    beteiligte?: string;
    kunden?: string;
    buero?: string;
    dokumente?: string;
  }>({});
  const beteiligteLesen = () =>
    api.beteiligte(projekt.id).then(
      (t) => {
        ladefehler.beteiligte = undefined;
        return t;
      },
      (e) => {
        ladefehler.beteiligte = fehlertext(e);
        return beteiligte;
      },
    );

  const geaendert = $derived(
    !!antrag &&
      !!gespeichert &&
      JSON.stringify(antrag) !== JSON.stringify(gespeichert.antrag),
  );
  const gebuehrenBauherr = $derived(antrag?.gebuehrenpflichtiger == null);

  function uebernehmen(g: GespeicherterBauantrag) {
    // Ältere Anträge ohne Verfahrensart: wie die Auswahl „keine Angabe“, damit
    // die Auswahlliste den Antrag nicht als geändert erscheinen lässt.
    g.antrag.verfahrensart ??= "";
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
    return [...neueste.values()].sort((a, b) =>
      a.file_name.localeCompare(b.file_name, "de"),
    );
  }

  async function laden() {
    try {
      const [g, d, listen, b, t] = await Promise.all([
        api.bauantrag(projekt.id),
        api.dokumente(projekt.id),
        api.xbauCodelisten(),
        api.bauherren(projekt.id),
        beteiligteLesen(),
        entwurfsverfasserLaden(),
      ]);
      uebernehmen(g);
      bauherren = b;
      beteiligte = t;
      dokumente = neuesteVersionen(d);
      ladefehler.dokumente = undefined;
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
    untrack(stammdatenAbgleichen);
  });
  async function stammdatenAbgleichen() {
    try {
      const [g, b, t] = await Promise.all([
        api.bauantrag(projekt.id),
        api.bauherren(projekt.id),
        beteiligteLesen(),
      ]);
      bauherren = b;
      beteiligte = t;
      if (!geaendert) uebernehmen(g);
      else if (gespeichert)
        gespeichert = {
          ...gespeichert,
          fehlende_angaben: g.fehlende_angaben,
          hinweise: g.hinweise,
        };
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }
  // Büroangaben gespeichert (die Projektversion ändert sich dabei nicht):
  // Entwurfsverfasser und fehlende Angaben neu ermitteln.
  $effect(() =>
    bueroAbonnieren(() => {
      void entwurfsverfasserLaden();
      void stammdatenAbgleichen();
    }),
  );
  // Andere Bereiche haben Dokumente abgelegt: Anlagenauswahl aktualisieren.
  $effect(() =>
    abonnieren(projekt.id, ["dokumente"], () => void dokumenteLaden()),
  );
  async function dokumenteLaden() {
    try {
      dokumente = neuesteVersionen(await api.dokumente(projekt.id));
      ladefehler.dokumente = undefined;
    } catch (e) {
      ladefehler.dokumente = fehlertext(e);
    }
  }

  async function neuLaden() {
    if (
      geaendert &&
      !(await ask(
        "Ungespeicherte Antragsdaten verwerfen und den aktuellen Stand laden?",
        {
          title: "Bauantrag",
          kind: "warning",
          okLabel: "Stand laden",
          cancelLabel: "Weiter bearbeiten",
        },
      ))
    )
      return;
    laeuft = true;
    try {
      await laden();
    } finally {
      laeuft = false;
    }
  }
  // Die Büroangaben können sich im Dialog geändert haben. Fehlen sie, bleibt
  // die Maske nutzbar; die fehlenden Angaben nennen den Entwurfsverfasser.
  async function entwurfsverfasserLaden() {
    try {
      entwurfsverfasser = (await api.buero()).entwurfsverfasser ?? [];
      ladefehler.buero = undefined;
    } catch (e) {
      ladefehler.buero = fehlertext(e);
    }
  }
  async function kundenLaden() {
    if (kunden.length > 0) return;
    try {
      kunden = await api.kunden();
      ladefehler.kunden = undefined;
    } catch (e) {
      ladefehler.kunden = fehlertext(e);
    }
  }

  async function speichern(): Promise<boolean> {
    if (!antrag || !gespeichert) return false;
    laeuft = true;
    try {
      uebernehmen(
        await api.bauantragSpeichern(projekt.id, antrag, gespeichert.version),
      );
      return true;
    } catch (e) {
      if (istKonflikt(e)) {
        onfehler(
          "Der Bauantrag wurde inzwischen an einem anderen Arbeitsplatz geändert. Ihre Eingaben bleiben erhalten; bitte den aktuellen Stand abgleichen.",
        );
      } else {
        onfehler(fehlertext(e));
      }
      return false;
    } finally {
      laeuft = false;
    }
  }

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

  const anlageGewaehlt = (id: string) =>
    antrag?.anlagen.find((a) => a.dokument === id);

  function anlageUmschalten(d: DocumentVersion, an: boolean) {
    if (!antrag) return;
    antrag.anlagen = an
      ? [
          ...antrag.anlagen,
          { dokument: d.document_id, anlageart: "", bezeichnung: "" },
        ]
      : antrag.anlagen.filter((a) => a.dokument !== d.document_id);
  }

  const dokumentName = (id: string) =>
    dokumente.find((d) => d.document_id === id)?.file_name ?? "(gelöscht)";

  export function hatAenderungen() {
    return geaendert;
  }
  export function istBeschaeftigt() {
    return laeuft;
  }
  export async function speichernBeimSchliessen(): Promise<boolean> {
    if (istBeschaeftigt()) return false;
    await speichern();
    return !hatAenderungen();
  }
  editorAnmelden("bauantrag", {
    hatAenderungen,
    istBeschaeftigt,
    speichern: speichernBeimSchliessen,
  });
</script>

{#if antrag && gespeichert}
  <div class="bauantrag">
    <div class="knoepfe formularaktionen">
      <button class="sekundaer" onclick={onbuero}>Büroangaben …</button>
      <button class="sekundaer" onclick={neuLaden} disabled={laeuft}
        >Aktuellen Stand laden</button
      >
      {#if navigation}<button
          class="sekundaer"
          onclick={() =>
            navigation.springen({
              bereich: "verfahren",
              objekt: null,
              register: null,
              eintrag: null,
            })}>Zur Abgabe in der Verfahrensakte …</button
        >{/if}
      <span class="werkzeug-trenner" aria-hidden="true"></span>
      <button
        class="sekundaer"
        onclick={() =>
          antrag &&
          gespeichert &&
          (antrag = structuredClone(gespeichert.antrag))}
        disabled={!geaendert || laeuft}>Verwerfen</button
      >
      <button onclick={speichern} disabled={!darf || !geaendert || laeuft}
        >Speichern</button
      >
    </div>
    <p class="meta">
      Die Angaben gelten für jeden Abgabestand der Verfahrensakte. Dort entstehen
      XBau-Nachricht und Druck nach der Verfahrensart, mit Behörde und
      Aktenzeichen der Akte.
      {#if gespeichert.geaendert_am}Zuletzt gespeichert {new Date(
          gespeichert.geaendert_am,
        ).toLocaleString("de-DE")} von {gespeichert.geaendert_von}.{/if}
    </p>
    {#if gespeichert.fehlende_angaben.length > 0}
      <div class="hinweisbox" role="status">
        <strong
          >Für die Abgabe fehlt noch{geaendert
            ? " (Stand der letzten Speicherung)"
            : ""}:</strong
        >
        <ul>
          {#each gespeichert.fehlende_angaben as f}<li>{f}</li>{/each}
        </ul>
      </div>
    {/if}
    {#if gespeichert.hinweise?.length}
      <div class="hinweisbox" role="status">
        <strong>Bitte prüfen (sperrt die Abgabe nicht):</strong>
        <ul>
          {#each gespeichert.hinweise as h}<li>{h}</li>{/each}
        </ul>
      </div>
    {/if}

    <fieldset disabled={!darf}>
      <legend>Bauvorhaben</legend>
      <label class="voll"
        >Bezeichnung
        <input bind:value={antrag.bezeichnung} placeholder={projekt.name} />
      </label>
      <div class="massnahmen" role="group" aria-label="Art der Maßnahme">
        <span class="meta">Art der Maßnahme</span>
        {#each MASSNAHMEN as m}
          <label
            ><input
              type="checkbox"
              checked={antrag.massnahmen.includes(m.code)}
              onchange={(e) =>
                massnahmeUmschalten(m.code, e.currentTarget.checked)}
            />
            {m.name}</label
          >
        {/each}
      </div>
      <label
        >Anzahl der Gebäude
        <input
          type="number"
          min="0"
          step="1"
          bind:value={antrag.anzahl_gebaeude}
          readonly={!!gespeichert.gebaeude_zentral}
        />
      </label>
      {#if gespeichert.gebaeude_zentral}<p class="meta">
          Aus Gebäude/Geschosse: Anzahl der dort erfassten Gebäude.
        </p>{/if}
      {#if gespeichert.verfahrensarten}
        <label
          >Verfahrensart ({gespeichert.verfahrensarten.name})
          <select bind:value={antrag.verfahrensart}>
            <option value="">keine Angabe</option>
            {#each gespeichert.verfahrensarten.eintraege as e}
              <option value={e.code}>{e.beschreibung || e.code}</option>
            {/each}
          </select>
        </label>
      {:else if antrag.verfahrensart}
        <p class="meta">
          Verfahrensart {antrag.verfahrensart}: Die Codeliste des Landes ist
          nicht geladen.
          <button type="button" onclick={() => (antrag!.verfahrensart = "")}
            >Angabe entfernen</button
          >
        </p>
      {:else}
        <p class="meta">
          Verfahrensart: Für die Angabe in der XBau-Nachricht die Codeliste
          des Landes aus dem XRepository in den Codelisten-Ordner legen.
        </p>
      {/if}
    </fieldset>

    <fieldset disabled={!darf}>
      <legend>Bauaufsichtsbehörde</legend>
      <p class="meta">
        Für Abgaben gilt die Behörde der Verfahrensakte. Diese Angabe lässt sich
        dort übernehmen.
      </p>
      <div class="feldzeilen">
        <label class="breit"
          >Name <input
            bind:value={antrag.bauaufsicht.name}
            placeholder="z. B. Landratsamt …"
          /></label
        >
        <label class="breit"
          >Behördenkennung (DVDV) <input
            bind:value={antrag.bauaufsicht.kennung}
            placeholder="z. B. bab:091620000"
          /></label
        >
      </div>
    </fieldset>

    <fieldset disabled={!darf}>
      <legend>Bauherren</legend>
      {#each bauherren as k (k.id)}
        <div>
          {kundenname(k)}
          <span class="meta">{anschriftZeile(k.anschrift)}</span>
        </div>
      {:else}
        <p class="meta">Noch kein Bauherr zugeordnet.</p>
      {/each}
      <button type="button" class="sekundaer" onclick={onbauherren}
        >Bauherren aus der Kundendatenbank wählen …</button
      >
      {#each beteiligte as b (`${b.rolle}:${b.kunde.id}`)}
        <div>
          {kundenname(b.kunde)}
          <span class="meta">{beteiligtenrolleName(b.rolle)}</span>
        </div>
      {/each}
      {#if ladefehler.beteiligte}<p class="fehlertext" role="alert">
          Weitere Beteiligte konnten nicht geladen werden: {ladefehler.beteiligte}
        </p>{/if}
      <button type="button" class="sekundaer" onclick={onbeteiligte}
        >Eigentümer und weitere Beteiligte …</button
      >
      <label class="zeile">
        <input
          type="checkbox"
          checked={gebuehrenBauherr}
          onchange={(e) =>
            antrag &&
            (antrag.gebuehrenpflichtiger = e.currentTarget.checked
              ? null
              : neuePerson())}
        />
        Der (erste) Bauherr zahlt die Gebühren
      </label>
      {#if antrag.gebuehrenpflichtiger}
        <div class="eintrag">
          <span class="meta">Gebührenpflichtig</span>
          <BeteiligterFelder
            bind:wer={antrag.gebuehrenpflichtiger}
            gesperrt={!darf}
          />
        </div>
        <label
          >Aus der Kundendatenbank übernehmen
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
            {#each kunden as k (k.id)}<option value={k.id}
                >{kundenname(k)}</option
              >{/each}
          </select>
        </label>
        {#if ladefehler.kunden}<p class="fehlertext" role="alert">
            Kunden konnten nicht geladen werden: {ladefehler.kunden}
          </p>{/if}
      {/if}
    </fieldset>

    <fieldset disabled={!darf}>
      <legend>Entwurfsverfasser</legend>
      {#if ladefehler.buero}<p class="fehlertext" role="alert">
          Büroangaben konnten nicht geladen werden: {ladefehler.buero}
        </p>{/if}
      {#if entwurfsverfasser.length === 0}
        <p class="meta">
          In den Büroangaben ist noch keine bauvorlageberechtigte Person
          eingetragen.
        </p>
      {:else}
        <label
          >Bauvorlageberechtigte Person
          <select
            value={antrag.entwurfsverfasser ?? ""}
            onfocus={entwurfsverfasserLaden}
            onchange={(e) =>
              antrag &&
              (antrag.entwurfsverfasser = e.currentTarget.value || null)}
          >
            <option value=""
              >{entwurfsverfasser.length === 1
                ? `Einzige Person des Büros (${entwurfsverfasserName(entwurfsverfasser[0])})`
                : "– Person wählen –"}</option
            >
            {#if antrag.entwurfsverfasser && !entwurfsverfasser.some((e) => e.id === antrag?.entwurfsverfasser)}
              <option value={antrag.entwurfsverfasser}
                >Nicht mehr in den Büroangaben</option
              >
            {/if}
            {#each entwurfsverfasser as e (e.id)}<option value={e.id}
                >{entwurfsverfasserName(e)}</option
              >{/each}
          </select>
        </label>
      {/if}
      <button type="button" class="sekundaer" onclick={onbuero}
        >Personen in den Büroangaben pflegen …</button
      >
    </fieldset>

    <fieldset disabled={!darf}>
      <legend>Baugrundstück</legend>
      <AnschriftFelder
        bind:anschrift={antrag.grundstueck.anschrift}
        gesperrt={!darf}
      />
      {#if gespeichert?.flurstuecke_zentral}
        <p class="meta">
          Die Flurstücke stammen aus den Grundstücksinformationen des Projekts
          und gelten unverändert für Antrag und Abgabe.
        </p>
        <table class="ergebnistabelle">
          <thead
            ><tr
              ><th>Gemarkung</th><th>Flur</th><th>Flurstück Zähler</th><th
                >Nenner</th
              ></tr
            ></thead
          >
          <tbody>
            {#each antrag.grundstueck.flurstuecke as f}
              <tr
                ><td>{f.gemarkung}</td><td>{f.flur || "–"}</td><td
                  >{f.zaehler}</td
                ><td>{f.nenner || "–"}</td></tr
              >
            {/each}
          </tbody>
        </table>
        {#if navigation}<button
            type="button"
            class="sekundaer"
            onclick={() =>
              navigation.springen({
                bereich: "flurstuecke",
                objekt: null,
                register: null,
                eintrag: null,
              })}>Flurstücke bearbeiten</button
          >{/if}
      {:else}
      <table class="ergebnistabelle">
        <thead
          ><tr
            ><th>Gemarkung (Name oder Nummer)</th><th>Flur</th><th
              >Flurstück Zähler</th
            ><th>Nenner</th><th></th></tr
          ></thead
        >
        <tbody>
          {#each antrag.grundstueck.flurstuecke as f, i}
            <tr>
              <td><input bind:value={f.gemarkung} aria-label="Gemarkung" /></td>
              <td><input bind:value={f.flur} size="4" aria-label="Flur" /></td>
              <td
                ><input
                  bind:value={f.zaehler}
                  size="6"
                  aria-label="Zähler"
                /></td
              >
              <td
                ><input
                  bind:value={f.nenner}
                  size="4"
                  aria-label="Nenner"
                /></td
              >
              <td
                ><button
                  type="button"
                  class="sekundaer"
                  onclick={() =>
                    antrag && antrag.grundstueck.flurstuecke.splice(i, 1)}
                  >Entfernen</button
                ></td
              >
            </tr>
          {/each}
        </tbody>
      </table>
      <button
        type="button"
        class="sekundaer"
        onclick={() =>
          antrag?.grundstueck.flurstuecke.push({
            gemarkung: "",
            flur: "",
            zaehler: "",
            nenner: "",
          })}>Flurstück hinzufügen</button
      >
      {/if}
    </fieldset>

    <fieldset disabled={!darf}>
      <legend>Anlagen (Bauvorlagen aus den Dokumenten)</legend>
      {#if !anlagenarten}
        <p class="fehlertext">
          Die Codeliste der Anlagenarten fehlt. Ohne sie kann die Art der Anlage
          nur als Code eingetragen werden, und eine XBau-Abgabe ist nicht möglich.
        </p>
      {/if}
      {#if ladefehler.dokumente}<p class="fehlertext" role="alert">
          Die Dokumentliste konnte nicht aktualisiert werden: {ladefehler.dokumente}
        </p>{/if}
      {#if dokumente.length === 0}
        <p class="meta">
          Noch keine Dokumente im Projekt. Bauvorlagen zuerst unter „Dokumente“
          ablegen.
        </p>
      {:else}
        <table class="ergebnistabelle">
          <thead
            ><tr
              ><th></th><th>Dokument</th><th>Art der Anlage</th><th
                >Bezeichnung für die Behörde</th
              ></tr
            ></thead
          >
          <tbody>
            {#each dokumente as d (d.document_id)}
              {@const a = anlageGewaehlt(d.document_id)}
              <tr>
                <td
                  ><input
                    type="checkbox"
                    checked={!!a}
                    onchange={(e) =>
                      anlageUmschalten(d, e.currentTarget.checked)}
                    aria-label="{d.file_name} beifügen"
                  /></td
                >
                <td
                  >{d.file_name}
                  <span class="meta">(Version {d.version})</span></td
                >
                <td>
                  {#if a}
                    {#if anlagenarten}
                      <select
                        class="anlageart"
                        bind:value={a.anlageart}
                        aria-label="Art der Anlage"
                      >
                        <option value="">– bitte wählen –</option>
                        {#each anlagenarten.eintraege as e}<option
                            value={e.code}
                            title={e.beschreibung}
                            >{einrueckung(e.code)}{e.code}
                            {e.beschreibung}</option
                          >{/each}
                      </select>
                    {:else}
                      <input
                        bind:value={a.anlageart}
                        size="6"
                        placeholder="Code"
                        aria-label="Art der Anlage (Code)"
                      />
                    {/if}
                  {/if}
                </td>
                <td
                  >{#if a}<input
                      bind:value={a.bezeichnung}
                      placeholder={d.file_name}
                      aria-label="Bezeichnung"
                    />{/if}</td
                >
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
      {#each antrag.anlagen.filter((a) => !dokumente.some((d) => d.document_id === a.dokument)) as a}
        <p class="fehlertext">
          Ausgewählte Anlage {dokumentName(a.dokument)} ist nicht mehr im Projekt.
        </p>
      {/each}
    </fieldset>
  </div>
{:else}
  <p class="meta">Wird geladen …</p>
{/if}
