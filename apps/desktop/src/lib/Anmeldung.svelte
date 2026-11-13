<script lang="ts">
  import { api, fehlertext, type Anmeldestatus } from "./api";
  import KennwortAendern from "./KennwortAendern.svelte";

  let {
    status,
    onstatus,
  }: {
    status: Anmeldestatus;
    onstatus: (s: Anmeldestatus) => void;
  } = $props();

  let anmeldename = $state("");
  let kennwort = $state("");
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);

  async function ausfuehren(aufruf: () => Promise<Anmeldestatus>) {
    fehler = null;
    laeuft = true;
    try {
      const s = await aufruf();
      kennwort = "";
      onstatus(s);
    } catch (e) {
      kennwort = "";
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }

  function anmelden(ev: SubmitEvent) {
    ev.preventDefault();
    void ausfuehren(() => api.anmelden(anmeldename, kennwort));
  }
</script>

<main class="anmeldeseite">
  <section class="anmeldekarte" aria-labelledby="anmeldung-titel">
    <h1 id="anmeldung-titel">BauPlanDESK</h1>
    {#if status.kennwort_aendern}
      <h2>Eigenes Kennwort festlegen</h2>
      <p class="meta">
        Angemeldet als <strong>{status.kennwort_aendern.anzeigename}</strong>
        ({status.kennwort_aendern.anmeldename}).
        {status.bisheriges_kennwort_noetig
          ? "Bitte ersetzen Sie das Erstkennwort durch ein eigenes Kennwort."
          : "Bitte legen Sie jetzt ein Kennwort fest. Danach melden Sie sich an jedem Arbeitsplatz mit Anmeldename und Kennwort an."}
      </p>
      <KennwortAendern
        bisherNoetig={status.bisheriges_kennwort_noetig}
        mindestlaenge={status.kennwort_mindestlaenge}
        knopf="Kennwort festlegen und weiter"
        ongespeichert={onstatus}
        onabbrechen={() => void ausfuehren(() => api.abmelden())}
      />
    {:else}
      <h2>Anmeldung</h2>
      <form class="anmeldeformular" onsubmit={anmelden}>
        <label
          >Anmeldename
          <!-- svelte-ignore a11y_autofocus -->
          <input
            autocomplete="username"
            bind:value={anmeldename}
            required
            autofocus
          /></label
        >
        <label
          >Kennwort
          <input
            type="password"
            autocomplete="current-password"
            bind:value={kennwort}
            required
          /></label
        >
        {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
        <div class="knoepfe">
          <button type="submit" disabled={laeuft}
            >{laeuft ? "Wird geprüft …" : "Anmelden"}</button
          >
        </div>
      </form>
      {#if status.optionen?.uebergang_moeglich}
        <div class="uebergang" role="note">
          <p>
            {#if status.optionen.erster_administrator}
              Diese Datenbank hat noch keine Benutzer. Sie können sich als
              <strong>{status.optionen.rechnerkonto}</strong> (Benutzerkonto dieses
              Rechners) als erster Administrator anmelden und legen dabei sofort
              ein Kennwort fest.
            {:else}
              Noch kein Administrator hat ein Kennwort. Als Administrator
              <strong>{status.optionen.rechnerkonto}</strong> können Sie sich
              einmalig über das Benutzerkonto dieses Rechners anmelden und legen
              dabei sofort ein Kennwort fest.
            {/if}
          </p>
          <button
            class="sekundaer"
            disabled={laeuft}
            onclick={() => void ausfuehren(() => api.anmeldenUebergang())}
            >Als {status.optionen.rechnerkonto} anmelden und Kennwort festlegen</button
          >
        </div>
      {/if}
      <p class="meta">
        Kein Kennwort oder Kennwort vergessen? Ein Administrator vergibt in der
        Benutzerverwaltung ein Erstkennwort.
      </p>
    {/if}
  </section>
</main>

<style>
  .anmeldeseite {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: var(--space-4) 1rem;
    box-sizing: border-box;
    background: var(--bg);
  }
  .anmeldekarte {
    width: min(26rem, 100%);
    background: var(--panel);
    border: 1px solid var(--border);
    padding: var(--space-4);
    display: grid;
    gap: var(--space-2);
  }
  .anmeldekarte h1 {
    margin: 0;
    font-size: 1.3rem;
  }
  .anmeldekarte h2 {
    margin: 0;
    font-size: 1.05rem;
  }
  .anmeldeformular {
    display: grid;
    gap: var(--space-2);
  }
  .anmeldeformular label {
    display: grid;
    gap: 0.2rem;
  }
  .uebergang {
    border: 1px solid var(--border-innen);
    background: var(--warn-bg);
    color: var(--warning-text);
    padding: var(--space-2);
  }
  .uebergang p {
    margin: 0 0 var(--space-2);
  }
</style>
