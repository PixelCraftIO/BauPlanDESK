<script lang="ts">
  import { onMount } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, type Project, type DocumentVersion } from "../api";
  import type { Suchtreffer, Erinnerung } from "../generated/plattform";
  import { verfahrenApi, heute } from "./api";
  import { indexieren } from "./textExtraktion";
  import { editorAnmelden } from "../editoren";
  let {
    projekt,
    benutzerId,
    darf,
  }: { projekt: Project; benutzerId: string; darf: boolean } = $props();
  let frage = $state(""),
    treffer = $state<Suchtreffer[]>([]),
    erinnerungen = $state<Erinnerung[]>([]),
    dateien = $state<DocumentVersion[]>([]),
    auswahl = $state(""),
    busy = $state(false),
    fehler = $state(""),
    meldung = $state(""),
    nurEigene = $state(false),
    hinweise = $state<string[]>([]);
  async function aktualisieren() {
    if (busy) return;
    busy = true;
    fehler = "";
    try {
      [erinnerungen, dateien] = await Promise.all([
        verfahrenApi.erinnerungen(projekt.id, heute()),
        api.dokumente(projekt.id),
      ]);
      for (const e of erinnerungen) {
        if (
          "Notification" in window &&
          Notification.permission === "granted" &&
          !sessionStorage.getItem(
            `erinnert.${projekt.id}.${e.objekt}.${e.datum}.${heute()}`,
          )
        ) {
          new Notification("BauProjektIO – Wiedervorlage", {
            body: `${e.titel}: ${e.datum}`,
          });
          sessionStorage.setItem(
            `erinnert.${projekt.id}.${e.objekt}.${e.datum}.${heute()}`,
            "1",
          );
        }
      }
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  onMount(() => {
    void aktualisieren();
    const timer = setInterval(() => void aktualisieren(), 60_000);
    return () => clearInterval(timer);
  });
  editorAnmelden("aktensuche", {
    hatAenderungen: () => false,
    istBeschaeftigt: () => busy,
    speichern: async () => !busy,
  });
  async function suchen() {
    busy = true;
    fehler = "";
    try {
      treffer = await verfahrenApi.suche(projekt.id, frage);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  async function index(ocr: boolean) {
    const d = dateien.find((d) => `${d.document_id}:${d.version}` === auswahl);
    if (!d) return;
    busy = true;
    fehler = "";
    try {
      await indexieren(projekt.id, d, ocr, (s) => (meldung = s));
      meldung =
        "Dokumentinhalt indexiert. OCR-Texte sind Suchhilfen; das Original bleibt maßgebend.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  async function exportieren() {
    busy = true;
    fehler = "";
    try {
      const ziel = await save({
        title: "Vollständige Projektakte exportieren",
        defaultPath: "Projektakte.zip",
        filters: [{ name: "ZIP", extensions: ["zip"] }],
      });
      if (ziel) {
        await verfahrenApi.aktenexport(projekt.id, ziel);
        meldung =
          "Projektakte mit allen Dateiversionen, Fachhistorien und Abgabe-Artefakten exportiert.";
      }
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  async function kalenderExport() {
    busy = true;
    fehler = "";
    try {
      const ziel = await save({
        title: "Verfahrenstermine als Kalender exportieren",
        defaultPath: "Verfahrenstermine.ics",
        filters: [{ name: "Kalender (ICS)", extensions: ["ics"] }],
      });
      if (ziel) {
        await verfahrenApi.termineIcs(projekt.id, ziel);
        meldung =
          "Laufende Fristen und Termine als Kalenderdatei exportiert. Ein erneuter Import aktualisiert die Einträge.";
      }
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  async function benachrichtigungen() {
    if (!("Notification" in window)) {
      meldung =
        "Systembenachrichtigungen sind in dieser Laufzeit nicht verfügbar. Wiedervorlagen werden hier angezeigt.";
      return;
    }
    const p = await Notification.requestPermission();
    meldung =
      p === "granted"
        ? "Benachrichtigungen beim geöffneten Arbeitsbereich aktiviert."
        : "Keine Systembenachrichtigungen erlaubt. Wiedervorlagen bleiben hier sichtbar.";
  }
</script>

<h2>Aktensuche und Wiedervorlagen</h2>
{#if fehler}<p role="alert" class="fehlertext">{fehler}</p>{/if}{#if meldung}<p
    role="status"
  >
    {meldung}
  </p>{/if}
<section>
  <h3>Fällige und bevorstehende Vorgänge</h3>
  <div class="knoepfe links">
    <button class="sekundaer" disabled={busy} onclick={aktualisieren}
      >Wiedervorlagen aktualisieren</button
    ><button class="sekundaer" disabled={busy} onclick={kalenderExport}
      >Termine als Kalender (ICS) exportieren</button
    ><button class="sekundaer" onclick={benachrichtigungen}
      >Systembenachrichtigungen aktivieren</button
    ><label
      ><input type="checkbox" bind:checked={nurEigene} />Nur meine
      Zuständigkeiten</label
    >
  </div>
  <p class="meta">
    Fristen werden manuell fachlich bestätigt; Rechtsbehelfsfrist, Geltungsdauer
    und Fertigstellungsfrist berechnet die Anwendung aus dem Bescheid.
    Erinnerungen erscheinen ab dem eingestellten Vorlauf; Ablauftermine enden
    mit ihrem Datum. Systemhinweise benötigen eine geöffnete Anwendung und
    eine erteilte Berechtigung.
  </p>
  <table class="ergebnistabelle">
    <thead><tr><th>Vorgang</th><th>Fristdatum</th><th>Stand</th></tr></thead
    ><tbody
      >{#each erinnerungen.filter((e) => !nurEigene || e.verantwortlich === benutzerId) as e}<tr
          ><td>{e.titel}</td><td>{e.datum}</td><td
            >{e.ueberfaellig
              ? "Überfällig"
              : e.art === "ablauf" ||
                  e.art === "rechtsbehelfsfrist" ||
                  e.art === "zurueckstellung"
                ? "Läuft ab"
                : "Wiedervorlage"}</td
          ></tr
        >{:else}<tr><td colspan="3">Keine fälligen Vorgänge.</td></tr
        >{/each}</tbody
    >
  </table>
</section>
<section>
  <h3>Verfahren, Kommunikation und Dokumentinhalte suchen</h3>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void suchen();
    }}
  >
    <label
      >Suchbegriff<input
        minlength="2"
        maxlength="200"
        bind:value={frage}
      /></label
    ><button disabled={busy || frage.trim().length < 2}>Akte durchsuchen</button
    >
  </form>
  <p class="meta">
    Bis zu 100 Treffer. Dokumentinhalte müssen zuvor indexiert werden;
    Dateinamen und Verfahrenseinträge sind direkt durchsuchbar.
  </p>
  <table class="ergebnistabelle">
    <thead><tr><th>Bereich</th><th>Treffer</th><th>Auszug</th></tr></thead
    ><tbody
      >{#each treffer as t}<tr
          ><td>{t.bereich}</td><td
            >{t.titel}{#if t.dokument}<button
                class="link"
                onclick={() =>
                  api
                    .dokumentExternOeffnen(
                      t.dokument!.dokument_id,
                      t.dokument!.version,
                    )
                    .catch((e) => (fehler = fehlertext(e)))}
                >Original öffnen</button
              >{/if}</td
          ><td>{t.auszug}</td></tr
        >{/each}</tbody
    >
  </table>
</section>
<fieldset disabled={!darf || busy}>
  <legend>Suchindex und OCR</legend><label
    >Dokumentstand<select bind:value={auswahl}
      ><option value="">Dokument auswählen</option
      >{#each dateien as d (`${d.document_id}:${d.version}`)}<option
          value={`${d.document_id}:${d.version}`}
          >{d.file_name} · Version {d.version}</option
        >{/each}</select
    ></label
  >
  <div class="knoepfe links">
    <button
      type="button"
      class="sekundaer"
      disabled={!auswahl}
      onclick={() => index(false)}>Text aus PDF / E-Mail indexieren</button
    ><button
      type="button"
      class="sekundaer"
      disabled={!auswahl}
      onclick={() => index(true)}>Scan-PDF mit OCR indexieren</button
    >
  </div>
  <p class="meta">
    OCR läuft lokal mit Tesseract (Sprachpakete Deutsch und Englisch). Bei
    fehlender Installation erscheint eine Fehlermeldung. Originaldateien werden
    nicht verändert.
  </p>
</fieldset>
<section>
  <h3>Vollständiger Aktenexport</h3>
  <p>
    Exportiert die gesamte Projektakte einschließlich aller Dokumentversionen,
    Verfahrenshistorien und festgehaltenen Ausgaben. Für die Wiederherstellung
    einer Installation stehen die Profil- und Bürosicherungen zur Verfügung.
  </p>
  <button class="sekundaer" disabled={busy} onclick={exportieren}
    >Projektakte als ZIP exportieren …</button
  >
</section>
