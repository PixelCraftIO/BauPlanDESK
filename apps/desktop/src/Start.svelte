<script lang="ts">
  // Anmeldetor: Die Anwendung startet erst, wenn ein Benutzer vollständig
  // angemeldet ist. Abmelden lädt die Seite neu und landet wieder hier.
  import { onMount } from "svelte";
  import App from "./App.svelte";
  import Anmeldung from "./lib/Anmeldung.svelte";
  import EditorFenster from "./lib/EditorFenster.svelte";
  import { initialisieren as darstellungInitialisieren } from "./lib/darstellung";
  import { api, fehlertext, type Anmeldestatus } from "./lib/api";

  let status = $state<Anmeldestatus | null>(null);
  // Editorfenster: dieselbe Oberfläche, aber nur das Aufmaß eines Projekts.
  const abfrage = new URLSearchParams(location.search);
  const editorProjekt = abfrage.get("fenster") === "editor" ? abfrage.get("projekt") : null;
  let fehler = $state<string | null>(null);

  onMount(() => {
    darstellungInitialisieren();
    api
      .anmeldestatus()
      .then((s) => (status = s))
      .catch((e) => (fehler = fehlertext(e)));
  });
</script>

{#if status?.benutzer && editorProjekt}
  <EditorFenster projektId={editorProjekt} benutzer={status.benutzer} />
{:else if status?.benutzer}
  <App />
{:else if status}
  <Anmeldung {status} onstatus={(s) => (status = s)} />
{:else if fehler}
  <p class="fehlertext" role="alert">
    Anmeldestatus nicht lesbar: {fehler}
  </p>
{/if}
