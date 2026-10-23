<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { entwurfspeicher } from "../entwurfspeicher";
  import { projektNavigation } from "../projektNavigation.svelte";
  import { fehlertext } from "../api";
  import type {
    FachPlan,
    Regelkontext,
    JsonValue,
  } from "../generated/plattform";
  type Paket = {
    modul: FachPlan["modul"];
    regelkontext: Regelkontext;
    regeln: JsonValue;
  };
  let {
    plan,
    darf,
    onplan,
  }: { plan: FachPlan; darf: boolean; onplan: (plan: FachPlan) => void } =
    $props();
  const navigation = projektNavigation();
  let pakete = $state<Paket[]>([]),
    auswahl = $state(""),
    meldung = $state(""),
    busy = $state(false),
    beendet = false;
  const key = () =>
    `bauplan.regelpakete.${navigation?.benutzerId ?? "nichtangemeldet"}`;
  const id = (p: Paket) =>
    JSON.stringify([
      p.modul,
      p.regelkontext.bezeichnung,
      p.regelkontext.version,
      p.regelkontext.geltungsbereich,
    ]);
  const aktuelle = $derived(pakete.filter((p) => p.modul === plan.modul));
  const regeln = () => {
    const e = plan.eingaben;
    return e && typeof e === "object" && !Array.isArray(e)
      ? e.regeln
      : undefined;
  };
  const paket = (): Paket => ({
    modul: plan.modul,
    regelkontext: {
      ...JSON.parse(JSON.stringify(plan.regelkontext)),
      anwendbarkeit_bestaetigt: false,
    },
    regeln: JSON.parse(JSON.stringify(regeln() ?? null)),
  });
  onMount(async () => {
    try {
      const p = await entwurfspeicher.lesen(key());
      if (!beendet && Array.isArray(p))
        pakete = p.filter(
          (p) =>
            p && typeof p.modul === "string" && p.regelkontext && "regeln" in p,
        );
    } catch (e) {
      meldung = fehlertext(e);
    }
  });
  onDestroy(() => (beendet = true));
  async function merken() {
    if (
      busy ||
      !darf ||
      !navigation?.benutzerId ||
      !plan.regelkontext.anwendbarkeit_bestaetigt
    )
      return;
    busy = true;
    try {
      const p = paket(),
        alt = pakete.find((a) => id(a) === id(p));
      if (alt && JSON.stringify(alt) !== JSON.stringify(p))
        throw Error("Für geänderte Regeln eine neue Regelversion vergeben.");
      const neu = [...pakete.filter((a) => id(a) !== id(p)), p];
      await entwurfspeicher.schreiben(key(), neu);
      pakete = neu;
      meldung =
        "Versioniertes Regelpaket auf diesem Gerät gespeichert. Die Anwendbarkeit muss in jedem Projekt erneut bestätigt werden.";
    } catch (e) {
      meldung = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  function anwenden() {
    const p = aktuelle.find((p) => id(p) === auswahl);
    if (!p || !darf) return;
    const neu = JSON.parse(JSON.stringify(plan)) as FachPlan;
    neu.regelkontext = {
      ...JSON.parse(JSON.stringify(p.regelkontext)),
      anwendbarkeit_bestaetigt: false,
    };
    if (
      neu.eingaben &&
      typeof neu.eingaben === "object" &&
      !Array.isArray(neu.eingaben)
    )
      neu.eingaben.regeln = JSON.parse(JSON.stringify(p.regeln));
    onplan(neu);
    meldung =
      "Regelpaket übernommen. Geltung, Eingaben und neue Regelversion prüfen, speichern und projektbezogen bestätigen.";
  }
  function exportieren() {
    const p = paket();
    const u = URL.createObjectURL(
      new Blob([JSON.stringify(p, null, 2)], { type: "application/json" }),
    );
    const a = document.createElement("a");
    a.href = u;
    a.download = `Regelpaket-${plan.modul}.json`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(u), 1000);
  }
</script>

{#if regeln() !== undefined}<details>
    <summary>Versionierte Regelpakete wiederverwenden</summary>
    <p>
      Persönliche Pakete auf diesem Gerät. Für das Büro eine geprüfte JSON-Datei
      exportieren und auf den anderen Arbeitsplätzen importieren.
      Projektbezogene Geltung und Freigaben werden nicht übertragen.
    </p>
    <label
      >Gespeichertes Regelpaket<select bind:value={auswahl}
        ><option value="">Paket auswählen</option
        >{#each aktuelle as p (id(p))}<option value={id(p)}
            >{p.regelkontext.bezeichnung} · {p.regelkontext.version} · {p
              .regelkontext.geltungsbereich}</option
          >{/each}</select
      ></label
    ><button
      type="button"
      disabled={!darf || !auswahl || busy}
      onclick={anwenden}>Paket als ungeprüfte Grundlage übernehmen</button
    >
    <button
      type="button"
      class="sekundaer"
      disabled={!darf || busy || !plan.regelkontext.anwendbarkeit_bestaetigt}
      onclick={merken}>Bestätigte Regelversion auf diesem Gerät merken</button
    ><button type="button" class="link" onclick={exportieren}
      >Regelpaket als JSON für das Büro exportieren</button
    >
    {#if meldung}<p role="status">{meldung}</p>{/if}
  </details>{/if}
