<script lang="ts">
  /**
   * Eigenes Fenster für das Aufmaß eines Projekts (`?fenster=editor&projekt=<id>`).
   * Teilt Anmeldung, Ablage und Lizenz mit dem Hauptfenster; Datenänderungen
   * laufen über die Fensterbrücke in projektEreignisse.ts in beide Richtungen.
   */
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, imProjekt, mindestens, type Benutzer, type Lizenzstatus, type Project } from "./api";
  import type { ProjektRecht } from "./generated/plattform";
  import { projektEditoren } from "./editoren";
  import { projektNavigationAnlegen } from "./projektNavigation.svelte";
  import { bereichAktivSetzen } from "./ansichtsAbfrage.svelte";
  import { abonnieren } from "./projektEreignisse";
  import Aufmasstabs from "./planaufmass/Aufmasstabs.svelte";
  let { projektId, benutzer }: { projektId: string; benutzer: Benutzer } = $props();
  let projekt = $state<Project | null>(null), recht = $state<ProjektRecht>("lesen"), lizenz = $state<Lizenzstatus | null>(null), fehler = $state("");
  const editoren = projektEditoren();
  const navigation = projektNavigationAnlegen();
  bereichAktivSetzen(() => true);
  $effect(() => { navigation.benutzerId = benutzer.id; navigation.projektId = projektId; });
  const ich = $derived(imProjekt(lizenz && !lizenz.schreiben_erlaubt ? { ...benutzer, rolle: "lesen" } : benutzer, recht));
  const darf = $derived(ich.aktiv && mindestens(ich.rolle, "bearbeitung"));
  async function laden() {
    fehler = "";
    try {
      const [p, r] = await Promise.all([api.projektOeffnen(projektId), api.projektRecht(projektId)]);
      projekt = p; recht = r;
    } catch (e) { fehler = fehlertext(e); }
  }
  onMount(() => {
    void laden();
    api.lizenz().then(s => (lizenz = s)).catch(() => {});
    const ab = [
      listen<Lizenzstatus>("lizenz", e => (lizenz = e.payload)),
      // Projektdaten (z. B. Flurstücke aus dem Hauptfenster) nachladen, wenn sich die Akte ändert.
      Promise.resolve(abonnieren(projektId, ["fachobjekte", "dokumente"], () => void api.projektOeffnen(projektId).then(p => (projekt = p)).catch(() => {}))),
      getCurrentWindow().onCloseRequested(async event => {
        if (!editoren.hatAenderungen()) return;
        event.preventDefault();
        if (editoren.istBeschaeftigt()) return;
        const speichern = await ask("Ungespeicherte Änderungen im Aufmaß. Vor dem Schließen speichern?", { title: "Aufmaß", kind: "warning", okLabel: "Speichern und schließen", cancelLabel: "Weiter bearbeiten" });
        if (!speichern) return;
        const blockiert = await editoren.speichern();
        if (blockiert) { fehler = blockiert.hinweis ?? "Speichern nicht möglich. Bitte die Eingaben prüfen."; return; }
        await getCurrentWindow().destroy();
      }),
    ];
    return () => { for (const a of ab) void a.then(f => f()); };
  });
</script>

<div class="editorfenster">
  <header class="app-werkzeuge">
    <span class="app-marke" aria-hidden="true">B</span>
    <strong class="fenstertitel">{projekt?.name ?? "Aufmaß"}</strong>
    <span class="meta">Aufmaß · eigenes Fenster</span>
    <span class="rechts">{benutzer.anzeigename}{darf ? "" : " · nur Lesen"}</span>
  </header>
  {#if fehler}<p class="fehlertext" role="alert">{fehler} <button type="button" class="link" onclick={laden}>Erneut laden</button></p>{/if}
  {#if projekt}
    <main class="projekt-sitzung">
      <Aufmasstabs {projekt} benutzerId={benutzer.id} {darf} onprojekt={p => (projekt = p)} />
    </main>
  {:else if !fehler}<p role="status">Projekt wird geladen …</p>{/if}
</div>

<style>
  .editorfenster{display:flex;flex-direction:column;height:100vh;background:var(--bg)}
  .editorfenster > header{display:flex;align-items:center;gap:12px;padding:0 12px;min-height:36px}
  .fenstertitel{color:var(--titelleiste-fg)}
  .editorfenster > header .meta{color:var(--titelleiste-muted)}
  .editorfenster > header .rechts{margin-left:auto;color:var(--titelleiste-muted)}
  main{flex:1;min-height:0;overflow:auto;padding:0 12px 12px;background:var(--panel)}
</style>
