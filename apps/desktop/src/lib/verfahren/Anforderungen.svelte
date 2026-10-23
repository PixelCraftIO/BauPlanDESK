<script lang="ts">
  import { projektNavigation } from "../projektNavigation.svelte";
  import type { Anforderung } from "../generated/plattform";
  import type { DocumentVersion, Benutzer } from "../api";
  import Dokumentliste from "./Dokumentliste.svelte";
  import Zustaendigkeit from "./Zustaendigkeit.svelte";
  let {
    eintraege = $bindable(),
    dateien,
    benutzer,
    darfFreigeben,
    verfahrenId,
  }: {
    verfahrenId?: string | null;
    eintraege: Anforderung[];
    dateien: DocumentVersion[];
    benutzer: Benutzer[];
    darfFreigeben: boolean;
  } = $props();
  const navigation = projektNavigation();
  const auftrag = $derived(navigation?.unterlagenauftrag);
  function erstellen(id: string, titel: string, bereich = "formulare") {
    if (!verfahrenId) return;
    navigation?.unterlageErstellen(
      {
        bereich: "verfahren",
        objekt: verfahrenId,
        register: "anforderungen",
        eintrag: id,
      },
      titel,
      bereich,
    );
  }
</script>

<h3>Anforderungen und Vollständigkeit</h3>
<p class="meta">
  Eine Kategorie allein erfüllt keine Anforderung. Erforderlichkeit und genaue
  Unterlagenstände werden ausdrücklich zugeordnet und geprüft.
</p>
<button
  type="button"
  class="sekundaer"
  onclick={() =>
    eintraege.push({
      id: crypto.randomUUID(),
      titel: "",
      erforderlich: "ungeklaert",
      grund: "",
      quelle: "Manuell",
      dokumente: [],
      freigegeben: false,
      verantwortlich: null,
    })}>Anforderung ergänzen</button
>
{#each eintraege as a (a.id)}<details
    data-uebersicht-eintrag={a.id}
    open={!a.freigegeben}
  >
    <summary
      >{a.titel || "Neue Anforderung"} · {a.erforderlich} · {a.freigegeben
        ? "Freigegeben"
        : "Zu prüfen"}</summary
    >
    {#if verfahrenId}<div class="knoepfe links">
        <button
          type="button"
          class="sekundaer"
          onclick={() => erstellen(a.id, a.titel)}
          >Formular für diese Anforderung erstellen …</button
        ><button
          type="button"
          class="link"
          onclick={() => erstellen(a.id, a.titel, "dokumente")}
          >Datei importieren oder auswählen …</button
        >
      </div>{/if}
    {#if auftrag?.ziel.objekt === verfahrenId && auftrag?.ziel.eintrag === a.id && auftrag.dokument}<aside
        class="hinweisbox"
      >
        <p>
          Neue Dokumentversion bereit. Die fachliche Freigabe wird nach der
          Zuordnung zurückgesetzt.
        </p>
        <button
          type="button"
          onclick={() => {
            const d = auftrag.dokument!;
            if (
              !a.dokumente.some(
                (x) =>
                  x.dokument_id === d.dokument_id && x.version === d.version,
              )
            )
              a.dokumente.push({ ...d });
            a.freigegeben = false;
            if (navigation) navigation.unterlagenauftrag = null;
          }}>Version dieser Anforderung zuordnen</button
        ><button
          type="button"
          class="link"
          onclick={() => {
            if (navigation) navigation.unterlagenauftrag = null;
          }}>Vorschlag verwerfen</button
        >
      </aside>{/if}
    <div class="raster">
      <label
        >Anforderung<span class="pflicht" aria-hidden="true"> *</span><input
          aria-label="Anforderung"
          required
          maxlength="500"
          bind:value={a.titel}
        /></label
      ><label
        >Erforderlichkeit<select
          bind:value={a.erforderlich}
          onchange={() => (a.freigegeben = false)}
          ><option value="ungeklaert">Noch zu klären</option><option
            value="erforderlich">Erforderlich</option
          ><option value="nicht_erforderlich">Nicht erforderlich</option
          ></select
        ></label
      ><Zustaendigkeit {benutzer} bind:wert={a.verantwortlich} />
    </div>
    <label
      >Quelle / Katalogstand<input
        maxlength="4000"
        bind:value={a.quelle}
      /></label
    ><label
      >Begründung<textarea rows="3" maxlength="4000" bind:value={a.grund}
      ></textarea></label
    ><Dokumentliste {dateien} bind:werte={a.dokumente} /><label
      ><input
        type="checkbox"
        disabled={!darfFreigeben}
        bind:checked={a.freigegeben}
      />Fachlich freigegeben (Projektleitung)</label
    >
  </details>{/each}
