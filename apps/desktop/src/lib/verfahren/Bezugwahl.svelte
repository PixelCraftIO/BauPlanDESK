<script lang="ts">
  import type { Verfahrensakte, Bezug } from "../generated/plattform";
  let { akte, werte = $bindable() }: { akte: Verfahrensakte; werte: Bezug[] } =
    $props();
  let auswahl = $state("");
  const optionen = $derived([
    ...akte.anforderungen.map((a) => ({
      name: `Anforderung: ${a.titel}`,
      bezug: { art: "anforderung", id: a.id } as Bezug,
    })),
    ...akte.nachforderungen.flatMap((n) =>
      n.punkte.map((p) => ({
        name: `Nachforderung: ${p.titel}`,
        bezug: { art: "nachforderung", id: n.id, punkt: p.id } as Bezug,
      })),
    ),
    ...akte.abgaben.map((a) => ({
      name: `Abgabe: ${a.titel}`,
      bezug: { art: "abgabe", id: a.id } as Bezug,
    })),
    ...akte.bescheide.map((a) => ({
      name: `Bescheid: ${a.datum}`,
      bezug: { art: "bescheid", id: a.id } as Bezug,
    })),
    ...akte.kommunikation.map((a) => ({
      name: `Kommunikation: ${a.betreff}`,
      bezug: { art: "kommunikation", id: a.id } as Bezug,
    })),
  ]);
</script>

<label
  >Verfahrensbezug<select bind:value={auswahl}
    ><option value="">Bezug auswählen</option>{#each optionen as o}<option
        value={JSON.stringify(o.bezug)}>{o.name}</option
      >{/each}</select
  ></label
><button
  type="button"
  class="sekundaer"
  disabled={!auswahl}
  onclick={() => {
    const b = JSON.parse(auswahl) as Bezug;
    if (!werte.some((w) => JSON.stringify(w) === auswahl))
      werte = [...werte, b];
    auswahl = "";
  }}>Bezug hinzufügen</button
>
<ul>
  {#each werte as w}<li>
      {optionen.find((o) => JSON.stringify(o.bezug) === JSON.stringify(w))
        ?.name ?? w.art}
    </li>{/each}
</ul>
