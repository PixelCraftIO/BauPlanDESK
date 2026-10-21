<script lang="ts">
  import type {
    Unterschrift,
    Unterschriftsart,
    Unterschriftsbedarf,
    Unterschriftsrolle,
  } from "../generated/plattform";
  import { fehlertext } from "../api";
  import { UNTERSCHRIFTSROLLE, heute, verfahrenApi } from "./api";
  let {
    eintraege = $bindable(),
    projektId,
    verfahrenId,
  }: {
    eintraege: Unterschrift[];
    projektId: string;
    verfahrenId: string | null;
  } = $props();
  eintraege ??= [];
  let bedarf = $state<Unterschriftsbedarf[]>([]),
    fehler = $state(""),
    name = $state(""),
    art = $state<Unterschriftsart>("qualifiziert"),
    datum = $state(heute());
  async function laden() {
    if (!verfahrenId) return;
    try {
      bedarf = await verfahrenApi.unterschriften(projektId, verfahrenId);
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  $effect(() => {
    if (verfahrenId) void laden();
  });
  const schluessel = (b: Unterschriftsbedarf) =>
    `${b.dokument.dokument_id}:${b.dokument.version}`;
  const vorhanden = (b: Unterschriftsbedarf, r: Unterschriftsrolle) =>
    eintraege.find(
      (u) =>
        u.rolle === r &&
        u.dokument.dokument_id === b.dokument.dokument_id &&
        u.dokument.version === b.dokument.version,
    );
  function erfassen(b: Unterschriftsbedarf, rolle: Unterschriftsrolle) {
    if (!name.trim()) {
      fehler = "Name der unterzeichnenden Person angeben.";
      return;
    }
    fehler = "";
    eintraege = [
      ...eintraege,
      {
        id: crypto.randomUUID(),
        dokument: structuredClone($state.snapshot(b.dokument)),
        rolle,
        name: name.trim(),
        art,
        datum,
      },
    ];
  }
</script>

<h3>Unterschriften der Bauvorlagen</h3>
<p class="meta">
  Erforderliche Unterschriften je Dokumentkategorie nach dem Landesrecht. Eine
  Unterschrift gilt für genau eine Dokumentversion. Der Abgabestand weist auf
  fehlende Unterschriften hin.
</p>
<div class="raster">
  <label>Unterzeichnet von<input maxlength="500" bind:value={name} /></label
  ><label
    >Art<select bind:value={art}
      ><option value="qualifiziert">Qualifizierte elektronische Signatur</option
      ><option value="handschriftlich">Handschriftlich</option><option
        value="portal">Authentifiziert im Portal</option
      ></select
    ></label
  ><label>Datum<input type="date" max={heute()} bind:value={datum} /></label>
</div>
{#if fehler}<p class="fehler" role="alert">{fehler}</p>{/if}
<table class="ergebnistabelle">
  <thead
    ><tr><th>Unterlage</th><th>Kategorie</th><th>Unterschriften</th></tr></thead
  ><tbody
    >{#each bedarf as b (schluessel(b))}<tr
        ><td>{b.name} · V{b.dokument.version}</td><td
          >{b.kategorie}<span class="meta"> · {b.quelle}</span></td
        ><td
          >{#each b.erforderlich as r}{@const u = vorhanden(b, r)}<div>
              {#if u}✓ {UNTERSCHRIFTSROLLE[r]}: {u.name} ({u.datum}){:else}<button
                  type="button"
                  class="link"
                  onclick={() => erfassen(b, r)}
                  >{UNTERSCHRIFTSROLLE[r]} erfassen</button
                >{/if}
            </div>{/each}</td
        ></tr
      >{:else}<tr
        ><td colspan="3"
          >Keine Unterlagen mit unterschriftspflichtiger Kategorie.</td
        ></tr
      >{/each}</tbody
  >
</table>
