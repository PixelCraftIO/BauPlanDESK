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
    onfolgeverfahren,
  }: {
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
    if (a.art === "schreiben") onregister("schreiben");
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
    <ul>
      {#each lage.optionen as o}<li>
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
        </li>{/each}
    </ul>
  </section>{:else if fehler}<p class="meta">{fehler}</p>{/if}
