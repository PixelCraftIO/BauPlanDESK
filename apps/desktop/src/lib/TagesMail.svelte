<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { api, fehlertext, type Benutzer } from "./api";
  import type { Tagesmaileinstellung } from "./generated/plattform";
  let { modus, ich }: { modus: string; ich: { id: string } | null } = $props();
  const netzwerk = $derived(modus === "Netzwerk");
  let e = $state<Tagesmaileinstellung>({
      aktiv: false,
      adresse: "",
      umfang: "meine",
      abwesend_bis: null,
      vertretung: null,
    }),
    benutzer = $state<Benutzer[]>([]),
    fehler = $state<string | null>(null),
    meldung = $state<string | null>(null),
    laeuft = $state(false);
  async function laden() {
    try {
      const [x, b] = await Promise.all([
        invoke<Tagesmaileinstellung>("tagesmail_einstellung"),
        api.benutzerAuflisten(),
      ]);
      e = x;
      benutzer = b.filter((u) => u.aktiv && u.id !== ich?.id);
    } catch (err) {
      fehler = fehlertext(err);
    }
  }
  async function speichern() {
    laeuft = true;
    fehler = null;
    meldung = null;
    try {
      e = await invoke<Tagesmaileinstellung>("tagesmail_setzen", {
        einstellung: $state.snapshot(e),
      });
      meldung = e.aktiv
        ? "Gespeichert. Die Zusammenfassung kommt jeden Morgen, wenn Fristen anstehen."
        : "Gespeichert. Keine tägliche E-Mail.";
    } catch (err) {
      fehler = fehlertext(err);
    } finally {
      laeuft = false;
    }
  }
  $effect(() => {
    if (netzwerk) void laden();
  });
</script>

<section class="einstellungsabschnitt">
  <h2>Tägliche E-Mail</h2>
  {#if !netzwerk}
    <p class="meta">
      Die tägliche Fristen-E-Mail verschickt der BauPlanDESK-Server im
      Netzwerkbetrieb über den SMTP-Zugang des Büros.
    </p>
  {:else}
    <p class="meta">
      Jeden Morgen eine Zusammenfassung der überfälligen Fristen und der
      Termine der nächsten sieben Tage. Ohne fällige Termine kommt keine
      Nachricht. Der Server braucht dafür einen eingerichteten SMTP-Zugang
      (Administration).
    </p>
    <div class="raster">
      <label class="checkbox"
        ><input type="checkbox" bind:checked={e.aktiv} />Tägliche E-Mail erhalten</label
      >
      <label
        >E-Mail-Adresse<input
          type="email"
          bind:value={e.adresse}
          placeholder="name@buero.de"
          required={e.aktiv}
        /></label
      >
      <label
        >Umfang<select bind:value={e.umfang}
          ><option value="meine">Nur meine Zuständigkeit</option><option value="alle"
            >Alle Fristen</option
          ></select
        ></label
      >
      <label
        >Abwesend bis<input type="date" bind:value={e.abwesend_bis} /></label
      >
      <label
        >Vertretung während der Abwesenheit<select bind:value={e.vertretung}
          ><option value={null}>Keine</option
          >{#each benutzer as b (b.id)}<option value={b.id}>{b.anzeigename}</option
            >{/each}</select
        ></label
      >
    </div>
    <p class="meta">
      Während der Abwesenheit erhält die Vertretung Ihre Termine in ihrer
      eigenen Zusammenfassung; Sie erhalten bis dahin keine Nachricht.
    </p>
    <div class="knoepfe links">
      <button onclick={speichern} disabled={laeuft}>Speichern</button>
    </div>
    {#if meldung}<p role="status">{meldung}</p>{/if}
  {/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
</section>
