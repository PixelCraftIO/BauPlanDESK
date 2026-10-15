<script lang="ts">
  import {
    api,
    fehlertext,
    landName,
    type Project,
    type ZuletztEintrag,
  } from "./api";
  let {
    zuletzt,
    darfAnlegen,
    onneu,
    onoeffnen,
  }: {
    zuletzt: ZuletztEintrag[];
    darfAnlegen: boolean;
    onneu: () => void;
    onoeffnen: (id: string) => void;
  } = $props();
  let projekte = $state<Project[]>([]),
    suche = $state(""),
    fehler = $state<string | null>(null),
    laedt = $state(false);
  let sortierung = $state<"name" | "updated_at">("updated_at");
  const liste = $derived(
    projekte
      .filter((p) =>
        (p.name + " " + landName(p.bundesland))
          .toLowerCase()
          .includes(suche.trim().toLowerCase()),
      )
      .sort((a, b) =>
        sortierung === "name"
          ? a.name.localeCompare(b.name, "de")
          : b.updated_at.localeCompare(a.updated_at),
      ),
  );
  async function laden() {
    laedt = true;
    fehler = null;
    try {
      projekte = await api.projekte();
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laedt = false;
    }
  }
  $effect(() => {
    void zuletzt;
    void laden();
  });
</script>

<div class="verzeichnis-kopf">
  <div>
    <h1>Projektverzeichnis</h1>
    <p class="meta">Projektakten öffnen und parallel bearbeiten.</p>
  </div>
  <button onclick={onneu} disabled={!darfAnlegen}>Neues Projekt</button>
</div>
<div class="verzeichnis-filter">
  <label
    >Projekte suchen<input
      type="search"
      bind:value={suche}
      placeholder="Projektname oder Bundesland"
    /></label
  ><label
    >Sortierung<select bind:value={sortierung}
      ><option value="updated_at">Zuletzt geändert</option><option value="name"
        >Projektname</option
      ></select
    ></label
  ><button class="sekundaer" onclick={laden} disabled={laedt}
    >Aktualisieren</button
  ><span class="meta">{liste.length} Projekte</span>
</div>
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
<div class="tabellen-scroll">
  <table class="projekttabelle">
    <thead
      ><tr
        ><th>Projekt</th><th>Bundesland</th><th>Grundstück</th><th
          >Zuletzt geändert</th
        ><th>Aktion</th></tr
      ></thead
    ><tbody
      >{#each liste as p (p.id)}<tr
          ><td
            ><button class="link" onclick={() => onoeffnen(p.id)}
              >{p.name}</button
            ></td
          ><td>{landName(p.bundesland)}</td><td class="zahl"
            >{#if p.planungsrecht.grundstuecksinformationen || p.planungsrecht.baugrundstuecke}{p
                .planungsrecht.grundstuecksinformationen?.flurstuecke.length ??
                p.planungsrecht.baugrundstuecke?.flurstuecke.length ??
                0} Flurstücke{:else}{p.grundstuecksflaeche_m2?.toLocaleString(
                "de-DE",
              ) ?? "—"}
              {p.grundstuecksflaeche_m2 == null
                ? ""
                : "m² (noch nicht aufgeschlüsselt)"}{/if}</td
          ><td>{new Date(p.updated_at).toLocaleDateString("de-DE")}</td><td
            ><button class="sekundaer" onclick={() => onoeffnen(p.id)}
              >Öffnen</button
            ></td
          ></tr
        >{:else}<tr
          ><td colspan="5"
            >{laedt
              ? "Projekte werden geladen …"
              : "Keine Projekte gefunden."}</td
          ></tr
        >{/each}</tbody
    >
  </table>
</div>
<p class="meta">
  Geöffnete Akten erscheinen oben als Projekttabs. Eingaben bleiben beim Wechsel
  erhalten.
</p>
