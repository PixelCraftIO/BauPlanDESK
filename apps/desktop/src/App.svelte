<script lang="ts">
  import { initialisieren as darstellungInitialisieren } from "./lib/darstellung";
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import {
    api,
    fehlertext,
    istKonflikt,
    mindestens,
    rollenName,
    type AppInfo,
    type Benutzer,
    type Kunde,
    type Lizenzstatus,
    type Project,
    type Projektwerte as Werte,
    type UpdateInfo,
    type ZuletztEintrag,
  } from "./lib/api";
  import Formularbibliothek from "./lib/Formularbibliothek.svelte";
  import { MODULE, voraussetzung } from "./lib/module";
  import NavIcon from "./lib/NavIcon.svelte";
  import Darstellung from "./lib/Darstellung.svelte";
  import Speicherorte from "./lib/Speicherorte.svelte";
  import DruckEinstellungen from "./lib/DruckEinstellungen.svelte";
  import KategorienEinstellungen from "./lib/KategorienEinstellungen.svelte";
  import AblageEinstellungen from "./lib/AblageEinstellungen.svelte";
  import Projektakte from "./lib/Projektakte.svelte";
  import ProjektVerzeichnis from "./lib/ProjektVerzeichnis.svelte";
  import { type Ansicht } from "./lib/projektSitzung";
  import BauherrenDialog from "./lib/Bauherren.svelte";
  import BeteiligteDialog from "./lib/Beteiligte.svelte";
  import Benutzerverwaltung from "./lib/Benutzerverwaltung.svelte";
  import BueroDialog from "./lib/Buero.svelte";
  import Dialog from "./lib/Dialog.svelte";
  import Kunden from "./lib/Kunden.svelte";
  import ProjektFormular from "./lib/ProjektFormular.svelte";
  import Projektmitglieder from "./lib/Projektmitglieder.svelte";
  import ProjektOeffnen from "./lib/ProjektOeffnen.svelte";
  import LizenzDialog from "./lib/Lizenz.svelte";

  type DialogArt =
    | "neu"
    | "oeffnen"
    | "projektdaten"
    | "benutzer"
    | "buero"
    | "lizenz"
    | "bauherren"
    | "beteiligte"
    | "mitglieder"
    | null;

  let info = $state<AppInfo | null>(null);
  type Sitzung = {
    projekt: Project;
    ansicht: Ansicht;
    planDokument: string | null;
    importAusloeser: number;
    editor?: Projektakte;
  };
  onMount(darstellungInitialisieren);
  let sitzungen = $state<Sitzung[]>([]);
  let aktiveId = $state<string | null>(null);
  let globalAnsicht = $state<
    "projekte" | "kunden" | "verwaltung" | "formulare" | null
  >("projekte");
  const aktiv = $derived(sitzungen.find((s) => s.projekt.id === aktiveId));
  const projekt = $derived(aktiv?.projekt ?? null);
  $effect(() => {
    if (aktiv) {
      const modul = MODULE.find((m) => m.id === aktiv.ansicht);
      if (modul && voraussetzung(aktiv.projekt, modul))
        aktiv.ansicht = "rechtsgrundlagen";
    }
  });
  let schliessZiel = $state<string | null>(null);
  let schliessLaeuft = $state(false);
  let beendenAngefragt = $state(false);
  let beendenLaeuft = $state(false);
  let kundenBesucht = $state(false);
  let kundenEditor = $state<Kunden>();
  let mitgliederEditor = $state<Projektmitglieder>();
  let speicherortEditor = $state<Speicherorte>();
  let druckEditor = $state<DruckEinstellungen>();
  let kategorienEditor = $state<KategorienEinstellungen>();
  let ablageEditor = $state<AblageEinstellungen>();
  let bibliothekEditor = $state<Formularbibliothek>();
  let formulareBesucht = $state(false);
  $effect(() => {
    if (globalAnsicht === "formulare") formulareBesucht = true;
  });
  let verwaltungBesucht = $state(false);
  $effect(() => {
    if (globalAnsicht === "verwaltung") verwaltungBesucht = true;
  });
  $effect(() => {
    if (globalAnsicht === "kunden") kundenBesucht = true;
  });
  let speicherLaeuft = $state(false);
  const ansicht = $derived(aktiv?.ansicht ?? "projektuebersicht");
  function bereich(id: Ansicht) {
    if (aktiv) {
      const modul = MODULE.find((m) => m.id === id);
      if (modul && voraussetzung(aktiv.projekt, modul)) id = "rechtsgrundlagen";
      aktiv.ansicht = id;
      globalAnsicht = null;
    }
  }
  function projektSetzen(p: Project) {
    const s = sitzungen.find((s) => s.projekt.id === p.id);
    if (s) s.projekt = p;
  }
  // Serialize native title/menu updates; never replace a retained draft on activation.
  let nativeQueue = Promise.resolve();
  $effect(() => {
    const id = aktiveId;
    nativeQueue = nativeQueue.then(async () => {
      if (aktiveId !== id) return;
      try {
        if (id) {
          await api.projektAktivieren(id);
          zuletzt = await api.zuletzt();
        } else await api.projektSchliessen();
      } catch (e) {
        if (aktiveId === id) melde(e);
      }
    });
  });
  let dialog = $state<DialogArt>(null);
  let zuletzt = $state<ZuletztEintrag[]>([]);
  let fehler = $state<string | null>(null);
  let hinweis = $state<string | null>(null);
  let update = $state<UpdateInfo | null>(null);
  let installiere = $state(false);
  let updatePrueft = $state(false);
  let updateMeldung = $state("");
  let updateStatus = $state<{
    eingerichtet: boolean;
    quelle: string;
    hinweis: string;
  } | null>(null);
  let updateFortschritt = $state<{
    geladen: number;
    gesamt: number | null;
    phase: string;
  } | null>(null);
  function updateBlockiert() {
    return !!(
      dialog ||
      beendenAngefragt ||
      schliessZiel ||
      speicherLaeuft ||
      schliessLaeuft ||
      beendenLaeuft ||
      sitzungen.some(
        (s) => s.editor?.hatAenderungen() || s.editor?.istBeschaeftigt(),
      ) ||
      kundenEditor?.hatAenderungen() ||
      kundenEditor?.istBeschaeftigt() ||
      ablageEditor?.istBeschaeftigt() ||
      druckEditor?.istBeschaeftigt() ||
      kategorienEditor?.istBeschaeftigt() ||
      speicherortEditor?.istBeschaeftigt() ||
      bibliothekEditor?.istBeschaeftigt()
    );
  }

  let lizenz = $state<Lizenzstatus | null>(null);
  /** Kunden zur Auswahl im Dialog „Neues Projekt“. */
  let kundenAuswahl = $state<Kunde[] | null>(null);
  let kundenLadefehler = $state<string | null>(null);

  /** Angemeldeter Benutzer; ohne Schreibrecht der Lizenz wie „Lesen“. */
  const ich = $derived.by((): Benutzer | null => {
    const b = info?.benutzer ?? null;
    return b && lizenz && !lizenz.schreiben_erlaubt
      ? { ...b, rolle: "lesen" }
      : b;
  });
  const darf = (min: "bearbeitung" | "projektleitung") =>
    !!ich && ich.aktiv && mindestens(ich.rolle, min);

  const melde = (e: unknown) => (fehler = fehlertext(e));

  async function oeffnen(id: string) {
    dialog = null;
    if (sitzungen.some((s) => s.projekt.id === id)) {
      aktiveId = id;
      globalAnsicht = null;
      return;
    }
    try {
      const p = await api.projektOeffnen(id);
      if (!sitzungen.some((s) => s.projekt.id === id))
        sitzungen.push({
          projekt: p,
          ansicht: "projektuebersicht",
          planDokument: null,
          importAusloeser: 0,
        });
      aktiveId = id;
      globalAnsicht = null;
      zuletzt = await api.zuletzt();
    } catch (e) {
      melde(e);
    }
  }
  function schliessen(id = aktiveId) {
    const s = sitzungen.find((s) => s.projekt.id === id);
    if (!s) return;
    if (s.editor?.istBeschaeftigt()) {
      hinweis =
        "In dieser Projektakte läuft eine Aktion oder ein Dokumentdialog ist geöffnet. Bitte zuerst abschließen.";
      return;
    }
    if (s.editor?.hatAenderungen()) {
      schliessZiel = s.projekt.id;
      return;
    }
    entfernen(s.projekt.id);
  }
  function entfernen(id: string) {
    const index = sitzungen.findIndex((s) => s.projekt.id === id);
    sitzungen = sitzungen.filter((s) => s.projekt.id !== id);
    if (aktiveId === id) {
      aktiveId =
        sitzungen[index]?.projekt.id ??
        sitzungen[index - 1]?.projekt.id ??
        null;
      if (!aktiveId) globalAnsicht = "projekte";
    }
    schliessZiel = null;
  }
  async function speichernUndSchliessen() {
    const id = schliessZiel;
    const s = sitzungen.find((s) => s.projekt.id === id);
    if (!s || !id) return;
    schliessLaeuft = true;
    try {
      if (await s.editor?.speichern()) entfernen(id);
      else {
        aktiveId = id;
        globalAnsicht = null;
        schliessZiel = null;
        hinweis =
          "Speichern konnte nicht abgeschlossen werden. Die Projektakte bleibt mit ihren Eingaben geöffnet.";
      }
    } finally {
      schliessLaeuft = false;
    }
  }
  async function aktiveAkteSpeichern() {
    if (!aktiv || speicherLaeuft) return;
    speicherLaeuft = true;
    try {
      if (!(await aktiv.editor?.speichern()))
        hinweis =
          "Bitte prüfen Sie die Angaben in der Projektakte. Die Eingaben bleiben erhalten.";
    } finally {
      speicherLaeuft = false;
    }
  }
  async function speichernUndBeenden() {
    beendenLaeuft = true;
    try {
      for (const s of sitzungen) {
        if (!(await s.editor?.speichern())) {
          aktiveId = s.projekt.id;
          globalAnsicht = null;
          beendenAngefragt = false;
          hinweis =
            "Speichern konnte nicht abgeschlossen werden. Die Anwendung und alle Projektakten bleiben geöffnet.";
          return;
        }
      }
      if (
        kundenEditor?.hatAenderungen() &&
        !(await kundenEditor.speichernBeimSchliessen())
      ) {
        globalAnsicht = "kunden";
        beendenAngefragt = false;
        hinweis =
          "Kundendaten konnten nicht gespeichert werden. Ihre Eingaben bleiben erhalten.";
        return;
      }
      await getCurrentWindow().destroy();
    } catch (e) {
      melde(e);
    } finally {
      beendenLaeuft = false;
    }
  }
  async function neuesProjekt() {
    dialog = "neu";
    kundenAuswahl = null;
    kundenLadefehler = null;
    try {
      kundenAuswahl = await api.kunden();
    } catch (e) {
      kundenLadefehler = fehlertext(e);
    }
  }

  async function anlegen(werte: Werte, bauherr: string | null) {
    try {
      const neu = await api.projektAnlegen(werte);
      // Die Neuanlage ist bereits gespeichert. Auch bei Folgefehlern schließen
      // und öffnen wir genau dieses Projekt, damit kein Duplikat entsteht.
      dialog = null;
      let angelegt = neu;
      if (bauherr) {
        try {
          angelegt = await api.bauherrenSetzen(neu.id, [bauherr], neu.version);
        } catch (e) {
          fehler = `Projekt „${neu.name}“ wurde angelegt. Der Bauherr konnte nicht zugeordnet werden: ${fehlertext(e)} Bitte unter Bauherren zuordnen.`;
        }
      }
      sitzungen.push({
        projekt: angelegt,
        ansicht: "projektuebersicht",
        planDokument: null,
        importAusloeser: 0,
      });
      aktiveId = neu.id;
      globalAnsicht = null;
    } catch (e) {
      melde(e);
    }
  }

  async function speichern(werte: Werte) {
    const ziel = projekt;
    if (!ziel) return;
    try {
      projektSetzen(
        await api.projektAktualisieren({
          id: ziel.id,
          expected_version: ziel.version,
          ...werte,
        }),
      );
      dialog = null;
    } catch (e) {
      if (istKonflikt(e)) {
        // Optimistic Locking: jemand anderes hat das Projekt geändert.
        try {
          projektSetzen(await api.projektOeffnen(ziel.id));
          fehler = "Das Projekt wurde inzwischen an einem anderen Arbeitsplatz geändert. Die aktuelle Fassung wurde geladen; bitte Änderungen erneut eintragen.";
        } catch (ladenFehler) {
          fehler = `Versionskonflikt. Die aktuelle Fassung konnte nicht geladen werden: ${fehlertext(ladenFehler)} Ihre Eingaben bleiben erhalten.`;
        }
      } else {
        melde(e);
      }
    }
  }

  async function updatePruefen() {
    if (updatePrueft || installiere) return;
    updatePrueft = true;
    updateMeldung = "Updatequelle wird geprüft …";
    try {
      updateStatus = await api.updateStatus();
      if (!updateStatus.eingerichtet) {
        updateMeldung = updateStatus.hinweis;
        return;
      }
      update = await api.updatePruefen();
      updateMeldung = update
        ? `Version ${update.version} steht bereit.`
        : "BauProjektIO ist auf dem neuesten Stand.";
    } catch (e) {
      updateMeldung = fehlertext(e);
    } finally {
      updatePrueft = false;
      if (globalAnsicht !== "verwaltung" && !update) hinweis = updateMeldung;
    }
  }
  async function updateInstallieren() {
    if (!update || installiere || updatePrueft) return;
    if (updateBlockiert()) {
      hinweis =
        "Vor der Aktualisierung alle Projekt- und Kundeneingaben speichern oder verwerfen und laufende Aktionen sowie Dialoge abschließen. Ihre Eingaben bleiben geöffnet.";
      return;
    }
    installiere = true;
    updateFortschritt = null;
    try {
      await api.updateInstallieren(update.version);
    } catch (e) {
      melde(e);
      updateMeldung =
        "Installation fehlgeschlagen. Erneut versuchen oder die Updatequelle prüfen.";
    } finally {
      installiere = false;
    }
  }

  function menue(id: string) {
    if (installiere) return;
    if (id.startsWith("zuletzt:")) {
      oeffnen(id.slice("zuletzt:".length));
      return;
    }
    switch (id) {
      case "anwendung.speichern":
        void aktiveAkteSpeichern();
        break;
      case "anwendung.einstellungen":
        globalAnsicht = "verwaltung";
        break;
      case "datei.neu":
        neuesProjekt();
        break;
      case "bearbeiten.kunden":
        dialog = null;
        globalAnsicht = "kunden";
        break;
      case "bearbeiten.benutzer":
        dialog = "benutzer";
        break;
      case "bearbeiten.buero":
        dialog = "buero";
        break;
      case "ansicht.bauantrag":
        bereich("bauantrag");
        break;
      case "ansicht.protokoll":
        bereich("protokoll");
        break;
      case "datei.oeffnen":
        dialog = "oeffnen";
        break;
      case "datei.schliessen":
        schliessen();
        break;
      case "datei.importieren":
        bereich("dokumente");
        aktiv && aktiv.importAusloeser++;
        break;
      case "bearbeiten.projektdaten":
        dialog = "projektdaten";
        break;
      case "ansicht.dokumente":
        bereich("dokumente");
        break;
      case "ansicht.plaene":
        bereich("plaene");
        break;
      case "berechnung.mass":
        bereich("mass");
        break;
      case "berechnung.abstand":
        bereich("abstand");
        break;
      case "hilfe.updates":
        updatePruefen();
        break;
      case "hilfe.lizenz":
        dialog = "lizenz";
        break;
      // Nur unter Linux als eigene Menüpunkte (siehe menu.rs).
      case "bearbeiten.rueckgaengig":
        document.execCommand("undo");
        break;
      case "bearbeiten.wiederholen":
        document.execCommand("redo");
        break;
    }
  }

  onMount(() => {
    (async () => {
      try {
        [info, zuletzt, lizenz] = await Promise.all([
          api.appInfo(),
          api.zuletzt(),
          api.lizenz(),
        ]);
      } catch (e) {
        melde(e);
      }
    })();
    const ab = [
      getCurrentWindow().onCloseRequested((event) => {
        if (installiere) {
          event.preventDefault();
          return;
        }
        if (dialog) {
          event.preventDefault();
          hinweis = "Bitte den geöffneten Dialog zuerst abschließen.";
          return;
        }
        if (
          sitzungen.some((s) => s.editor?.istBeschaeftigt()) ||
          kundenEditor?.istBeschaeftigt() ||
          ablageEditor?.istBeschaeftigt() ||
          druckEditor?.istBeschaeftigt() ||
          kategorienEditor?.istBeschaeftigt() ||
          speicherortEditor?.istBeschaeftigt() ||
          bibliothekEditor?.istBeschaeftigt() ||
          speicherLaeuft ||
          schliessLaeuft
        ) {
          event.preventDefault();
          hinweis = "Bitte laufende Aktionen zuerst abschließen.";
          return;
        }
        if (
          sitzungen.some((s) => s.editor?.hatAenderungen()) ||
          kundenEditor?.hatAenderungen()
        ) {
          event.preventDefault();
          beendenAngefragt = true;
        }
      }),
      listen<string>("menu", (e) => menue(e.payload)),
      listen<UpdateInfo>("update-verfuegbar", (e) => {
        if (!installiere) update = e.payload;
      }),
      listen<{ geladen: number; gesamt: number | null; phase: string }>(
        "update-fortschritt",
        (e) => (updateFortschritt = e.payload),
      ),
      // Regelmäßige Meldung des Arbeitsplatzes (siehe main.rs).
      listen<Lizenzstatus>("lizenz", (e) => (lizenz = e.payload)),
    ];
    void Promise.all(ab)
      .then(async () => {
        const status = await api.updateStatus();
        updateStatus = status;
        if (!installiere && !updatePrueft && !update && status.verfuegbar)
          update = status.verfuegbar;
      })
      .catch(() => {});
    return () => ab.forEach((p) => p.then((f) => f()));
  });
</script>

{#if update}
  <div class="banner">
    <div>
      <strong>Version {update.version} ist verfügbar.</strong>
      {#if update.notizen}<details>
          <summary>Änderungen anzeigen</summary>
          <p style="white-space: pre-wrap">{update.notizen}</p>
        </details>{/if}
      {#if installiere}<p role="status">
          {updateFortschritt?.phase === "installation"
            ? "Update wird installiert …"
            : "Signiertes Update wird heruntergeladen …"}
        </p>
        {#if updateFortschritt?.phase === "download"}<progress
            aria-label="Update herunterladen"
            value={updateFortschritt.gesamt
              ? updateFortschritt.geladen
              : undefined}
            max={updateFortschritt.gesamt ?? undefined}
          ></progress>
          <span
            >{(updateFortschritt.geladen / 1048576).toFixed(1)} MB{updateFortschritt.gesamt
              ? ` von ${(updateFortschritt.gesamt / 1048576).toFixed(1)} MB`
              : ""}</span
          >{/if}
      {:else}<p class="meta">
          Die Anwendung wird neu gestartet. Vorher alle Eingaben speichern und
          Dialoge schließen.
        </p>{/if}
    </div>
    <button onclick={updateInstallieren} disabled={installiere || updatePrueft}>
      {installiere
        ? "Wird installiert …"
        : "Update installieren und neu starten"}
    </button>
    <button
      class="sekundaer"
      onclick={() => (update = null)}
      disabled={installiere}>Später</button
    >
  </div>
{/if}

{#if lizenz && !lizenz.schreiben_erlaubt}
  <div class="fehler" role="alert">
    {lizenz.meldung}
    <button class="sekundaer" onclick={() => (dialog = "lizenz")}
      >Lizenz …</button
    >
  </div>
{/if}

{#if fehler}
  <div class="fehler" role="alert">
    {fehler}
    <button class="sekundaer" onclick={() => (fehler = null)}>Schließen</button>
  </div>
{/if}

{#if hinweis}
  <div class="banner" role="status">
    {hinweis}
    <button class="sekundaer" onclick={() => (hinweis = null)}>OK</button>
  </div>
{/if}

<header class="app-werkzeuge">
  <nav aria-label="Arbeitsplatz">
    <button
      class:aktiv={globalAnsicht === "projekte"}
      aria-pressed={globalAnsicht === "projekte"}
      onclick={() => (globalAnsicht = "projekte")}
      ><NavIcon art="projekte" />Projekte</button
    >
    <button
      class:aktiv={globalAnsicht === "kunden"}
      aria-pressed={globalAnsicht === "kunden"}
      onclick={() => (globalAnsicht = "kunden")}
      ><NavIcon art="kunden" />Kunden / Bauherren</button
    >
    <button
      class:aktiv={globalAnsicht === "formulare"}
      aria-pressed={globalAnsicht === "formulare"}
      onclick={() => (globalAnsicht = "formulare")}
      ><NavIcon art="formulare" />Formularbibliothek</button
    >
    <span class="werkzeug-trenner" aria-hidden="true"></span>
    <button
      class:aktiv={globalAnsicht === "verwaltung"}
      aria-pressed={globalAnsicht === "verwaltung"}
      onclick={() => (globalAnsicht = "verwaltung")}
      ><NavIcon art="einstellungen" />Einstellungen</button
    >
  </nav>
  <span class="konto">{ich?.anzeigename ?? "Wird geladen …"}</span>
</header>
<div class="arbeitsrahmen" inert={installiere}>
  <div class="aktenbereich">
    <nav class="projekttabs" aria-label="Geöffnete Projekte">
      <div class="offene-projekte">
        {#each sitzungen as s (s.projekt.id)}
          <div
            class="projekttab"
            class:aktiv={!globalAnsicht && aktiveId === s.projekt.id}
          >
            <button
              class="tab-auswahl"
              title={s.projekt.name}
              aria-current={!globalAnsicht && aktiveId === s.projekt.id
                ? "page"
                : undefined}
              onclick={() => {
                aktiveId = s.projekt.id;
                globalAnsicht = null;
              }}
              >{s.projekt.name}{#if s.editor?.hatAenderungen()}<span
                  class="entwurf-punkt"
                  aria-label="Ungespeicherte Änderungen">●</span
                >{/if}</button
            >
            <button
              class="tab-schliessen"
              aria-label={`${s.projekt.name} schließen`}
              onclick={() => schliessen(s.projekt.id)}>×</button
            >
          </div>
        {/each}
      </div>
    </nav>
    <div class="globale-seite" hidden={globalAnsicht !== "projekte"}>
      <ProjektVerzeichnis
        {zuletzt}
        darfAnlegen={darf("projektleitung")}
        onneu={neuesProjekt}
        onoeffnen={oeffnen}
      />
    </div>
    {#if kundenBesucht && ich}<div
        class="globale-seite"
        hidden={globalAnsicht !== "kunden"}
      >
        <h1>Kunden / Bauherren</h1>
        <Kunden
          bind:this={kundenEditor}
          {ich}
          onoeffnen={oeffnen}
          onschliessen={() => (globalAnsicht = "projekte")}
        />
      </div>{/if}
    {#if formulareBesucht && ich}<div
        class="globale-seite"
        hidden={globalAnsicht !== "formulare"}
      >
        <Formularbibliothek bind:this={bibliothekEditor} {ich} />
      </div>{/if}
    {#if verwaltungBesucht}<div
        class="globale-seite"
        hidden={globalAnsicht !== "verwaltung"}
      >
        <h1>Einstellungen</h1>
        <Darstellung />
        <Speicherorte {ich} bind:this={speicherortEditor} />
        <AblageEinstellungen bind:this={ablageEditor} />
        <KategorienEinstellungen {ich} bind:this={kategorienEditor} />
        <DruckEinstellungen {ich} bind:this={druckEditor} />
        <div class="verwaltungszeile">
          <div>
            <h2>Büroangaben</h2>
            <p class="meta">Absenderdaten und Behördenkennung</p>
          </div>
          <button class="sekundaer" onclick={() => (dialog = "buero")}
            >Bearbeiten</button
          >
        </div>
        <div class="verwaltungszeile">
          <div>
            <h2>Benutzer und Rechte</h2>
            <p class="meta">Arbeitsplätze und Zugriffsberechtigungen</p>
          </div>
          <button class="sekundaer" onclick={() => (dialog = "benutzer")}
            >Öffnen</button
          >
        </div>
        <div class="verwaltungszeile">
          <div>
            <h2>Lizenz</h2>
            <p class="meta">Lizenzstatus und verfügbare Arbeitsplätze</p>
          </div>
          <button class="sekundaer" onclick={() => (dialog = "lizenz")}
            >Anzeigen</button
          >
        </div>
        <div class="verwaltungszeile">
          <div>
            <h2>Anwendung und Updates</h2>
            <p class="meta">
              {info?.version} · {info?.modus} · {info?.storage}
            </p>
          </div>
          <div>
            {#if updateStatus}<p class="meta">{updateStatus.hinweis}</p>
              {#if updateStatus.quelle}<p
                  class="meta"
                  style="overflow-wrap:anywhere"
                >
                  {updateStatus.quelle}
                </p>{/if}{/if}
            {#if updateMeldung}<p role="status">{updateMeldung}</p>{/if}
            <button
              class="sekundaer"
              onclick={updatePruefen}
              disabled={updatePrueft || installiere}
              >{updatePrueft ? "Wird geprüft …" : "Updates prüfen"}</button
            >
          </div>
        </div>
      </div>{/if}
    {#each sitzungen as s (s.projekt.id)}
      <div
        class="projekt-sitzung"
        data-projekt-id={s.projekt.id}
        hidden={!!globalAnsicht || aktiveId !== s.projekt.id}
      >
        <Projektakte
          bind:this={s.editor}
          projekt={s.projekt}
          bind:ansicht={s.ansicht}
          bind:planDokument={s.planDokument}
          aktiv={!globalAnsicht && aktiveId === s.projekt.id}
          {info}
          {ich}
          {darf}
          importAusloeser={s.importAusloeser}
          onprojekt={projektSetzen}
          ondialog={(art) => {
            aktiveId = s.projekt.id;
            dialog = art;
          }}
        />
      </div>
    {/each}
  </div>
</div>
{#if beendenAngefragt}
  <Dialog
    titel="Anwendung schließen?"
    schliessenErlaubt={!beendenLaeuft}
    onclose={() => {
      if (!beendenLaeuft) beendenAngefragt = false;
    }}
  >
    <p>In Projektakten oder Kundendaten gibt es ungespeicherte Änderungen.</p>
    <div class="knoepfe">
      <button
        class="sekundaer"
        disabled={beendenLaeuft}
        onclick={() => (beendenAngefragt = false)}>Weiter bearbeiten</button
      ><button
        class="sekundaer"
        disabled={beendenLaeuft}
        onclick={() => getCurrentWindow().destroy().catch(melde)}
        >Verwerfen und beenden</button
      ><button
        disabled={beendenLaeuft || !darf("bearbeitung")}
        onclick={speichernUndBeenden}
        >{beendenLaeuft
          ? "Wird gespeichert …"
          : "Alle speichern und beenden"}</button
      >
    </div>
  </Dialog>
{/if}
{#if schliessZiel}
  <Dialog
    schliessenErlaubt={!schliessLaeuft}
    titel="Projektakte schließen?"
    onclose={() => {
      if (!schliessLaeuft) schliessZiel = null;
    }}
  >
    <p>
      <strong
        >{sitzungen.find((s) => s.projekt.id === schliessZiel)?.projekt
          .name}</strong
      > enthält ungespeicherte Änderungen. Andere Projektakten bleiben geöffnet.
    </p>
    <div class="knoepfe">
      <button
        class="sekundaer"
        disabled={schliessLaeuft}
        onclick={() => (schliessZiel = null)}>Weiter bearbeiten</button
      ><button
        class="sekundaer"
        disabled={schliessLaeuft}
        onclick={() => schliessZiel && entfernen(schliessZiel)}
        >Verwerfen und schließen</button
      ><button
        disabled={schliessLaeuft || !darf("bearbeitung")}
        onclick={speichernUndSchliessen}
        >{schliessLaeuft
          ? "Wird gespeichert …"
          : "Speichern und schließen"}</button
      >
    </div>
  </Dialog>
{/if}

<footer class="statusleiste">
  {#if info}
    <span>{projekt?.name ?? "Projektverzeichnis"}</span>
    <button
      class="link"
      onclick={() => (dialog = "benutzer")}
      title="Benutzerverwaltung"
      >{info.benutzer.anzeigename} ({rollenName(ich?.rolle ?? "lesen")})</button
    >
    {#if lizenz?.zustand.art === "testversion"}
      <button class="link" onclick={() => (dialog = "lizenz")}
        >Testversion, noch {lizenz.zustand.tage_uebrig}
        {lizenz.zustand.tage_uebrig === 1 ? "Tag" : "Tage"}</button
      >
    {/if}
    <span>Ablage: {info.storage}</span>
    <span class="rechts">BauProjektIO {info.version}</span>
  {/if}
</footer>

{#if dialog === "lizenz" && info && lizenz}
  <Dialog titel="Lizenz" onclose={() => (dialog = null)}>
    <LizenzDialog
      status={lizenz}
      ich={info.benutzer}
      onstatus={(s) => (lizenz = s)}
      onschliessen={() => (dialog = null)}
    />
  </Dialog>
{:else if dialog === "benutzer" && info}
  <Dialog titel="Benutzerverwaltung" breit onclose={() => (dialog = null)}>
    <Benutzerverwaltung
      ich={info.benutzer}
      onich={(b: Benutzer) => info && (info = { ...info, benutzer: b })}
      onschliessen={() => (dialog = null)}
    />
  </Dialog>
{:else if dialog === "buero" && ich}
  <Dialog titel="Büroangaben" breit onclose={() => (dialog = null)}>
    <BueroDialog {ich} onschliessen={() => (dialog = null)} />
  </Dialog>
{:else if dialog === "mitglieder" && projekt}
  <Dialog
    titel="Projektmitglieder"
    breit
    schliessenErlaubt={!mitgliederEditor?.istBeschaeftigt()}
    onrequestclose={async () => mitgliederEditor?.schliessenErlaubt() ?? true}
    onclose={() => (dialog = null)}
  >
    <Projektmitglieder
      bind:this={mitgliederEditor}
      projektId={projekt.id}
      onschliessen={() => (dialog = null)}
    />
  </Dialog>
{:else if dialog === "bauherren" && ich && projekt}
  <Dialog titel="Bauherren" breit onclose={() => (dialog = null)}>
    <BauherrenDialog
      {projekt}
      {ich}
      onprojekt={projektSetzen}
      onschliessen={() => (dialog = null)}
    />
  </Dialog>
{:else if dialog === "beteiligte" && ich && projekt}
  <Dialog titel="Weitere Beteiligte" breit onclose={() => (dialog = null)}>
    <BeteiligteDialog
      {projekt}
      {ich}
      onprojekt={projektSetzen}
      onschliessen={() => (dialog = null)}
    />
  </Dialog>
{:else if dialog === "neu"}
  <Dialog titel="Neues Projekt" onclose={() => (dialog = null)}>
    <ProjektFormular
      start={{
        name: "",
        bundesland: projekt?.bundesland ?? "BY",
        grundstuecksflaeche_m2: null,
      }}
      kunden={kundenAuswahl}
      kundenLadefehler={kundenLadefehler}
      knopf="Anlegen"
      absenden={anlegen}
      onabbrechen={() => (dialog = null)}
    />
  </Dialog>
{:else if dialog === "oeffnen"}
  <Dialog titel="Projekt öffnen" onclose={() => (dialog = null)}>
    <ProjektOeffnen onoeffnen={oeffnen} onabbrechen={() => (dialog = null)} />
  </Dialog>
{:else if dialog === "projektdaten" && projekt}
  <Dialog titel="Projektdaten" onclose={() => (dialog = null)}>
    <ProjektFormular
      start={{
        name: projekt.name,
        bundesland: projekt.bundesland,
        grundstuecksflaeche_m2: projekt.grundstuecksflaeche_m2,
      }}
      knopf="Speichern"
      absenden={speichern}
      onabbrechen={() => (dialog = null)}
    />
  </Dialog>
{/if}
