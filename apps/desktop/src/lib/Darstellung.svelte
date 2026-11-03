<script lang="ts">
  import {
    GROESSEN,
    gespeicherterStil,
    gespeicherteGroesse,
    groesseBeobachten,
    groesseSpeichern,
    speichern,
    type Stil,
  } from "./darstellung";
  let stil = $state<Stil>(gespeicherterStil());
  let groesse = $state(gespeicherteGroesse());
  $effect(() => groesseBeobachten((f) => (groesse = f)));
  // Ein über das Menü eingestellter Zwischenwert (z. B. 120 %) bleibt wählbar.
  const optionen = $derived(
    [...new Set<number>([...GROESSEN, groesse])].sort((a, b) => a - b),
  );
  const prozent = (f: number) => `${Math.round(f * 100)} %`;
</script>

<section class="einstellungsabschnitt">
  <h2>Darstellung</h2>
  <label
    >Farbschema<select
      aria-label="Farbschema"
      bind:value={stil}
      onchange={() => speichern(stil)}
      ><option value="system">Wie Betriebssystem</option><option value="light"
        >Hell</option
      ><option value="dark">Dunkel</option></select
    ></label
  >
  <label
    >Oberflächengröße<select
      aria-label="Oberflächengröße"
      value={groesse}
      onchange={(e) => groesseSpeichern(Number(e.currentTarget.value))}
      >{#each optionen as f (f)}<option value={f}
          >{prozent(f)}{f === 1 ? " (Standard)" : ""}</option
        >{/each}</select
    ></label
  >
  <p class="meta">
    Die Oberflächengröße vergrößert Schrift, Felder und Abstände gemeinsam.
    Tastatur: Strg + Plus, Strg + Minus, Strg + 0 (Menü „Fenster“). Beide
    Einstellungen gelten für diesen Arbeitsplatz und bleiben nach einem Neustart
    erhalten.
  </p>
</section>
