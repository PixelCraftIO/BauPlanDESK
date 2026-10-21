<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    api,
    fehlertext,
    type DocumentVersion,
    type ArchivierteEmail,
  } from "./api";
  import type { MailAdresse, Verfahrensakte } from "./generated/plattform";
  import { plattformApi, type Datensatz } from "./plattformApi";
  let {
    dokument,
    onschliessen,
    darf = false,
    onimport = () => {},
  }: {
    dokument: DocumentVersion;
    onschliessen: () => void;
    darf?: boolean;
    onimport?: () => void;
  } = $props();
  let importiert = $state<string[]>([]),
    importBusy = $state(false);
  async function anhangAblegen(index: number) {
    importBusy = true;
    fehler = "";
    try {
      const d = await api.emailAnhangAblegen(
        dokument.document_id,
        version,
        index,
      );
      importiert = [...importiert, `${d.file_name} im Projekt gespeichert.`];
      onimport();
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      importBusy = false;
    }
  }
  // Übernahme als Kommunikation in eine Verfahrensakte.
  let akten = $state<Datensatz<Verfahrensakte>[]>([]),
    akteId = $state(""),
    eingehend = $state(true),
    zugang = $state(""),
    aktenhinweis = $state("");
  async function aktenLaden() {
    try {
      akten = await plattformApi.liste<Verfahrensakte>(
        dokument.project_id,
        "verfahren",
      );
      if (!akteId && akten.length === 1) akteId = akten[0].objekt;
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  async function inAkteUebernehmen() {
    const a = akten.find((x) => x.objekt === akteId);
    if (!a) return;
    importBusy = true;
    fehler = "";
    aktenhinweis = "";
    try {
      await api.emailInVerfahren(
        dokument.project_id,
        dokument.document_id,
        version,
        a.objekt,
        a.revision,
        eingehend,
        zugang || null,
      );
      aktenhinweis = `Als Kommunikation in „${a.daten.bezeichnung}“ erfasst. Fristen und Nachforderungen dort anhand des Originals bearbeiten.`;
      await aktenLaden();
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      importBusy = false;
    }
  }
  let mail = $state<ArchivierteEmail | null>(null),
    fehler = $state(""),
    laeuft = $state(false),
    version = $state(0),
    versionen = $state<DocumentVersion[]>([]);
  let generation = 0,
    beendet = false;
  const adressen = (liste: MailAdresse[]) =>
    liste
      .map((a) =>
        a.name && a.adresse ? `${a.name} <${a.adresse}>` : a.name || a.adresse,
      )
      .filter(Boolean)
      .join("; ") || "Nicht angegeben";
  async function laden(stand: number) {
    const id = ++generation;
    version = stand;
    laeuft = true;
    fehler = "";
    mail = null;
    try {
      const daten = await api.emailLaden(dokument.document_id, stand);
      if (!beendet && id === generation) mail = daten;
    } catch (e) {
      if (!beendet && id === generation) fehler = fehlertext(e);
    } finally {
      if (!beendet && id === generation) laeuft = false;
    }
  }
  onMount(() => {
    void laden(dokument.version);
    void aktenLaden();
    void api
      .historie(dokument.document_id)
      .then((v) => {
        if (!beendet) versionen = v;
      })
      .catch(() => {});
  });
  onDestroy(() => {
    beendet = true;
    generation++;
  });
  async function externOeffnen() {
    try {
      await api.dokumentExternOeffnen(dokument.document_id, version);
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
</script>

<div class="knoepfe links">
  <label
    >Dateiversion<select
      aria-label="E-Mail-Version"
      value={version}
      disabled={importBusy}
      onchange={(e) => laden(Number(e.currentTarget.value))}
    >
      {#if versionen.length}{#each versionen as v}<option value={v.version}
            >Version {v.version} · {v.file_name}</option
          >{/each}{:else}<option value={version}
          >Version {version} · {dokument.file_name}</option
        >{/if}
    </select></label
  >
  <button class="sekundaer" disabled={!version} onclick={externOeffnen}
    >Im Mailprogramm öffnen</button
  >
  <button class="sekundaer" onclick={onschliessen}>Schließen</button>
</div>
{#if laeuft}<p role="status">E-Mail wird gelesen …</p>{/if}
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>
  <button class="sekundaer" onclick={() => laden(version)}>Erneut lesen</button
  >{/if}
{#if mail}
  <h2>{mail.inhalt.betreff || "Ohne Betreff"}</h2>
  <dl>
    <dt>Von</dt>
    <dd>{adressen(mail.inhalt.absender)}</dd>
    <dt>An</dt>
    <dd>{adressen(mail.inhalt.empfaenger)}</dd>
    {#if mail.inhalt.cc.length}<dt>Cc</dt>
      <dd>{adressen(mail.inhalt.cc)}</dd>{/if}
    {#if mail.inhalt.bcc.length}<dt>Bcc</dt>
      <dd>{adressen(mail.inhalt.bcc)}</dd>{/if}
    <dt>Datum</dt>
    <dd>
      {mail.inhalt.datum
        ? new Date(mail.inhalt.datum).toLocaleString("de-DE")
        : "Nicht angegeben"}
    </dd>
    {#if mail.inhalt.nachrichten_id}<dt>Nachrichten-ID</dt>
      <dd>{mail.inhalt.nachrichten_id}</dd>{/if}
    <dt>Original</dt>
    <dd>
      {mail.dokument.file_name} · {mail.inhalt.format} · Version {mail.dokument
        .version}
    </dd>
  </dl>
  {#each mail.inhalt.hinweise as h}<p class="hinweisbox">{h}</p>{/each}
  <section aria-label="Nachrichtentext">
    <h3>Nachricht</h3>
    <pre>{mail.inhalt.text || "Kein lesbarer Nachrichtentext vorhanden."}</pre>
  </section>
  <h3>Anhänge ({mail.inhalt.anhaenge.length})</h3>
  {#if mail.inhalt.anhaenge.length}<table aria-label="E-Mail-Anhänge">
      <thead
        ><tr><th>Datei</th><th>Typ</th><th>Größe</th><th>Aktion</th></tr></thead
      ><tbody
        >{#each mail.inhalt.anhaenge as a, i}<tr
            ><td
              >{a.name}{a.eingebettete_nachricht
                ? " (eingebettete Nachricht)"
                : ""}</td
            ><td>{a.medientyp ?? "Nicht angegeben"}</td><td
              >{a.groesse_bytes === null
                ? "Nicht angegeben"
                : `${a.groesse_bytes} Bytes`}</td
            ><td
              ><button
                class="sekundaer"
                disabled={!darf ||
                  importBusy ||
                  a.eingebettete_nachricht ||
                  !a.groesse_bytes}
                onclick={() => anhangAblegen(i)}
                >Als Projektunterlage übernehmen</button
              ></td
            ></tr
          >{/each}</tbody
      >
    </table>
    <p class="meta">
      Übernommene Anhänge erhalten eine eigene Dokumentidentität mit Bezug zur
      Nachrichtenversion. Erneutes Übernehmen erzeugt kein Duplikat.
    </p>{:else}<p class="meta">Keine Anhänge erfasst.</p>{/if}
{/if}
{#if mail && darf}
  <section aria-label="In Verfahrensakte übernehmen">
    <h3>In Verfahrensakte übernehmen</h3>
    {#if akten.length}
      <div class="knoepfe links">
        <label
          >Verfahrensakte<select aria-label="Verfahrensakte" bind:value={akteId}
            ><option value="">– wählen –</option>{#each akten as a (a.objekt)}<option
                value={a.objekt}>{a.daten.bezeichnung}</option
              >{/each}</select
          ></label
        ><label
          >Richtung<select bind:value={eingehend}
            ><option value={true}>Eingehend</option><option value={false}
              >Ausgehend</option
            ></select
          ></label
        ><label
          >{eingehend ? "Zugang am" : "Versendet am"}<input
            aria-label="Zugangsdatum"
            type="date"
            bind:value={zugang}
          /></label
        ><button
          disabled={importBusy || !akteId}
          onclick={inAkteUebernehmen}>Als Kommunikation übernehmen</button
        >
      </div>
      <p class="meta">
        Betreff, Absender, Empfänger, Text und Original werden übernommen;
        bereits abgelegte Anhänge werden verknüpft. Ohne Datum gilt das
        Sendedatum der E-Mail. Dieselbe Nachrichtenversion steht projektweit nur
        in einer Akte.
      </p>
    {:else}<p class="meta">Im Projekt gibt es noch keine Verfahrensakte.</p>{/if}
    {#if aktenhinweis}<p role="status">{aktenhinweis}</p>{/if}
  </section>
{/if}
{#each importiert as text}<p role="status">{text}</p>{/each}

<style>
  label {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  h2,
  td {
    overflow-wrap: anywhere;
  }
  dl {
    display: grid;
    grid-template-columns: 8rem minmax(0, 1fr);
    gap: 0.5rem;
  }
  dt {
    font-weight: 600;
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font: inherit;
    max-height: 24rem;
    overflow: auto;
    border: 1px solid var(--border);
    padding: 1rem;
  }
  table {
    width: 100%;
  }
</style>
