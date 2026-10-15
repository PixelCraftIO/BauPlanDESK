<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api, fehlertext, type Project, type Benutzer } from "../api";
  import type { Aufgabe } from "../generated/plattform";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  let {
    projekt,
    benutzerId,
    darf,
  }: { projekt: Project; benutzerId: string; darf: boolean } = $props();
  const s = untrack(
    () =>
      new FachSitzung<Aufgabe>(
        projekt.id,
        "aufgaben",
        benutzerId,
        () => ({
          titel: "",
          beschreibung: "",
          faellig: null,
          verantwortlich: null,
          status: "offen",
          bezug: null,
        }),
        (a) =>
          `${a.titel} · ${a.status === "erledigt" ? "Erledigt" : (a.faellig ?? "Ohne Frist")}`,
      ),
  );
  let benutzer = $state<Benutzer[]>([]);
  onMount(async () => {
    try {
      benutzer = await api.benutzerAuflisten();
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
</script>

<Fachliste titel="Aufgaben und Fristen" {s} bereich="aufgaben" {darf}>
  {#snippet children(a)}<div class="raster">
      <label
        >Aufgabe<input bind:value={a.titel} required maxlength="500" /></label
      ><label
        >Status<select bind:value={a.status}
          ><option value="offen">Offen</option><option value="in_arbeit"
            >In Arbeit</option
          ><option value="erledigt">Erledigt</option></select
        ></label
      ><label
        >Frist<input
          type="date"
          value={a.faellig ?? ""}
          onchange={(e) => (a.faellig = e.currentTarget.value || null)}
        /></label
      ><label
        >Verantwortlich<select bind:value={a.verantwortlich}
          ><option value={null}>Noch nicht zugeordnet</option
          >{#each benutzer.filter((b) => b.aktiv) as b (b.id)}<option
              value={b.id}>{b.anzeigename}</option
            >{/each}</select
        ></label
      >
    </div>
    <label
      >Beschreibung<textarea bind:value={a.beschreibung} maxlength="10000"
      ></textarea></label
    >{/snippet}
</Fachliste>
