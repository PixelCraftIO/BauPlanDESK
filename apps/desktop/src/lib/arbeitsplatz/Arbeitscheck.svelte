<script lang="ts">
  import type {
    Projektuebersicht,
    UebersichtsZiel,
  } from "../generated/plattform";
  import { pruefgruppen, punkteFuer, ziel } from "./arbeitscheck";
  let {
    daten,
    entwuerfe = false,
    verfahren = "",
    kompakt = false,
    onoeffnen,
  }: {
    daten: Projektuebersicht;
    entwuerfe?: boolean;
    verfahren?: string;
    kompakt?: boolean;
    onoeffnen: (z: UebersichtsZiel) => void;
  } = $props();
</script>

<section class="arbeitscheck" aria-label="Arbeitscheck vor der Abgabe">
  <details class="gesamtcheck" open={!kompakt}>
    <summary>Arbeitscheck vor der Abgabe</summary>
    <p class="meta">
      Offene Punkte aus dem gespeicherten Projektstand. Anforderungen und
      gewählten Abgabestand abschließend prüfen; eine leere Liste ist keine
      Vollständigkeitsbestätigung. Die vollständige Liste mit Ampel, Plänen,
      Formularen und Antragsangaben steht in der
      <button
        type="button"
        class="link"
        onclick={() =>
          onoeffnen({
            bereich: "einreichungspruefung",
            objekt: null,
            register: null,
            eintrag: null,
          })}>Einreichungsprüfung</button
      >.
    </p>
    {#if entwuerfe}<p class="hinweisbox" role="status">
        Ungespeicherte Änderungen sind noch nicht berücksichtigt. Betroffene
        Bereiche vor der Abgabe speichern und erneut prüfen.
      </p>{/if}
    <div class="pruefgruppen">
      {#each pruefgruppen as g}
        {@const punkte = punkteFuer(daten.punkte, g.id, verfahren)}
        <details>
          <summary
            >{g.titel}
            <span
              >{punkte.length
                ? `${punkte.length} ${punkte.length === 1 ? "Hinweis" : "Hinweise"}`
                : "Keine offenen Hinweise erfasst"}</span
            ></summary
          >
          <ul>
            {#each punkte as p (p.id)}<li>
                <strong>{p.titel}</strong>
                <p>{p.hinweis}</p>
                <button
                  type="button"
                  class="link"
                  onclick={() => onoeffnen(p.ziel)}
                  >Prüfpunkt öffnen: {p.titel}</button
                >
              </li>{/each}
          </ul>
          <button
            type="button"
            class="sekundaer"
            onclick={() =>
              onoeffnen(
                ziel(
                  g.bereich,
                  ["verfahren", "portale"].includes(g.bereich)
                    ? verfahren || null
                    : null,
                  g.register,
                ),
              )}>{g.titel} prüfen</button
          >
        </details>
      {/each}
    </div>
    <p class="meta">
      Stand: {new Date(daten.ermittelt_am).toLocaleString("de-DE")}. Aufgaben
      und Fristen bleiben zusätzlich in der Projektübersicht sichtbar.
    </p>
  </details>
</section>

<style>
  .arbeitscheck {
    padding: 1rem;
    border: 1px solid var(--border);
    background: var(--leiste);
  }
  .gesamtcheck {
    border: 0;
    padding: 0;
    background: transparent;
  }
  .gesamtcheck > summary {
    font-size: 1rem;
  }
  .pruefgruppen {
    display: grid;
    gap: 0.5rem;
  }
  details {
    background: var(--panel);
    border: 1px solid var(--border);
    padding: 0.55rem 0.8rem;
  }
  summary {
    font-weight: 600;
  }
  summary span {
    font-weight: 400;
    color: var(--muted);
    margin-left: 0.75rem;
  }
  ul {
    padding: 0;
    margin: 0.5rem 0;
    list-style: none;
  }
  li {
    display: block;
    padding: 0.8rem 0;
    border-bottom: 1px solid var(--border);
  }
  li p {
    margin: 0.4rem 0;
  }
</style>
