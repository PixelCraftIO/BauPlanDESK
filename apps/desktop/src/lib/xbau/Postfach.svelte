<script lang="ts">
  import { untrack } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    mindestens,
    type Project,
    type Benutzer,
    type DocumentVersion,
  } from "../api";
  import type {
    JsonValue,
    Dokumentbezug,
    Fachstand,
    Nachrichtentyp,
    Nachrichtenstand,
    Pruefbericht,
    Richtung,
    Belegart,
    Nachrichtenbeleg,
  } from "../generated/plattform";
  import { editorAnmelden } from "../editoren";
  import { xbau } from "./api";
  import { plattformApi } from "../plattformApi";
  let { projekt, benutzer }: { projekt: Project; benutzer: Benutzer } =
    $props();
  const darf = $derived(
    benutzer.aktiv && mindestens(benutzer.rolle, "bearbeitung"),
  );
  let nachrichten = $state<Nachrichtenstand[]>([]);
  let dokumente = $state<DocumentVersion[]>([]);
  let verfahren = $state<Fachstand[]>([]);
  let typen = $state<Nachrichtentyp[]>([]);
  let auswahl = $state("");
  let originalKey = $state("");
  let richtung = $state<Richtung>("eingang");
  let bestaetigt = $state(false);
  let bericht = $state<Pruefbericht | null>(null);
  let laeuft = $state(false);
  let geladen = $state(false);
  let fehler = $state("");
  let hinweis = $state("");
  let suche = $state("");
  let verfahrensId = $state("");
  let generation = 0;
  const aktiv = $derived(nachrichten.find((n) => n.id === auswahl));
  const original = $derived(
    dokumente.find((d) => schluessel(d) === originalKey),
  );
  const kandidaten = $derived(
    dokumente.filter((d) => /\.(xml|zip)$/i.test(d.file_name)),
  );
  const belegNamen: Record<Belegart, string> = {
    versand: "Versand",
    transportquittung: "Transportquittung",
    fachliche_annahme: "Fachliche Annahme",
    rueckweisung: "Rückweisung",
  };
  function schluessel(d: DocumentVersion) {
    return `${d.document_id}:${d.version}`;
  }
  function bezug(d: DocumentVersion): Dokumentbezug {
    return { dokument_id: d.document_id, version: d.version };
  }
  function datum(s: string) {
    return new Date(s).toLocaleString("de-DE");
  }
  const leer = () => ({
    id: "",
    nachricht: "",
    art: "versand" as Belegart,
    datum: "",
    kanal: "",
    referenz: "",
    dokument: "",
    notiz: "",
    berichtigt: "",
  });
  const draftKey = untrack(
    () => `bauprojekt.xbau.beleg.${benutzer.id}.${projekt.id}`,
  );
  function entwurfLaden() {
    try {
      const s = JSON.parse(localStorage.getItem(draftKey) ?? "null");
      return s && typeof s === "object" ? { ...leer(), ...s } : leer();
    } catch {
      return leer();
    }
  }
  let entwurf = $state(entwurfLaden());
  const geaendert = $derived(
    JSON.stringify(entwurf) !== JSON.stringify(leer()),
  );
  $effect(() => {
    try {
      if (geaendert) localStorage.setItem(draftKey, JSON.stringify(entwurf));
      else localStorage.removeItem(draftKey);
    } catch {
      /* Eingabe bleibt in der Sitzung erhalten. */
    }
  });
  $effect(() => {
    void originalKey;
    bericht = null;
    bestaetigt = false;
    generation++;
  });
  async function laden() {
    laeuft = true;
    fehler = "";
    try {
      const [n, d, v, t] = await Promise.all([
        xbau.nachrichten(projekt.id),
        api.dokumente(projekt.id),
        plattformApi.liste<JsonValue>(projekt.id, "verfahren"),
        xbau.typen(),
      ]);
      nachrichten = n;
      dokumente = d;
      verfahren = v;
      typen = t;
      geladen = true;
      if (!auswahl && n.length) auswahl = n[0].id;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  $effect(() => {
    void projekt.id;
    untrack(laden);
  });
  async function importieren() {
    const pfad = await open({
      title: "XBau-Datei im Projekt archivieren",
      multiple: false,
      filters: [{ name: "XBau XML / ZIP", extensions: ["xml", "zip"] }],
    });
    if (typeof pfad !== "string") return;
    laeuft = true;
    fehler = "";
    hinweis = "";
    try {
      const d = await api.dokumentImportieren(projekt.id, pfad);
      await laden();
      originalKey = schluessel(d);
      hinweis =
        "Originaldatei archiviert. Jetzt prüfen und die Projektzuordnung bestätigen.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function pruefen() {
    if (!original) return;
    const d = bezug(original),
      token = ++generation;
    laeuft = true;
    fehler = "";
    bericht = null;
    hinweis = "";
    try {
      const b = await xbau.pruefen(projekt.id, d);
      if (token === generation) bericht = b;
    } catch (e) {
      if (token === generation) fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function aufnehmen() {
    if (!original || !bericht || !bestaetigt) return;
    const w = {
      original: bezug(original),
      richtung,
      zuordnung_bestaetigt: true,
    };
    laeuft = true;
    fehler = "";
    try {
      const n = await xbau.aufnehmen(projekt.id, w);
      await laden();
      auswahl = n.id;
      hinweis =
        "Nachricht im Journal erfasst. Dadurch wird kein Versand bestätigt.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function vorbereiten() {
    laeuft = true;
    fehler = "";
    hinweis = "";
    try {
      const n = await xbau.ausgang(projekt.id);
      await laden();
      auswahl = n.id;
      hinweis =
        "Bauantrag als geprüfte Ausgangsdatei im Projekt gespeichert. Für die Übermittlung das vorgeschriebene Behördenportal verwenden.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function exportieren(d: Dokumentbezug) {
    const meta = dokumente.find(
      (v) => v.document_id === d.dokument_id && v.version === d.version,
    );
    const ziel = await save({
      title: "Unverändertes Original exportieren",
      defaultPath: meta?.file_name ?? "XBau.zip",
    });
    if (!ziel) return;
    laeuft = true;
    fehler = "";
    try {
      await api.dokumentExportieren(d.dokument_id, ziel, d.version);
      hinweis = "Originaldatei exportiert. Kein Versand erfolgt.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function nachreichen() {
    if (!aktiv) return;
    laeuft = true;
    fehler = "";
    try {
      const n = await xbau.nachreichung(projekt.id, aktiv.id);
      await laden();
      auswahl = n.id;
      hinweis =
        "Nachreichung 0202 als neue Ausgangsdatei vorbereitet. Sie enthält den gespeicherten Bauvorhabenstand und die im Bauantrag ausgewählten Anlagen.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function anlageAblegen(id: string, dateiname: string) {
    laeuft = true;
    fehler = "";
    try {
      await xbau.anlage(projekt.id, id, dateiname);
      await laden();
      hinweis = "Anlage mit Herkunftsbezug als Projektdatei übernommen.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function uebernehmen() {
    if (!aktiv) return;
    const v = verfahren.find((v) => v.objekt === verfahrensId);
    if (!v) return;
    laeuft = true;
    fehler = "";
    try {
      await xbau.verfahren(projekt.id, aktiv.id, v.objekt, v.revision);
      await laden();
      hinweis =
        "Als Kommunikation in der Verfahrensakte erfasst. Fristen und fachliche Folgen dort anhand des Originals bearbeiten.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function belegSpeichern(): Promise<boolean> {
    const n = nachrichten.find((n) => n.id === entwurf.nachricht),
      d = dokumente.find((d) => schluessel(d) === entwurf.dokument);
    if (
      !n ||
      !d ||
      !entwurf.datum ||
      !entwurf.kanal.trim() ||
      !entwurf.referenz.trim()
    ) {
      fehler = "Nachricht, Datum, Kanal, Referenz und Belegdatei auswählen.";
      return false;
    }
    laeuft = true;
    fehler = "";
    try {
      if (!entwurf.id) entwurf.id = crypto.randomUUID();
      const b: Nachrichtenbeleg = {
        id: entwurf.id,
        art: entwurf.art,
        datum: new Date(entwurf.datum).toISOString(),
        kanal: entwurf.kanal,
        referenz: entwurf.referenz,
        dokument: bezug(d),
        notiz: entwurf.notiz,
        berichtigt: entwurf.berichtigt || null,
      };
      await xbau.beleg(projekt.id, n.id, n.revision, b);
      entwurf = leer();
      await laden();
      hinweis =
        "Beleg unveränderlich erfasst. Die Angaben wurden manuell dokumentiert; es wurde keine Verbindung zur Behörde hergestellt.";
      return true;
    } catch (e) {
      fehler = fehlertext(e);
      return false;
    } finally {
      laeuft = false;
    }
  }
  editorAnmelden("xbau", {
    hatAenderungen: () => geaendert,
    istBeschaeftigt: () => laeuft,
    speichern: belegSpeichern,
    hinweis: "Den XBau-Beleg vervollständigen oder den Entwurf verwerfen.",
  });
</script>

<div class="xbau-postfach" data-xbau>
  <div class="hinweisbox" role="status">
    <strong>Behördenanschluss: Dateiaustausch</strong>
    <p>
      XBau-Hochbau 2.7 / Kernmodul 1.4.0. Ein direkter Übermittlungsdienst ist
      noch nicht eingerichtet. Exportierte Dateien über das von der Behörde
      vorgegebene Portal übermitteln und die Belege hier erfassen.
    </p>
  </div>
  <div class="knoepfe links">
    <button onclick={laden} disabled={laeuft}>Aktualisieren</button>
    <button onclick={importieren} disabled={!darf || laeuft}
      >XML / ZIP importieren …</button
    >
    <button onclick={vorbereiten} disabled={!darf || laeuft}
      >Bauantrag als Ausgang vorbereiten</button
    >
  </div>
  <p class="meta">
    Der Ausgang verwendet den gespeicherten Bauantrag und seine ausgewählten
    Bauvorlagen. Ungespeicherte Antragsänderungen zuerst im Bereich „Bauantrag /
    XBau“ speichern.
  </p>
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if hinweis}<p class="hinweisbox" role="status">{hinweis}</p>{/if}
  {#if laeuft}<p role="status">XBau-Vorgang wird bearbeitet …</p>{/if}
  <fieldset disabled={laeuft}>
    <legend>Nachricht prüfen und ins Journal aufnehmen</legend>
    <div class="felder">
      <label
        >Original im Projekt<select
          aria-label="Original im Projekt"
          bind:value={originalKey}
          ><option value="">XML / ZIP auswählen</option
          >{#each kandidaten as d}<option value={schluessel(d)}
              >{d.file_name} · Version {d.version}</option
            >{/each}</select
        ></label
      >
      <label
        >Richtung<select aria-label="Richtung" bind:value={richtung}
          ><option value="eingang">Eingangsdatei von der Behörde</option><option
            value="ausgang">Ausgangsdatei an die Behörde</option
          ></select
        ></label
      >
    </div>
    <button onclick={pruefen} disabled={!original}
      >Datei und Anlagen prüfen</button
    >
    {#if bericht}
      <h3>{bericht.kopf.titel}</h3>
      <dl>
        <dt>Nachricht</dt>
        <dd>{bericht.kopf.element}</dd>
        <dt>Absender → Empfänger</dt>
        <dd>{bericht.kopf.absender} → {bericht.kopf.empfaenger}</dd>
        <dt>Antragsreferenz</dt>
        <dd>{bericht.kopf.antragsreferenz ?? "Nicht enthalten"}</dd>
        <dt>Aktenzeichen</dt>
        <dd>{bericht.kopf.aktenzeichen ?? "Nicht enthalten"}</dd>
        <dt>Bezieht sich auf</dt>
        <dd>{bericht.kopf.bezugsnachricht ?? "Keine Nachricht angegeben"}</dd>
        <dt>Nachrichten-ID</dt>
        <dd>{bericht.kopf.id}</dd>
        <dt>Prüfung</dt>
        <dd>{bericht.schemapruefer} · {datum(bericht.geprueft_am)}</dd>
      </dl>
      <ul class="meta">
        {#each bericht.hinweise as h}<li>{h}</li>{/each}
      </ul>
      <p>
        {bericht.anlagen.length} Anlagen vollständig vorhanden; vorhandene SHA-256-
        und Größenangaben stimmen überein.
      </p>
      <label class="bestaetigung"
        ><input type="checkbox" bind:checked={bestaetigt} disabled={!darf} /> Ich
        habe Projekt, Richtung und Beteiligte mit dem Original abgeglichen.</label
      >
      <button onclick={aufnehmen} disabled={!darf || !bestaetigt}
        >Im Nachrichtenjournal erfassen</button
      >
    {/if}
  </fieldset>
  <h3>Nachrichtenjournal</h3>
  {#if geladen && nachrichten.length === 0}<p>
      Noch keine XBau-Nachrichten erfasst.
    </p>{/if}
  <div class="tabelle">
    <table class="ergebnistabelle">
      <thead
        ><tr
          ><th>Erfasst</th><th>Richtung</th><th>Nachricht</th><th
            >Aktenzeichen</th
          ><th>Belege</th></tr
        ></thead
      ><tbody
        >{#each nachrichten as n}<tr class:aktiv={auswahl === n.id}
            ><td>{datum(n.nachricht.erfasst_am)}</td><td
              >{n.nachricht.richtung === "eingang" ? "Eingang" : "Ausgang"}</td
            ><td
              ><button class="sekundaer" onclick={() => (auswahl = n.id)}
                >{n.nachricht.pruefung.kopf.titel}</button
              ></td
            ><td>{n.nachricht.pruefung.kopf.aktenzeichen ?? "–"}</td><td
              >{n.nachricht.belege.length}</td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>
  {#if aktiv}
    {@const n = aktiv.nachricht}
    <fieldset disabled={laeuft}>
      <legend>{n.pruefung.kopf.titel}</legend>
      <dl>
        <dt>Absender → Empfänger</dt>
        <dd>{n.pruefung.kopf.absender} → {n.pruefung.kopf.empfaenger}</dd>
        <dt>Nachrichten-ID</dt>
        <dd>{aktiv.id}</dd>
        <dt>Antragsreferenz</dt>
        <dd>{n.pruefung.kopf.antragsreferenz ?? "–"}</dd>
        <dt>Original</dt>
        <dd>
          Dokumentversion {n.original.version} · SHA-256
          <code>{n.pruefung.sha256}</code>
        </dd>
      </dl>
      <div class="knoepfe links">
        <button onclick={() => exportieren(n.original)}
          >Original exportieren …</button
        >{#if ["baugenehmigung.antrag.0200", "prozessnachrichten.formellePruefungBefundliste.1140"].includes(n.pruefung.kopf.element)}<button
            onclick={nachreichen}
            disabled={!darf}>Nachreichung 0202 vorbereiten</button
          >{/if}
      </div>
      <div class="felder">
        <label
          >Verfahrensakte<select
            aria-label="Verfahrensakte"
            bind:value={verfahrensId}
            ><option value="">Akte auswählen</option
            >{#each verfahren as v}<option value={v.objekt}
                >{String(
                  (v.daten as Record<string, unknown>).bezeichnung ?? v.objekt,
                )}</option
              >{/each}</select
          ></label
        ><button onclick={uebernehmen} disabled={!darf || !verfahrensId}
          >Als Kommunikation übernehmen</button
        >
      </div>
      <h4>Anlagen</h4>
      <div class="tabelle">
        <table class="ergebnistabelle">
          <thead
            ><tr
              ><th>Dateiname</th><th>Bytes</th><th>SHA-256</th><th
                >Projektdatei</th
              ></tr
            ></thead
          ><tbody
            >{#each n.pruefung.anlagen as a}<tr
                ><td>{a.dateiname}</td><td>{a.bytes}</td><td
                  ><code>{a.sha256}</code></td
                ><td
                  ><button
                    class="sekundaer"
                    disabled={!darf}
                    onclick={() => anlageAblegen(aktiv.id, a.dateiname)}
                    >Übernehmen</button
                  ></td
                ></tr
              >{/each}</tbody
          >
        </table>
      </div>
      <h4>Übermittlungs- und Annahmebelege</h4>
      <p class="meta">
        Versand, Transportquittung und fachliche Annahme haben unterschiedliche
        Bedeutung. Belege werden manuell erfasst. Berichtigte Belege bleiben in
        der Historie sichtbar.
      </p>
      {#each n.belege as b}<article class="beleg">
          <strong>{belegNamen[b.art]}</strong> · {datum(b.datum)} · {b.kanal} · {b.referenz}{#if n.belege.some((v) => v.berichtigt === b.id)}
            <span> · berichtigt</span>{/if}
          <p>{b.notiz}</p>
          <button class="sekundaer" onclick={() => exportieren(b.dokument)}
            >Beleg exportieren …</button
          >
        </article>{:else}<p>Keine Übermittlung oder Annahme belegt.</p>{/each}
    </fieldset>
  {/if}
  {#if darf}
    <fieldset disabled={laeuft}>
      <legend>Beleg erfassen</legend>
      <div class="felder">
        <label
          >Nachricht<select
            aria-label="Nachricht"
            bind:value={entwurf.nachricht}
            ><option value="">Nachricht auswählen</option
            >{#each nachrichten as n}<option value={n.id}
                >{n.nachricht.pruefung.kopf.titel} · {datum(
                  n.nachricht.erfasst_am,
                )}</option
              >{/each}</select
          ></label
        >
        <label
          >Belegart<select aria-label="Belegart" bind:value={entwurf.art}
            >{#each Object.entries(belegNamen) as [id, name]}<option value={id}
                >{name}</option
              >{/each}</select
          ></label
        >
        <label
          >Datum / Uhrzeit<input
            type="datetime-local"
            bind:value={entwurf.datum}
          /></label
        >
        <label
          >Kanal / Portal<input
            bind:value={entwurf.kanal}
            maxlength="500"
          /></label
        >
        <label
          >Übermittlungsreferenz<input
            bind:value={entwurf.referenz}
            maxlength="500"
          /></label
        >
        <label
          >Belegdatei im Projekt<select
            aria-label="Belegdatei im Projekt"
            bind:value={entwurf.dokument}
            ><option value="">Originalbeleg auswählen</option
            >{#each dokumente as d}<option value={schluessel(d)}
                >{d.file_name} · Version {d.version}</option
              >{/each}</select
          ></label
        >
        <label
          >Berichtigung von<select
            aria-label="Berichtigung von"
            bind:value={entwurf.berichtigt}
            ><option value="">Neuer Beleg</option>{#each nachrichten
              .find((n) => n.id === entwurf.nachricht)
              ?.nachricht.belege.filter((b) => !nachrichten
                    .find((n) => n.id === entwurf.nachricht)
                    ?.nachricht.belege.some((v) => v.berichtigt === b.id)) ?? [] as b}<option
                value={b.id}>{belegNamen[b.art]} · {b.referenz}</option
              >{/each}</select
          ></label
        >
      </div>
      <label
        >Bemerkung<textarea
          rows="3"
          bind:value={entwurf.notiz}
          maxlength="10000"
        ></textarea></label
      >
      <div class="knoepfe links">
        <button onclick={belegSpeichern} disabled={!geaendert}
          >Beleg speichern</button
        ><button
          class="sekundaer"
          onclick={() => (entwurf = leer())}
          disabled={!geaendert}>Entwurf verwerfen</button
        >
      </div>
    </fieldset>
  {/if}
  <details>
    <summary
      >Unterstützte Schemaprüfung: {typen.length} Nachrichtenarten</summary
    >
    <p>
      Die Schemaprüfung unterstützt die aufgeführten Nachrichten. Fachliche
      Eingabemasken und automatische Erstellung sind derzeit für den Bauantrag
      0200 und die Nachreichung 0202 vorhanden. Weitere Nachrichten lassen sich
      als Originaldatei prüfen und archivieren.
    </p>
    <label>Nachrichten suchen<input type="search" bind:value={suche} /></label>
    <ul>
      {#each typen.filter((t) => (t.titel + " " + t.element)
          .toLowerCase()
          .includes(suche.toLowerCase())) as t}<li>
          {t.titel} <span class="meta">{t.element}</span>
        </li>{/each}
    </ul>
  </details>
</div>

<style>
  .xbau-postfach {
    display: grid;
    gap: 1.25rem;
    max-width: 110rem;
  }
  label {
    display: grid;
    gap: 0.5rem;
  }
  select,
  input,
  textarea {
    width: 100%;
    min-width: 0;
  }
  .felder {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(17rem, 1fr));
    gap: 1rem;
    align-items: end;
    margin-block: 1rem;
  }
  fieldset {
    min-width: 0;
    padding: 1.25rem;
    display: grid;
    gap: 1rem;
  }
  .tabelle {
    overflow-x: auto;
    min-width: 0;
  }
  dl {
    display: grid;
    grid-template-columns: minmax(9rem, 13rem) minmax(0, 1fr);
    gap: 0.6rem 1rem;
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  code {
    overflow-wrap: anywhere;
    font-size: 0.8rem;
  }
  .bestaetigung {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .bestaetigung input {
    width: auto;
  }
  .beleg {
    border-left: 3px solid var(--border);
    padding: 0.75rem 1rem;
  }
  .aktiv {
    background: var(--accent-soft);
  }
  p {
    margin: 0.25rem 0;
  }
  button {
    justify-self: start;
  }
  details {
    padding: 1rem;
    border: 1px solid var(--border);
    border-radius: 0.35rem;
  }
  @media (max-width: 700px) {
    dl {
      grid-template-columns: 1fr;
    }
    fieldset {
      padding: 0.8rem;
    }
  }
</style>
