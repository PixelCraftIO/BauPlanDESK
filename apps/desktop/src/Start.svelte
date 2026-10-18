<script lang="ts">
  // Anmeldetor: Die Anwendung startet erst, wenn ein Benutzer vollständig
  // angemeldet ist. Abmelden lädt die Seite neu und landet wieder hier.
  import { onMount } from "svelte";
  import App from "./App.svelte";
  import Anmeldung from "./lib/Anmeldung.svelte";
  import { initialisieren as darstellungInitialisieren } from "./lib/darstellung";
  import { api, fehlertext, type Anmeldestatus } from "./lib/api";

  let status = $state<Anmeldestatus | null>(null);
  let fehler = $state<string | null>(null);

  onMount(() => {
    darstellungInitialisieren();
    api
      .anmeldestatus()
      .then((s) => (status = s))
      .catch((e) => (fehler = fehlertext(e)));
  });
</script>

{#if status?.benutzer}
  <App />
{:else if status}
  <Anmeldung {status} onstatus={(s) => (status = s)} />
{:else if fehler}
  <p class="fehlertext" role="alert">
    Anmeldestatus nicht lesbar: {fehler}
  </p>
{/if}
