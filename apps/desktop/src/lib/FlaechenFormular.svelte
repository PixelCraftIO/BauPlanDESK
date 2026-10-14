<script lang="ts">
  import type {
    FlaechenAufstellung,
    FlaechenErgebnis,
    Bauteiltyp,
    Anrechnung,
  } from "./api";
  let {
    aufstellung = $bindable(),
    gebaeude = [],
    ergebnis,
    festsetzungen,
    bereich = "alle",
  }: {
    bereich?: "alle" | "grz" | "gfz";
    gebaeude?: import("./plattformApi").Datensatz<
      import("./generated/plattform").Gebaeude
    >[];
    aufstellung: FlaechenAufstellung;
    ergebnis: FlaechenErgebnis | null;
    festsetzungen: Festsetzungen;
  } = $props();
  import { regelwerk } from "./baunvo";
  import type { Festsetzungen } from "./api";
  const regel = $derived(regelwerk(aufstellung.bau_nvo));
  const modern = $derived(regel.grz_ii);
  const typen = $derived(
    Object.entries(regel.bauteile).map(([code, r]) => ({
      code: code as Bauteiltyp,
      text: r.bezeichnung,
    })),
  );
  const namen: Record<Anrechnung, string> = $derived({
    grz: modern ? "GRZ I" : "GRZ",
    grz_ii: "GRZ II – Zusatzfläche",
    keine: "Nicht angerechnet",
  });
  const faktor = $derived(
    regel.nichtvollgeschoss_planabweichung
      ? (festsetzungen.nichtvollgeschoss_faktor ??
          (aufstellung.nichtvollgeschoss_festsetzung.trim()
            ? aufstellung.nichtvollgeschoss_faktor
            : regel.nichtvollgeschoss_faktor))
      : regel.nichtvollgeschoss_faktor,
  );
  const z = (v: number) =>
    v.toLocaleString("de-DE", { maximumFractionDigits: 3 });
</script>

<section class="flaechenformular">
  {#if bereich !== "gfz"}<h3>
      Bauteile für {modern ? "GRZ I und GRZ II" : "GRZ"}
    </h3>
    <p class="meta">
      Grundflächen in horizontaler Projektion. Überlagerte Flächen nur einmal
      erfassen, beispielsweise eine Tiefgarage unter dem Hauptgebäude nur mit
      dem zusätzlichen Umgriff. Terrassen nach ihrer Zugehörigkeit zur Haupt-
      oder Nebenanlage einordnen.
    </p>
    {#if !modern}<p class="hinweisbox">
        Diese Fassung kennt keine GRZ II und keine pauschale 50-%-Überschreitung
        nach heutigem § 19 Abs. 4. Garagen werden grundsätzlich angesetzt;
        Sonderregelungen, etwa nach § 21a, benötigen einen gesondert geprüften
        Ansatz.
      </p>{/if}
    {#each aufstellung.bauteile as b, i}
      <fieldset>
        <legend>Bauteil {i + 1}</legend>
        <div class="raster">
          <label
            >Bezeichnung<input
              bind:value={b.bezeichnung}
              required
              maxlength="500"
              placeholder="z. B. Wohnhaus"
            /></label
          >
          <label
            >Bauteiltyp<select bind:value={b.typ}
              >{#each typen as t}<option value={t.code}>{t.text}</option
                >{/each}</select
            ></label
          >
          <label
            >Grundfläche (m²)<input
              type="number"
              min="0"
              step="any"
              bind:value={b.flaeche_m2}
              required
            /></label
          >
          <div>
            <span class="meta">Automatische Anrechnung</span>
            <p>
              {namen[
                b.abweichende_anrechnung ?? regel.bauteile[b.typ].anrechnung
              ]}
            </p>
          </div>
          {#if b.abweichende_anrechnung !== null}<details>
              <summary>Gespeicherter Sonderansatz</summary>
              <p>{b.begruendung}</p>
              <button
                type="button"
                class="sekundaer"
                onclick={() => {
                  b.abweichende_anrechnung = null;
                  b.begruendung = "";
                }}>Konfigurierten Regelansatz verwenden</button
              >
            </details>{/if}
        </div>
        {#if ergebnis}<p class="meta">
            Angesetzt: {z(ergebnis.bauteile[i].flaeche_m2)} m² · {namen[
              ergebnis.bauteile[i].anrechnung
            ]}
          </p>{/if}
        <button
          type="button"
          class="sekundaer"
          onclick={() => aufstellung.bauteile.splice(i, 1)}
          >Bauteil entfernen</button
        >
      </fieldset>
    {/each}
    <button
      type="button"
      class="sekundaer"
      onclick={() =>
        aufstellung.bauteile.push({
          bezeichnung: "",
          typ: "hauptanlage",
          flaeche_m2: 0,
          abweichende_anrechnung: null,
          begruendung: "",
        })}>Bauteil hinzufügen</button
    >
  {/if}{#if bereich !== "grz"}<h3>Geschosse für GFZ und Geschossigkeit</h3>
    <p class="meta">
      Vollgeschosse werden nach Außenmaßen angesetzt. Die Einstufung als
      Vollgeschoss ist nach dem für das Vorhaben maßgebenden Landesrecht zu
      prüfen. Jede Ebene einmal erfassen; mehrere Gebäudeteile derselben Ebene
      zusammenfassen.
    </p>
    <p class="hinweisbox">
      {faktor === 0
        ? "Nichtvollgeschosse werden nach diesem Regelwerk nicht auf die GFZ angerechnet."
        : `Aufenthaltsräume, Umfassungswände und zugehörige Erschließung in Nichtvollgeschossen werden mit ${z(faktor * 100)} % angerechnet.`}
      {regel.nichtvollgeschoss_planabweichung &&
      festsetzungen.nichtvollgeschoss_faktor != null
        ? "Die abweichende Festsetzung stammt aus den Projekt-Rechtsgrundlagen."
        : "Der Ansatz wird automatisch aus der BauNVO-Konfiguration übernommen."}
    </p>
    {#each aufstellung.geschosse as g, i}
      <fieldset>
        <legend>Geschoss {i + 1}</legend>
        <div class="raster">
          {#if gebaeude.length}<label
              >Gebäude<select
                bind:value={g.gebaeude_id}
                onchange={() => (g.geschoss_id = null)}
                ><option value={null}>Noch nicht zugeordnet</option
                >{#each gebaeude as haus (haus.objekt)}<option
                    value={haus.objekt}>{haus.daten.bezeichnung}</option
                  >{/each}</select
              ></label
            >
            {@const haus = gebaeude.find((h) => h.objekt === g.gebaeude_id)}
            {#if haus?.daten.geschosse.length}<label
                >Geschoss aus Gebäudemodell<select
                  bind:value={g.geschoss_id}
                  onchange={() => {
                    const quelle = haus.daten.geschosse.find(
                      (s) => s.id === g.geschoss_id,
                    );
                    if (quelle) {
                      g.bezeichnung = quelle.bezeichnung;
                      if (quelle.vollgeschoss !== null)
                        g.vollgeschoss = quelle.vollgeschoss;
                    }
                  }}
                  ><option value={null}>Geschoss auswählen</option
                  >{#each haus.daten.geschosse as geschoss (geschoss.id)}<option
                      value={geschoss.id}
                      >{geschoss.bezeichnung}{geschoss.vollgeschoss === null
                        ? " – Einstufung offen"
                        : ""}</option
                    >{/each}</select
                ></label
              >{/if}
          {/if}
          <label
            >Bezeichnung<input
              bind:value={g.bezeichnung}
              readonly={!!g.geschoss_id}
              required
              maxlength="500"
              placeholder="z. B. Dachgeschoss"
            /></label
          >
          <label
            >Geschosstyp<select
              bind:value={g.vollgeschoss}
              disabled={!!g.geschoss_id}
              ><option value={true}>Vollgeschoss</option><option value={false}
                >Nichtvollgeschoss</option
              ></select
            ></label
          >
          {#if g.vollgeschoss || faktor > 0}<label
              >Geschossfläche nach Außenmaßen (m²)<input
                type="number"
                min="0"
                step="any"
                bind:value={g.aussenmass_m2}
                required
              /></label
            >{/if}
          {#if !g.vollgeschoss && faktor > 0}
            <label
              >Aufenthaltsräume (m²)<input
                type="number"
                min="0"
                step="any"
                bind:value={g.aufenthalt_m2}
                required
              /></label
            >
            <label
              >Zugehörige Umfassungswände (m²)<input
                type="number"
                min="0"
                step="any"
                bind:value={g.waende_m2}
                required
              /></label
            >
            <label
              >Zugehörige Erschließung / Treppenräume (m²)<input
                type="number"
                min="0"
                step="any"
                bind:value={g.erschliessung_m2}
                required
              /></label
            >
          {/if}
        </div>
        {#if ergebnis}<p class="meta">
            Angesetzte Geschossfläche: {z(ergebnis.geschosse[i])} m²
          </p>{/if}
        <button
          type="button"
          class="sekundaer"
          onclick={() => aufstellung.geschosse.splice(i, 1)}
          >Geschoss entfernen</button
        >
      </fieldset>
    {/each}
    <button
      type="button"
      class="sekundaer"
      onclick={() =>
        aufstellung.geschosse.push({
          bezeichnung: "",
          vollgeschoss: true,
          aussenmass_m2: 0,
          aufenthalt_m2: 0,
          waende_m2: 0,
          erschliessung_m2: 0,
        })}>Geschoss hinzufügen</button
    >
  {/if}
  {#if ergebnis}<p class="meta">
      Regelwerk: {ergebnis.regelwerk_id}, Version {ergebnis.regelwerk_version}
    </p>
    <p class="hinweisbox">
      {#if bereich !== "gfz"}Grundfläche: {ergebnis.bauteile_vorhanden
          ? `${z(ergebnis.mass.grundflaeche_m2)} m²`
          : "Bauteile fehlen"}{/if}
      {#if bereich !== "grz"}Geschossfläche: {ergebnis.geschosse_vorhanden
          ? `${z(ergebnis.mass.geschossflaeche_m2)} m² · ${ergebnis.vollgeschosse} Vollgeschosse`
          : "Geschosse fehlen"}{/if}
    </p>{/if}
</section>
