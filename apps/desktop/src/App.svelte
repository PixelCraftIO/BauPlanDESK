<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    api,
    fehlertext,
    istKonflikt,
    landName,
    type AppInfo,
    type Project,
    type Projektwerte as Werte,
    type UpdateInfo,
    type ZuletztEintrag,
  } from "./lib/api";
  import Abstandsflaeche from "./lib/Abstandsflaeche.svelte";
  import Dialog from "./lib/Dialog.svelte";
  import Dokumente from "./lib/Dokumente.svelte";
  import MassDerNutzung from "./lib/MassDerNutzung.svelte";
  import ProjektFormular from "./lib/ProjektFormular.svelte";
  import Plaene from "./lib/Plaene.svelte";
  import ProjektOeffnen from "./lib/ProjektOeffnen.svelte";
  import Startseite from "./lib/Startseite.svelte";

  type Ansicht = "uebersicht" | "mass" | "abstand" | "dokumente" | "plaene";
  type DialogArt = "neu" | "oeffnen" | "projektdaten" | null;

  let info = $state<AppInfo | null>(null);
  let projekt = $state<Project | null>(null);
  let ansicht = $state<Ansicht>("uebersicht");
  let dialog = $state<DialogArt>(null);
  let zuletzt = $state<ZuletztEintrag[]>([]);
  let fehler = $state<string | null>(null);
  let hinweis = $state<string | null>(null);
  let update = $state<UpdateInfo | null>(null);
  let installiere = $state(false);
  let importAusloeser = $state(0);
  let planDokument = $state<string | null>(null);

  const melde = (e: unknown) => (fehler = fehlertext(e));

  async function oeffnen(id: string) {
    dialog = null;
    try {
      projekt = await api.projektOeffnen(id);
      ansicht = "uebersicht";
      planDokument = null;
    } catch (e) {
      melde(e);
    }
    zuletzt = await api.zuletzt();
  }

  async function schliessen() {
    projekt = null;
    await api.projektSchliessen();
    zuletzt = await api.zuletzt();
  }

  async function anlegen(werte: Werte) {
    try {
      const neu = await api.projektAnlegen(werte);
      await oeffnen(neu.id);
    } catch (e) {
      melde(e);
    }
  }

  async function speichern(werte: Werte) {
    if (!projekt) return;
    try {
      projekt = await api.projektAktualisieren({
        id: projekt.id,
        expected_version: projekt.version,
        ...werte,
      });
      dialog = null;
    } catch (e) {
      if (istKonflikt(e)) {
        // Optimistic Locking: jemand anderes hat das Projekt geändert.
        fehler = "Das Projekt wurde inzwischen an einem anderen Arbeitsplatz geändert. Die aktuelle Fassung wurde geladen; bitte Änderungen erneut eintragen.";
        projekt = await api.projektOeffnen(projekt.id);
      } else {
        melde(e);
      }
    }
  }

  /** Gemessene Fläche aus dem Plan-Viewer als Grundstücksfläche speichern. */
  async function grundstuecksflaecheUebernehmen(m2: number) {
    if (!projekt) return;
    await speichern({ name: projekt.name, bundesland: projekt.bundesland, grundstuecksflaeche_m2: m2 });
    if (projekt.grundstuecksflaeche_m2 === m2)
      hinweis = `Grundstücksfläche ${m2.toLocaleString("de-DE")} m² übernommen. Sie steht jetzt unter „Maß der Nutzung“ bereit.`;
  }

  async function updatePruefen() {
    try {
      update = await api.updatePruefen();
      if (!update) hinweis = "BauProjektIO ist auf dem neuesten Stand.";
    } catch (e) {
      melde(e);
    }
  }

  async function updateInstallieren() {
    installiere = true;
    try {
      await api.updateInstallieren(); // startet die App neu
    } catch (e) {
      melde(e);
      installiere = false;
    }
  }

  function menue(id: string) {
    if (id.startsWith("zuletzt:")) {
      oeffnen(id.slice("zuletzt:".length));
      return;
    }
    switch (id) {
      case "datei.neu": dialog = "neu"; break;
      case "datei.oeffnen": dialog = "oeffnen"; break;
      case "datei.schliessen": schliessen(); break;
      case "datei.importieren": ansicht = "dokumente"; importAusloeser++; break;
      case "bearbeiten.projektdaten": dialog = "projektdaten"; break;
      case "ansicht.dokumente": ansicht = "dokumente"; break;
      case "ansicht.plaene": ansicht = "plaene"; break;
      case "berechnung.mass": ansicht = "mass"; break;
      case "berechnung.abstand": ansicht = "abstand"; break;
      case "hilfe.updates": updatePruefen(); break;
      // Nur unter Linux als eigene Menüpunkte (siehe menu.rs).
      case "bearbeiten.rueckgaengig": document.execCommand("undo"); break;
      case "bearbeiten.wiederholen": document.execCommand("redo"); break;
    }
  }

  onMount(() => {
    (async () => {
      try {
        [info, zuletzt] = await Promise.all([api.appInfo(), api.zuletzt()]);
      } catch (e) {
        melde(e);
      }
    })();
    const ab = [
      listen<string>("menu", (e) => menue(e.payload)),
      listen<UpdateInfo>("update-verfuegbar", (e) => (update = e.payload)),
    ];
    return () => ab.forEach((p) => p.then((f) => f()));
  });

  const reiter: { id: Ansicht; text: string }[] = [
    { id: "uebersicht", text: "Übersicht" },
    { id: "mass", text: "Maß der Nutzung" },
    { id: "abstand", text: "Abstandsflächen" },
    { id: "dokumente", text: "Dokumente" },
    { id: "plaene", text: "Pläne" },
  ];
</script>

{#if update}
  <div class="banner">
    Version {update.version} ist verfügbar.
    <button onclick={updateInstallieren} disabled={installiere}>
      {installiere ? "Wird installiert …" : "Jetzt aktualisieren"}
    </button>
    <button class="sekundaer" onclick={() => (update = null)}>Später</button>
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

{#if projekt}
  <header class="projektkopf">
    <div>
      <h1>{projekt.name}</h1>
      <span class="meta">
        {landName(projekt.bundesland)}
        {projekt.grundstuecksflaeche_m2 ? ` · Grundstück ${projekt.grundstuecksflaeche_m2.toLocaleString("de-DE")} m²` : ""}
      </span>
    </div>
    <nav class="reiter" aria-label="Projektbereiche">
      {#each reiter as r}
        <button class:aktiv={ansicht === r.id} aria-current={ansicht === r.id} onclick={() => (ansicht = r.id)}>{r.text}</button>
      {/each}
    </nav>
  </header>

  <main>
    {#if ansicht === "uebersicht"}
      <section>
        <h2>Projektdaten</h2>
        <dl class="daten">
          <dt>Name</dt><dd>{projekt.name}</dd>
          <dt>Bundesland</dt><dd>{landName(projekt.bundesland)}</dd>
          <dt>Grundstücksfläche</dt><dd>{projekt.grundstuecksflaeche_m2 ? `${projekt.grundstuecksflaeche_m2.toLocaleString("de-DE")} m²` : "–"}</dd>
          <dt>Angelegt</dt><dd>{new Date(projekt.created_at).toLocaleString("de-DE")}</dd>
          <dt>Zuletzt geändert</dt><dd>{new Date(projekt.updated_at).toLocaleString("de-DE")} (Version {projekt.version})</dd>
        </dl>
        <button class="sekundaer" onclick={() => (dialog = "projektdaten")}>Bearbeiten …</button>
      </section>
    {:else if ansicht === "mass"}
      <section>
        <h2>Maß der baulichen Nutzung</h2>
        {#key `${projekt.id}:${projekt.grundstuecksflaeche_m2}`}
          <MassDerNutzung grundstuecksflaeche={projekt.grundstuecksflaeche_m2} onfehler={melde} />
        {/key}
      </section>
    {:else if ansicht === "abstand"}
      <section>
        <h2>Abstandsflächen</h2>
        {#key projekt.id}
          <Abstandsflaeche startLand={projekt.bundesland} regelpakete={info?.regelpakete ?? []} onfehler={melde} />
        {/key}
      </section>
    {:else if ansicht === "plaene"}
      <section>
        <h2>Pläne</h2>
        <Plaene projektId={projekt.id} bind:dokumentId={planDokument} onflaeche={grundstuecksflaecheUebernehmen} onfehler={melde} />
      </section>
    {/if}
    <!-- Dokumente bleiben eingehängt, damit "Datei → Dokument importieren" aus jeder Ansicht funktioniert. -->
    <section hidden={ansicht !== "dokumente"}>
      <h2>Dokumente</h2>
      <Dokumente
        projektId={projekt.id}
        {importAusloeser}
        onplan={(id) => { planDokument = id; ansicht = "plaene"; }}
        onfehler={melde}
      />
    </section>
  </main>
{:else}
  <Startseite
    {zuletzt}
    onneu={() => (dialog = "neu")}
    onoeffnen={() => (dialog = "oeffnen")}
    onzuletzt={oeffnen}
  />
{/if}

<footer class="statusleiste meta">
  {#if info}
    BauProjektIO {info.version} · {info.modus} · Ablage: {info.storage} · LBO-Regelpakete: {info.regelpakete.map((r) => r.land).join(", ") || "keine"}
  {/if}
</footer>

{#if dialog === "neu"}
  <Dialog titel="Neues Projekt" onclose={() => (dialog = null)}>
    <ProjektFormular
      start={{ name: "", bundesland: projekt?.bundesland ?? "BY", grundstuecksflaeche_m2: null }}
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
      start={{ name: projekt.name, bundesland: projekt.bundesland, grundstuecksflaeche_m2: projekt.grundstuecksflaeche_m2 }}
      knopf="Speichern"
      absenden={speichern}
      onabbrechen={() => (dialog = null)}
    />
  </Dialog>
{/if}
