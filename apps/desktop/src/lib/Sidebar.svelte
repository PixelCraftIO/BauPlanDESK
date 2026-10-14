<script lang="ts">
  import NavIcon from "./NavIcon.svelte";
  import type { Project } from "./api";
  import type { Ansicht } from "./projektSitzung";
  let {
    projekt,
    ansicht,
    globalAnsicht,
    onbereich,
    onglobal,
  }: {
    projekt: Project | null;
    ansicht: Ansicht;
    globalAnsicht: string | null;
    onbereich: (a: Ansicht) => void;
    onglobal: (a: "projekte" | "kunden" | "formulare" | "verwaltung") => void;
  } = $props();
  const mitPlan = $derived(
    !!projekt &&
      projekt.planungsrecht.rechtsgrundlage === "par30" &&
      !["kein", "ungeklaert"].includes(projekt.planungsrecht.bebauungsplan_art),
  );
  const rechnen = $derived(
    !!projekt?.planungsrecht.rechtsgrundlage &&
      !!projekt?.planungsrecht.bau_nvo,
  );
</script>

<aside class="aktennavigation" aria-label="Arbeitsbereiche">
  <div class="sidebar-scroll">
    <div class="nav-gruppe">Arbeitsplatz</div>
    <button
      class:aktiv={globalAnsicht === "projekte"}
      onclick={() => onglobal("projekte")}
      ><span class="nav-symbol"><NavIcon art="projekte" /></span
      >Projekte</button
    >
    <button
      class:aktiv={globalAnsicht === "kunden"}
      onclick={() => onglobal("kunden")}
      ><span class="nav-symbol"><NavIcon art="kunden" /></span>Kunden /
      Bauherren</button
    >
    <button
      class:aktiv={globalAnsicht === "formulare"}
      onclick={() => onglobal("formulare")}
      ><span class="nav-symbol" aria-hidden="true">▤</span
      >Formularbibliothek</button
    >
    {#if projekt}<div class="nav-gruppe projekt-nav-name" title={projekt.name}>
        {projekt.name}
      </div>
      <details open class="nav-zweig">
        <summary>Grundlagen</summary>
        <div>
          <button
            class:aktiv={!globalAnsicht && ansicht === "uebersicht"}
            onclick={() => onbereich("uebersicht")}>Stammdaten</button
          >
          <button
            class:aktiv={!globalAnsicht && ansicht === "rechtsgrundlagen"}
            onclick={() => onbereich("rechtsgrundlagen")}
            >Rechtsgrundlagen</button
          >
          {#if mitPlan}<button
              class:aktiv={!globalAnsicht && ansicht === "festsetzungen"}
              onclick={() => onbereich("festsetzungen")}
              >Bebauungsplanfestsetzungen</button
            >{/if}
          <button
            class:aktiv={!globalAnsicht &&
              ["grundstueck", "mass"].includes(ansicht)}
            onclick={() => onbereich("grundstueck")}>Baugrundstück</button
          >
        </div>
      </details>
      <details open class="nav-zweig">
        <summary>Berechnungen</summary>
        <div>
          {#each [{ id: "grz", text: "GRZ" }, { id: "gfz", text: "GFZ" }, { id: "geschosse", text: "Geschossigkeit" }, { id: "volumen", text: "Bauvolumen / BMZ" }] as e}<button
              disabled={!rechnen}
              title={!rechnen
                ? "Zuerst Rechtsgrundlage und BauNVO festlegen"
                : undefined}
              class:aktiv={!globalAnsicht && ansicht === e.id}
              onclick={() => onbereich(e.id as Ansicht)}>{e.text}</button
            >{/each}
          <button
            class:aktiv={!globalAnsicht && ansicht === "abstand"}
            onclick={() => onbereich("abstand")}>Abstandsflächen</button
          >
        </div>
      </details>
      <details open class="nav-zweig">
        <summary>Unterlagen</summary>
        <div>
          {#each [{ id: "formulare", text: "Formulare" }, { id: "dokumente", text: "Projektdateien" }, { id: "plaene", text: "Pläne / Aufmaß" }, { id: "bauantrag", text: "Bauantrag / XBau" }, { id: "einreichungen", text: "Einreichungen" }, { id: "protokoll", text: "Chronik" }] as e}<button
              class:aktiv={!globalAnsicht && ansicht === e.id}
              onclick={() => onbereich(e.id as Ansicht)}>{e.text}</button
            >{/each}
        </div>
      </details>{/if}
  </div>
  <div class="sidebar-system">
    <button
      class:aktiv={globalAnsicht === "verwaltung"}
      onclick={() => onglobal("verwaltung")}
      ><span class="nav-symbol" aria-hidden="true">⚙</span
      >Einstellungen</button
    >
  </div>
</aside>

<style>
  .aktennavigation {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .sidebar-scroll {
    overflow-y: auto;
    min-height: 0;
    flex: 1;
  }
  .sidebar-system {
    flex-shrink: 0;
  }
  .sidebar-scroll > button,
  .nav-zweig > div > button,
  .sidebar-system > button {
    display: block;
    width: 100%;
    text-align: left;
  }
  .nav-symbol {
    vertical-align: middle;
  }
  .nav-zweig {
    margin: 0.15rem 0;
  }
  .nav-zweig summary {
    padding: 0.55rem 0.7rem;
    font-weight: 650;
    font-size: 0.83rem;
    cursor: pointer;
    color: var(--muted);
  }
  .nav-zweig > div {
    margin-left: 0.8rem;
    padding-left: 0.4rem;
    border-left: 1px solid var(--border);
  }
  .nav-symbol {
    width: 1.4rem;
    display: inline-block;
    color: var(--muted);
  }
  .sidebar-system {
    border-top: 1px solid var(--border);
    margin-top: 1.2rem;
    padding-top: 0.65rem;
  }
  .projekt-nav-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
