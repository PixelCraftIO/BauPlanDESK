<script lang="ts">
  import { gespeicherterStil, speichern, type Stil } from "./darstellung";
  let stil = $state<Stil>(gespeicherterStil());
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
  <p class="meta">
    Gilt für diesen Arbeitsplatz und bleibt nach einem Neustart erhalten.
  </p>
</section>
