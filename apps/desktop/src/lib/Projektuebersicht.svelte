<script lang="ts">
  import Arbeitscheck from "./arbeitsplatz/Arbeitscheck.svelte";
  import Statushilfe from "./arbeitsplatz/Statushilfe.svelte";
  import { abonnieren } from "./projektEreignisse";
  import { onDestroy, untrack } from "svelte";
  import { plattformApi } from "./plattformApi";
  import { fehlertext } from "./api";
  import { invoke } from "@tauri-apps/api/core";
  import { save } from "@tauri-apps/plugin-dialog";
  import type {
    Projektuebersicht,
    UebersichtsZiel,
    Themenbereich,
    Dringlichkeit,
  } from "./generated/plattform";
  let {
    projektId,
    version,
    sichtbar,
    onoeffnen,
    entwuerfe = false,
    benutzerId = "",
  }: {
    projektId: string;
    version: number;
    sichtbar: boolean;
    entwuerfe?: boolean;
    benutzerId?: string;
    onoeffnen: (z: UebersichtsZiel) => void;
  } = $props();
  let daten = $state<Projektuebersicht | null>(null),
    fehler = $state<string | null>(null),
    laeuft = $state(false),
    thema = $state(""),
    dringlichkeit = $state(""),
    suche = $state(""),
    zustaendigkeit = $state(""),
    registerFilter = $state(""),
    anzahl = $state(20);
  let anfrage = 0;
  const themen: Record<Themenbereich, string> = {
    grundlagen: "Grundlagen",
    aufgaben: "Aufgaben",
    unterlagen: "Unterlagen",
    nachweise: "Nachweise",
    verfahren: "Verfahren",
    einreichungen: "Einreichungen",
  };
  const dringlich: Record<Dringlichkeit, string> = {
    ueberfaellig: "Überfällig",
    heute: "Heute fällig",
    handeln: "Zu erledigen",
    pruefen: "Zu prüfen",
    warten: "Rückmeldung ausstehend",
  };
  const entscheidungen = {
    genehmigt: "Genehmigt",
    teilweise_genehmigt: "Teilweise genehmigt",
    abgelehnt: "Abgelehnt",
    sonstiges: "Sonstige Entscheidung",
  };
  const datum = (s: string | null) =>
    s ? new Date(`${s}T12:00:00`).toLocaleDateString("de-DE", { day: "2-digit", month: "2-digit", year: "numeric" }) : "—";
  const ziel = (bereich: string): UebersichtsZiel => ({
    bereich,
    objekt: null,
    register: null,
    eintrag: null,
  });
  async function laden() {
    const nr = ++anfrage,
      id = projektId;
    laeuft = true;
    fehler = null;
    try {
      const neu = await plattformApi.uebersicht(id);
      if (nr === anfrage && id === projektId) {
        if (neu.projekt_id !== id)
          throw Error("Antwort gehört zu einem anderen Projekt");
        daten = neu;
      }
    } catch (e) {
      if (nr === anfrage && id === projektId) fehler = fehlertext(e);
    } finally {
      if (nr === anfrage && id === projektId) laeuft = false;
    }
  }
  $effect(() => {
    void version;
    const id = projektId;
    untrack(() => {
      if (daten && daten.projekt_id !== id) daten = null;
    });
    if (sichtbar) {
      void laden();
      const timer = setInterval(() => {
        if (!laeuft) void laden();
      }, 60000);
      return () => {
        clearInterval(timer);
        anfrage++;
        laeuft = false;
      };
    }
  });
  onDestroy(() => anfrage++);
  $effect(() => abonnieren(projektId, ["dokumente", "fachobjekte", "nachweise"], () => { if (sichtbar) void laden(); }));
  const gefiltert = $derived(
    (daten?.punkte ?? []).filter(
      (p) =>
        (!zustaendigkeit || (zustaendigkeit === "ich" ? p.verantwortlich === benutzerId : !p.verantwortlich)) &&
        (!thema || p.thema === thema) &&
        (!registerFilter || p.ziel.register === registerFilter) &&
        (!dringlichkeit || p.dringlichkeit === dringlichkeit) &&
        (!suche ||
          `${p.titel} ${p.hinweis}`
            .toLocaleLowerCase("de-DE")
            .includes(suche.toLocaleLowerCase("de-DE"))),
    ),
  );
  const naechster = $derived(
    daten?.punkte.find((p) => p.dringlichkeit !== "warten"),
  );
  const fristen = $derived(
    (daten?.punkte ?? [])
      .filter((p) => p.faellig)
      .sort((a, b) => a.faellig!.localeCompare(b.faellig!))
      .slice(0, 8),
  );
  function filtern(t: string, d = "", r = "") {
    registerFilter = r;
    thema = t;
    dringlichkeit = d;
    anzahl = 20;
  }
  let berichtMeldung = $state<string | null>(null);
  /** Sachstandsbericht für den Bauherrn (ohne bürointerne Notizen). */
  async function sachstandsbericht() {
    const ziel = await save({
      defaultPath: "Sachstandsbericht.pdf",
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (!ziel) return;
    try {
      await invoke("sachstandsbericht_pdf", { projektId, ziel });
      berichtMeldung = `Sachstandsbericht ausgegeben: ${ziel}`;
    } catch (e) {
      berichtMeldung = fehlertext(e);
    }
  }
</script>

<section
  class="projektuebersicht"
  aria-label="Zentrale Projektübersicht"
  aria-busy={laeuft}
>
  <div class="kopf">
    <div>
      <h2>Projektübersicht</h2>
      <p class="meta">
        Arbeitsstand, nächste Schritte und dokumentierter Behördenstand
      </p>
    </div>
    <div class="knoepfe">
      <button
        type="button"
        class="sekundaer"
        onclick={sachstandsbericht}
        title="Verfahrensstand, eingereichte Unterlagen, Nachforderungen und Fristen als PDF für den Bauherrn"
        >Sachstandsbericht für Bauherrn …</button
      >
      <button type="button" class="sekundaer" disabled={laeuft} onclick={laden}
        >{laeuft ? "Wird aktualisiert …" : "Übersicht aktualisieren"}</button
      >
    </div>
  </div>
  {#if berichtMeldung}<p role="status">{berichtMeldung}</p>{/if}
  {#if fehler}<div class="fehler" role="alert">
      Übersicht konnte nicht aktualisiert werden: {fehler}{daten
        ? " Angezeigt wird der letzte erfolgreich geladene Stand."
        : " Es liegt kein auswertbarer Stand vor."}
    </div>{/if}
  {#if entwuerfe}<p class="hinweisbox" role="status">
      In Projektbereichen liegen ungespeicherte Änderungen vor. Diese sind im
      angezeigten Arbeitsstand noch nicht berücksichtigt.
    </p>{/if}
  {#if daten}
    <div class="kennzahlen" aria-label="Arbeitsstand nach Bereichen">
      {#each [{ titel: "Offene Aufgaben", wert: daten.zahlen.offene_aufgaben, thema: "aufgaben" }, { titel: "Unterlagen klären", wert: daten.zahlen.fehlende_unterlagen, thema: "unterlagen" }, { titel: "Nachweise prüfen", wert: daten.zahlen.nachweise_pruefen, thema: "nachweise" }, { titel: "Nachforderungen offen", wert: daten.zahlen.offene_nachforderungen, thema: "verfahren", register: "nachforderungen" }, { titel: "Auflagen offen", wert: daten.zahlen.offene_auflagen, thema: "verfahren", register: "bescheide" }, { titel: "Fristen überfällig", wert: daten.zahlen.ueberfaellig, thema: "", dringlich: "ueberfaellig" }] as k}
        <button
          type="button"
          class="kennzahl"
          class:warnung={k.dringlich && k.wert > 0}
          onclick={() => filtern(k.thema, k.dringlich ?? "", k.register ?? "")}
          ><span>{k.titel}</span><strong>{k.wert}</strong><span class="meta"
            >Einträge anzeigen →</span
          ></button
        >
      {/each}
    </div>
    <div class="naechster">
      <div>
        <span class="meta">Nächster Arbeitsschritt</span>{#if naechster}<h3>
            {naechster.titel}
          </h3>
          <p>{naechster.hinweis}</p>{:else}<h3>
            Keine offenen Arbeitsschritte aus den erfassten Daten
          </h3>
          <p>
            Anforderungen und Vorhabenprofil prüfen. Daraus folgt keine
            automatische Vollständigkeit des Bauantrags.
          </p>{/if}
      </div>
      {#if naechster}<button
          type="button"
          onclick={() => onoeffnen(naechster.ziel)}
          >Arbeitsschritt öffnen</button
        >{:else}<button
          type="button"
          class="sekundaer"
          onclick={() => onoeffnen(ziel("verfahren"))}
          >Verfahrensakte öffnen</button
        >{/if}
    </div>
    <Arbeitscheck {daten} {entwuerfe} {onoeffnen} kompakt/>
    <div class="arbeitsraster">
      <section class="arbeitsbedarf" aria-label="Handlungsbedarf">
        <h3>Handlungsbedarf</h3>
        <div class="filter">
          <label>Zuständigkeit<select aria-label="Zuständigkeit" bind:value={zustaendigkeit} onchange={()=>anzahl=20}><option value="">Alle Zuständigkeiten</option><option value="ich" disabled={!benutzerId}>Mir zugewiesen</option><option value="offen">Ohne Zuweisung</option></select></label>
          <label
            >Bereich<select
              aria-label="Bereich"
              bind:value={thema}
              onchange={() => {
                anzahl = 20;
                registerFilter = "";
              }}
              ><option value="">Alle Bereiche</option
              >{#each Object.entries(themen) as [id, t]}<option value={id}
                  >{t}</option
                >{/each}</select
            ></label
          ><label
            >Dringlichkeit<select
              aria-label="Dringlichkeit"
              bind:value={dringlichkeit}
              onchange={() => {
                anzahl = 20;
                registerFilter = "";
              }}
              ><option value="">Alle Einträge</option
              >{#each Object.entries(dringlich) as [id, t]}<option value={id}
                  >{t}</option
                >{/each}</select
            ></label
          ><label class="suchfeld"
            >Suchen<input
              type="search"
              bind:value={suche}
              oninput={() => (anzahl = 20)}
              placeholder="Arbeitsschritt oder Hinweis"
            /></label
          >
        </div>
        {#if thema || dringlichkeit || registerFilter || suche || zustaendigkeit}<button type="button" class="link" onclick={()=>{filtern("");suche="";zustaendigkeit="";}}>Alle Filter zurücksetzen</button>{/if}
        {#if registerFilter}<p class="meta">
            Nur {registerFilter === "bescheide"
              ? "Auflagen"
              : "Nachforderungen"} ·
            <button
              type="button"
              class="link"
              onclick={() => (registerFilter = "")}
              >Zusatzfilter aufheben</button
            >
          </p>{/if}
        <p class="meta" role="status">
          {gefiltert.length} von {daten.punkte.length} Einträgen
        </p>
        <ol class="punktliste">
          {#each gefiltert.slice(0, anzahl) as p (p.id)}<li
              class:dringend={p.dringlichkeit === "ueberfaellig"}
            >
              <div class="punkt-meta">
                <span
                  class="status"
                  class:warnung={p.dringlichkeit === "ueberfaellig" ||
                    p.dringlichkeit === "heute"}
                  >{dringlich[p.dringlichkeit]}</span
                ><span class="meta"
                  >{themen[p.thema]}{p.faellig
                    ? ` · ${datum(p.faellig)}`
                    : ""}</span
                >
              </div>
              <div>
                <strong>{p.titel}</strong>
                <p>{p.hinweis}</p>
                <p class="meta">Zuständigkeit: {p.verantwortlich === benutzerId ? "Mir zugewiesen" : p.verantwortlich ? "Zugewiesen – Person im Eintrag anzeigen" : "Noch nicht zugewiesen"}</p>
              </div>
              <button
                type="button"
                class="sekundaer"
                aria-label={`${p.titel} öffnen`}
                onclick={() => onoeffnen(p.ziel)}>Öffnen →</button
              >
            </li>{:else}<li class="leer">
              {daten.punkte.length
                ? "Keine Einträge für diesen Filter."
                : "Keine offenen Einträge aus den erfassten Daten."}
            </li>{/each}
        </ol>
        {#if gefiltert.length > anzahl}<button
            type="button"
            class="sekundaer"
            onclick={() => (anzahl += 20)}>Weitere Einträge anzeigen</button
          >{/if}
      </section>
      <aside class="termine" aria-label="Nächste Fristen">
        <h3>Fristen im Blick</h3>
        {#if fristen.length}<ul>
            {#each fristen as f (f.id)}<li>
                <span class:warnung={f.dringlichkeit === "ueberfaellig"}
                  >{datum(f.faellig)} · {dringlich[f.dringlichkeit]}</span
                ><button
                  type="button"
                  class="link"
                  onclick={() => onoeffnen(f.ziel)}>{f.titel}</button
                >
              </li>{/each}
          </ul>{:else}<p class="meta">
            Keine offenen datierten Fristen erfasst.
          </p>{/if}
        <p class="meta">
          Stichtag {datum(daten.stichtag)}. Eingereichte Antworten werden als
          ausstehende Rückmeldung geführt.
        </p>
        <h3>Direkt zum Bereich</h3>
        <div class="schnellzugriff">
          {#each [["uebersicht", "Stammdaten"], ["rechtsgrundlagen", "Rechtsgrundlagen"], ["unterlagen", "Unterlagenregister"], ["verfahren", "Verfahrensakte"], ["nachweise", "Nachweisstände"], ["druck", "Drucken / PDF-Ausgabe"]] as [id, t]}<button
              type="button"
              class="link"
              onclick={() => onoeffnen(ziel(id))}>{t} →</button
            >{/each}
        </div>
      </aside>
    </div>
    <section aria-label="Verfahrensstand">
      <h3>Verfahren und Entscheidungen</h3>
      <div class="verfahrensliste">
        {#each daten.verfahren as v (v.id)}<article>
            <div>
              <h4>{v.bezeichnung}</h4>
              <p>{v.behoerde}{v.aktenzeichen ? ` · ${v.aktenzeichen}` : ""}</p>
              <p class="status">{v.status}</p>
              {#if v.entscheidung}<p>
                  Letzter aktueller Bescheid: {entscheidungen[v.entscheidung]} ·
                  {datum(v.entscheidungsdatum)}
                </p>{/if}
              <p class="meta">
                {v.offene_punkte} Nachforderungspunkte · {v.offene_auflagen} Auflagen
                offen · {v.versendet} von {v.abgaben} Abgabeständen versendet / eingegangen
                · {v.bescheide} aktuelle Bescheide
              </p>
            </div>
            <button
              type="button"
              class="sekundaer"
              onclick={() => onoeffnen({ ...ziel("verfahren"), objekt: v.id })}
              >Akte öffnen</button
            >
          </article>{:else}<p class="meta">
            Noch keine Verfahrensakte erfasst.
          </p>{/each}
      </div>
    </section>
    <section aria-label="Einreichungsstand">
      <h3>Abgaben und Einreichungen</h3>
      <p class="meta">
        Export, Versand und bestätigter Eingang sind getrennte Zustände.
      </p>
      <div class="tabellen-scroll">
        <table class="ergebnistabelle">
          <thead
            ><tr
              ><th>Unterlagenpaket</th><th>Empfänger / Quelle</th><th
                >Dokumente</th
              ><th>Status</th><th>Datum</th><th></th></tr
            ></thead
          ><tbody
            >{#each daten.abgaben.slice(0, 20) as a (`${a.quelle}:${a.id}`)}<tr
                ><td>{a.titel}</td><td
                  >{a.empfaenger}<br /><span class="meta">{a.quelle}</span></td
                ><td>{a.dokumente}</td><td>{a.status}</td><td
                  >{datum(a.datum)}</td
                ><td
                  ><button
                    type="button"
                    class="link"
                    onclick={() => onoeffnen(a.ziel)}>Öffnen</button
                  ></td
                ></tr
              >{:else}<tr
                ><td colspan="6"
                  >Noch keine Abgabestände oder Einreichungspakete vorhanden.</td
                ></tr
              >{/each}</tbody
          >
        </table>
      </div>
      {#if daten.abgaben.length > 20}<p class="meta">
          Die 20 zuletzt datierten Stände werden angezeigt. Alle Stände in der
          Verfahrensakte beziehungsweise unter Einreichungen öffnen.
        </p>{/if}
    </section>
    <section aria-label="Dokumentierter Behördenstand">
      <h3>Welche Dokumentversionen wurden eingereicht?</h3>
      {#each daten.empfaenger as emp, i (`${emp.empfaenger}:${i}`)}<h4>
          {emp.empfaenger}
        </h4>
        <div class="tabellen-scroll">
          <table class="ergebnistabelle">
            <thead
              ><tr
                ><th>Datei</th><th>Eingereichte Version</th><th
                  >Projektversion</th
                ><th>Eingereicht am</th><th>Vergleich</th></tr
              ></thead
            ><tbody
              >{#each emp.dokumente as d (d.dokument_id)}<tr
                  ><td><button type="button" class="link" onclick={()=>onoeffnen({...ziel("dokumente"),objekt:d.dokument_id,register:"historie",eintrag:String(d.eingereichte_version)})}>{d.dateiname}</button></td><td>{d.eingereichte_version}</td><td
                    >{d.aktuelle_version ?? "—"}</td
                  ><td>{datum(d.eingereicht_am)}</td><td>{d.status}</td></tr
                >{/each}</tbody
            >
          </table>
        </div>
        <button type="button" class="link" onclick={() => onoeffnen(emp.ziel)}
          >Einreichungshistorie öffnen →</button
        >{:else}<p class="meta">
          Noch keine bestätigten Einreichungen oder dokumentierten Versandstände
          vorhanden.
        </p>{/each}
    </section>
    <Statushilfe/>
    <footer>
      <p class="meta">
        Stand {new Date(daten.ermittelt_am).toLocaleString("de-DE")} · Projektversion
        {daten.projekt_version}{laeuft ? " · Aktualisierung läuft" : ""}
      </p>
      {#each daten.hinweise as h}<p class="meta">{h}</p>{/each}
    </footer>
  {:else if laeuft}<p role="status">Projektübersicht wird geladen …</p>{/if}
</section>

<style>
  .projektuebersicht {
    display: grid;
    gap: 14px;
  }
  .kopf {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .kopf {
    border-bottom: 2px solid var(--accent);
    padding-bottom: 4px;
    align-items: end;
  }
  .kopf h2 {
    margin: 0;
  }
  .kopf p {
    margin: 2px 0 0;
  }
  /* Kennzahlen als Prüfliste in einer Zeile statt als Kacheln. */
  .kennzahlen {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 170px), 1fr));
    gap: 0;
    border: 1px solid var(--border);
    background: var(--border);
    column-gap: 1px;
    row-gap: 1px;
  }
  .kennzahl {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 8px;
    align-items: baseline;
    text-align: left;
    background: var(--panel);
    color: var(--fg);
    border: 0;
    border-radius: 0;
    font-weight: 400;
    padding: 5px 10px;
    min-height: 0;
  }
  .kennzahl strong {
    font-size: 1.1rem;
    line-height: 1.2;
    font-variant-numeric: tabular-nums;
    grid-row: 1;
    grid-column: 2;
  }
  .kennzahl .meta {
    font-size: 0.88rem;
    grid-column: 1 / -1;
  }
  .warnung {
    color: var(--warning-text);
  }
  .naechster {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1.5rem;
    flex-wrap: wrap;
    border: 1px solid var(--border);
    background: var(--leiste);
    padding: 8px 12px;
  }
  .naechster > div {
    flex: 1;
    min-width: min(100%, 18rem);
  }
  .naechster h3 {
    margin: 2px 0 4px;
  }
  .naechster p {
    margin: 0;
  }
  .arbeitsraster {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(240px, 0.33fr);
    gap: 14px;
  }
  .arbeitsbedarf h3,
  .termine h3,
  section > h3 {
    margin: 0 0 8px;
    padding-bottom: 4px;
    border-bottom: 2px solid var(--accent);
    color: var(--accent);
  }
  .filter {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
    align-items: end;
  }
  .filter label {
    display: grid;
    gap: 0.4rem;
    min-width: 0;
  }
  .filter .suchfeld {
    flex: 1;
    min-width: min(100%, 14rem);
  }
  .filter input {
    width: 100%;
  }
  .punktliste {
    padding: 0;
    margin: 8px 0;
    list-style: none;
    border: 1px solid var(--border);
  }
  .punktliste > li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 2px 12px;
    padding: 5px 10px;
    border-top: 1px solid var(--border-innen);
    align-items: start;
  }
  .punkt-meta {
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
    grid-column: 1/-1;
  }
  .punktliste p {
    margin: 2px 0 0;
    color: var(--muted);
  }
  .punktliste .leer {
    display: block;
  }
  .status {
    font-size: 0.85rem;
    font-weight: 600;
  }
  .termine {
    padding: 8px 12px;
    border: 1px solid var(--border);
    align-self: start;
    background: var(--leiste);
  }
  .termine ul {
    margin: 0;
    list-style: none;
    padding: 0;
  }
  .termine li {
    display: grid;
    gap: 2px;
    padding: 4px 0;
    align-items: start;
  }
  .termine li > span {
    font-size: 0.85rem;
  }
  .termine h3:not(:first-child) {
    margin-top: 12px;
  }
  .schnellzugriff {
    display: grid;
    justify-items: start;
    gap: 0.5rem;
  }
  .verfahrensliste article {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 6px 0;
    border-top: 1px solid var(--border-innen);
    flex-wrap: wrap;
  }
  .verfahrensliste h4 {
    margin: 0 0 0.5rem;
  }
  .verfahrensliste p {
    margin: 0.35rem 0;
  }
  .tabellen-scroll {
    margin-block: 0.75rem;
  }
  table {
    width: 100%;
    min-width: 580px;
  }
  footer {
    border-top: 1px solid var(--border);
    padding-top: 1rem;
  }
  footer p {
    margin: 0.4rem 0;
  }
  @media (max-width: 1100px) {
    .arbeitsraster {
      grid-template-columns: 1fr;
    }
    .termine {
      background: var(--panel);
    }
  }
  @media (max-width: 650px) {
    .punktliste > li {
      grid-template-columns: 1fr;
    }
    .punkt-meta {
      grid-column: auto;
    }
  }
</style>
