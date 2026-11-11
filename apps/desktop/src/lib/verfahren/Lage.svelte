<script lang="ts">
  import { datumDe } from "../api";
  import type {
    Handlungsoption,
    Verfahrensart,
    Verfahrenslage,
  } from "../generated/plattform";
  import { fehlertext } from "../api";
  import { verfahrenApi, VERFAHRENSART } from "./api";
  let {
    projektId,
    verfahrenId,
    revision,
    darf,
    onregister,
    onschreiben,
    onfolgeverfahren,
  }: {
    /** Register Schreiben mit vorgewählter Art öffnen. */
    onschreiben?: (art: string) => void;
    projektId: string;
    verfahrenId: string | null;
    /** Gespeicherter Stand; neu laden, wenn er sich ändert. */
    revision: number;
    darf: boolean;
    onregister: (register: string) => void;
    onfolgeverfahren: (art: Verfahrensart, bezeichnung: string) => void;
  } = $props();
  let lage = $state<Verfahrenslage | null>(null),
    fehler = $state("");
  $effect(() => {
    const id = verfahrenId;
    void revision;
    if (!id || !revision) return;
    let aktuell = true;
    verfahrenApi.optionen(projektId, id).then(
      (l) => aktuell && (lage = l),
      (e) => aktuell && (fehler = fehlertext(e)),
    );
    return () => (aktuell = false);
  });
  function ausfuehren(o: Handlungsoption) {
    const a = o.aktion;
    if (!a) return;
    if (a.art === "schreiben")
      onschreiben ? onschreiben(a.schreiben) : onregister("schreiben");
    else if (a.art === "register") onregister(a.register);
    else
      onfolgeverfahren(
        a.verfahrensart,
        `${VERFAHRENSART[a.verfahrensart]} nach ${lage?.lage ?? "Ablehnung"}`,
      );
  }
</script>

{#if lage && lage.optionen.length}<section
    class="lage"
    aria-label="Lage und nächste Schritte"
  >
    <h3>Lage: {lage.lage}</h3>
    {#snippet eintrag(o: Handlungsoption)}<li>
        <strong>{o.titel}</strong>{o.frist ? ` · bis ${datumDe(o.frist)}` : ""}{o.rechtsgrundlage
          ? ` · ${o.rechtsgrundlage}`
          : ""}
        <p class="meta">{o.beschreibung}</p>
        {#if o.aktion && (darf || o.aktion.art !== "folgeverfahren")}<button
            type="button"
            class="link"
            onclick={() => ausfuehren(o)}
            >{o.aktion.art === "folgeverfahren"
              ? `${VERFAHRENSART[o.aktion.verfahrensart]} anlegen`
              : "Öffnen"}</button
          >{/if}
      </li>{/snippet}
    <ul>
      {@render eintrag(lage.optionen[0])}
    </ul>
    {#if lage.optionen.length > 1}<details>
        <summary
          >Weitere Möglichkeiten ({lage.optionen.length - 1}): {lage.optionen
            .slice(1)
            .map((o) => o.titel)
            .join(" · ")}</summary
        >
        <ul>
          {#each lage.optionen.slice(1) as o}{@render eintrag(o)}{/each}
        </ul>
      </details>{/if}
  </section>{:else if fehler}<p class="meta">{fehler}</p>{/if}
