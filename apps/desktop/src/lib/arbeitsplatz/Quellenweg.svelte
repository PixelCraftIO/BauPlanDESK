<script lang="ts">
  import { editorRegister } from "../editoren";
  import type { Ansicht } from "../projektSitzung";
  import { projektNavigation } from "../projektNavigation.svelte";
  import { ziel } from "./arbeitscheck";
  let { bereich }: { bereich: string } = $props();
  const navigation = projektNavigation();
  const editoren = editorRegister();
  let busy = $state(false),
    meldung = $state("");
  async function quelleSpeichern(id: string) {
    if (busy) return;
    busy = true;
    meldung = "";
    try {
      const ok = await editoren?.speichernIn(id as Ansicht);
      meldung = ok
        ? "Quelle gespeichert. Werte bewusst übernehmen und neu prüfen."
        : "Quelle konnte nicht vollständig gespeichert werden. Den Entwurf an der Quelle prüfen.";
    } catch {
      meldung =
        "Speichern fehlgeschlagen; der Entwurf bleibt an der Quelle erhalten.";
    } finally {
      busy = false;
    }
  }
  const wege: Record<
    string,
    { titel: string; text: string; links: [string, string][] }
  > = {
    grundstueck: {
      titel: "Flurstücke → Baugrundstück → Nutzungsnachweise",
      text: "Flurstücke sind die Faktenbasis. Nach Änderungen Zuordnung und Bezugsflächen neu ermitteln und speichern; anschließend betroffene Nachweise prüfen.",
      links: [
        ["flurstuecke", "Flurstücke"],
        ["festsetzungen", "Planfestsetzungen"],
        ["nachweise", "Betroffene Nachweise"],
      ],
    },
    nutzung: {
      titel: "Grundlagen → Baugrundstück und Gebäude → Berechnung",
      text: "Bezugsflächen kommen aus der Baugrundstücksermittlung. Gebäude und Geschosse liefern zugeordnete Flächen; die maßgebende BauNVO bestimmt ihre Anrechnung. Nach Änderungen neu berechnen und einen neuen Nachweis archivieren.",
      links: [
        ["rechtsgrundlagen", "Rechtsgrundlagen"],
        ["grundstueck", "Bezugsflächen"],
        ["gebaeude", "Gebäude / Geschosse"],
        ["nachweise", "Nachweisstände"],
      ],
    },
    abstand: {
      titel: "Wandgeometrie und Regeln → Abstandsfläche → Nachweis",
      text: "Originalquellen und Planversionen sowie verknüpfte Aufmaße prüfen. Nach geänderten Höhen, Grenzen oder Quellen neu berechnen und einen neuen Nachweisstand archivieren. Die fachliche Freigabe erfolgt in den Nachweisständen.",
      links: [
        ["gebaeude", "Gebäude / Geschosse"],
        ["planaufmass", "PDF-Aufmaß"],
        ["nachweise", "Nachweisstände"],
      ],
    },
    fach: {
      titel: "Eingaben und Quellen → Prüfbericht → Nachweisstand",
      text: "Unter Quellen / Übernahmen die verknüpften Datenstände prüfen. Geänderte Quellen bewusst aktualisieren, neu berechnen und anschließend archivieren. Freigaben gelten für den jeweiligen Nachweisstand.",
      links: [
        ["grundstueck", "Baugrundstück"],
        ["planaufmass", "PDF-Aufmaß"],
        ["nachweise", "Nachweisstände"],
      ],
    },
    unterlagen: {
      titel: "Anforderung → Dokumentversion → Abgabe → Eingangsbeleg",
      text: "Formulare im Projekt ausfüllen und speichern. In den Anforderungen und Unterlagen konkrete Dokumentversionen zuordnen; korrigierte Dateien als neue Version ablegen. Abgabestände halten die tatsächlich ausgewählten Versionen fest.",
      links: [
        ["verfahren", "Anforderungen / Verfahrensakte"],
        ["unterlagen", "Unterlagenregister"],
        ["formulare", "Formulare ausfüllen"],
        ["dokumente", "Projektdateien"],
        ["einreichen", "Abgabe vorbereiten"],
      ],
    },
  };
  const weg = $derived(
    wege[bereich] ??
      (["grz", "gfz", "geschosse", "volumen"].includes(bereich)
        ? wege.nutzung
        : ["dokumente", "formulare", "druck"].includes(bereich)
          ? wege.unterlagen
          : wege.fach),
  );
</script>

<details class="quellenweg">
  <summary>Datenherkunft und nächste Schritte</summary><strong
    >{weg.titel}</strong
  >
  <p>{weg.text}</p>
  <div>
    {#each weg.links.filter(([id]) => id !== bereich) as [id, titel]}<button
        type="button"
        class="link"
        onclick={() => {
          if (navigation) navigation.rueckweg = ziel(bereich);
          navigation?.springen(ziel(id));
        }}>{titel} öffnen →</button
      >
      {#if editoren?.hatAenderungenIn(id as Ansicht)}<span class="hinweisbox"
          >Quelle hat ungespeicherte Änderungen. <button
            type="button"
            class="link"
            disabled={busy}
            onclick={() => quelleSpeichern(id)}
            >Nur diese Quelle speichern</button
          ></span
        >{/if}{/each}
  </div>
  {#if meldung}<p role="status">{meldung}</p>{/if}
</details>

<style>
  .quellenweg {
    grid-column: 1/-1;
    padding: 0.6rem 1rem;
    border: 1px solid var(--border);
    background: var(--leiste);
    margin-bottom: 0.8rem;
  }
  summary {
    font-weight: 600;
  }
  p {
    max-width: 100ch;
    margin: 0.6rem 0;
  }
  div {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
  }
</style>
