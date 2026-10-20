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
    projektId,
    grundstueckId,
    ergebnis,
    festsetzungen,
    bereich = "alle",
    einzigesGrundstueck = false,
    flurstueckIds = [],
  }: {
    projektId: string;
    grundstueckId?: string;
    bereich?: "alle" | "grz" | "gfz";
    einzigesGrundstueck?: boolean;
    /** Flurstücke dieses Baugrundstücks laut Ermittlung. */
    flurstueckIds?: string[];
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
  // Wirksamer Faktor aus dem Core-Ergebnis (Regelwerk und, nur mit
  // Bebauungsplan, Planfestsetzung). Vor der ersten Prüfung unbekannt.
  const faktor = $derived(ergebnis?.nichtvollgeschoss_faktor ?? null);
  import { plattformApi } from "./plattformApi";
  import { fehlertext } from "./api";
  import type { GebaeudeGrundlagen } from "./generated/plattform";
  let importId = $state("");
  let importFehler = $state("");
  let importHinweis = $state("");
  let importLaeuft = $state(false);
  let einzelEinstufung = $state<boolean | null>(null);
  // Angebot zur erneuten Verknüpfung: Gebäude mit einem Flurstück dieses
  // Grundstücks (Altbestand: frühere Zuordnung); ob die Lage eindeutig ist,
  // prüft der Core beim Prüfen und Speichern.
  const importGebaeude = $derived(gebaeude.filter((g) => {
    const f = g.daten.flurstueck_ids ?? [];
    if (f.length) return f.some((id) => flurstueckIds.includes(id));
    if (grundstueckId && g.daten.grundstueck_ids.includes(grundstueckId)) return true;
    return einzigesGrundstueck;
  }));
  async function uebernehmen() {
    const haus = importGebaeude.find((g) => g.objekt === importId);
    if (!haus || importLaeuft) return;
    // Die Antwort bleibt an die ursprünglich bearbeitete Aufstellung gebunden.
    const ziel = aufstellung;
    const istGrz = bereich !== "gfz";
    importLaeuft = true; importFehler = ""; importHinweis = "";
    try {
      const basis: GebaeudeGrundlagen = await plattformApi.gebaeudeAuswerten(projektId, haus.objekt, JSON.parse(JSON.stringify(haus.daten)));
      if (ziel !== aufstellung) return;
      if (istGrz) {
        if (basis.grundflaeche_m2 === null || basis.typ === null) throw Error("Grundfläche und Gebäudetyp zuerst in Gebäude/Geschosse ergänzen.");
        if (ziel.bauteile.some((b) => b.gebaeude_id === haus.objekt)) throw Error("Die Grundfläche dieses Gebäudes ist bereits verknüpft.");
        ziel.bauteile.push({gebaeude_id:haus.objekt,bezeichnung:basis.bezeichnung,typ:basis.typ,flaeche_m2:basis.grundflaeche_m2,abweichende_anrechnung:null,begruendung:""});
        importHinweis = "Gebäudegrundfläche verknüpft. Überlagerungen mit anderen Bauteilen vor der Berechnung prüfen.";
      } else {
        const vorhanden = ziel.geschosse.filter((g) => g.gebaeude_id === haus.objekt);
        if (vorhanden.some((g) => !g.geschoss_id)) throw Error("Für dieses Gebäude ist bereits ein Einzelgeschoss oder eine manuelle Ebene erfasst. Zuordnung vor der Übernahme prüfen.");
        const neu = basis.geschosse.filter((g) => !vorhanden.some((v) => v.geschoss_id === g.id));
        if (!neu.length) throw Error("Alle Geschosse dieses Gebäudes sind bereits erfasst.");
        if (neu.some((g) => g.flaeche_m2 === null)) throw Error("Die Flächen aller zu übernehmenden Geschosse zuerst in den Grundlagen ergänzen.");
        // Eine im Gebäude festgelegte Einstufung des Einzelgeschosses ist maßgebend.
        const einstufung = (g: (typeof neu)[number]) => (g.implizit ? (g.vollgeschoss ?? einzelEinstufung) : g.vollgeschoss);
        if (neu.some((g) => g.implizit && g.vollgeschoss !== null && einzelEinstufung !== null && g.vollgeschoss !== einzelEinstufung)) throw Error("Die gewählte Einstufung widerspricht der Einstufung in Gebäude/Geschosse. Dort ändern.");
        if (neu.some((g) => einstufung(g) === null)) throw Error("Vollgeschosseinstufung zuerst prüfen und festlegen, am besten unter Gebäude/Geschosse.");
        if (ziel.geschosse.some((g) => !g.gebaeude_id)) throw Error("Vorhandene Geschosse zuerst ihren Gebäuden zuordnen.");
        for (const g of neu) ziel.geschosse.push({gebaeude_id:haus.objekt,geschoss_id:g.implizit ? null : g.id,flaeche_aus_grundlagen:true,teilflaechen_geprueft:false,bezeichnung:g.bezeichnung,vollgeschoss:einstufung(g)!,aussenmass_m2:g.flaeche_m2!,aufenthalt_m2:0,waende_m2:0,erschliessung_m2:0});
        importHinweis = "Geschossflächen verknüpft. Bei anzurechnenden Nichtvollgeschossen Aufenthaltsräume, Wände und Erschließung gesondert ergänzen.";
      }
    } catch (e) { if (ziel === aufstellung) importFehler = fehlertext(e); }
    finally { importLaeuft = false; }
  }
  const z = (v: number) =>
    v.toLocaleString("de-DE", { maximumFractionDigits: 3 });
</script>

<section class="flaechenformular">
  {#if gebaeude.length}<fieldset>
    <legend>Bekannte Flächen aus Gebäude/Geschosse</legend>
    <p class="meta">Gebäude, die diesem Baugrundstück zugeordnet sind, werden mit vollständigen Grundlagen automatisch übernommen und beim Prüfen und Speichern aktualisiert. Hier lässt sich ein Gebäude erneut verknüpfen, etwa nach „Verknüpfung lösen“ oder um die Einstufung eines Einzelgeschosses festzulegen. Bereits manuell erfasste Flächen auf Doppelungen prüfen.</p>
    <label>Gebäude<select bind:value={importId} onchange={() => { einzelEinstufung = null; importFehler = ""; importHinweis = ""; }}><option value="">Gebäude auswählen</option>{#each importGebaeude as haus}<option value={haus.objekt}>{haus.daten.bezeichnung}</option>{/each}</select></label>
    {#if bereich === "gfz" && importGebaeude.find((g) => g.objekt === importId)?.daten.geschosse.length === 0}
      {@const imGebaeude = importGebaeude.find((g) => g.objekt === importId)?.daten.geometrie?.einzelgeschoss_vollgeschoss}
      {#if imGebaeude === undefined}<label>Geprüfte Einstufung des Einzelgeschosses<select bind:value={einzelEinstufung}><option value={null}>Noch zu prüfen</option><option value={true}>Vollgeschoss</option><option value={false}>Nichtvollgeschoss</option></select></label>
        <p class="meta">Besser unter Gebäude/Geschosse festlegen; dann gilt sie für alle Berechnungen.</p>
      {:else}<p class="meta">Einstufung des Einzelgeschosses laut Gebäude/Geschosse: {imGebaeude ? "Vollgeschoss" : "Nichtvollgeschoss"}.</p>{/if}
    {/if}
    <button type="button" class="sekundaer" disabled={!importId || importLaeuft} onclick={uebernehmen}>{bereich === "gfz" ? "Geschossflächen verknüpfen" : "Gebäudegrundfläche verknüpfen"}</button>
    {#if !importGebaeude.length}<p class="meta">Für eine Übernahme dem Gebäude unter Gebäude/Geschosse die Flurstücke zuordnen, auf denen es steht. Bei Gebäuden auf mehreren Baugrundstücken die Flächen getrennt aufteilen.</p>{/if}
    {#if importFehler}<p class="fehlertext" role="alert">{importFehler}</p>{/if}
    {#if importHinweis}<p class="meta" role="status">{importHinweis}</p>{/if}
  </fieldset>{/if}
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
            >Bezeichnung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Bezeichnung"
              bind:value={b.bezeichnung}
              required
              maxlength="500"
              placeholder="z. B. Wohnhaus"
            /></label
          >
          <label
            >Bauteiltyp<select bind:value={b.typ}
              disabled={!!b.gebaeude_id}
              >{#each typen as t}<option value={t.code}>{t.text}</option
                >{/each}</select
            ></label
          >
          <label
            >Grundfläche (m²)<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Grundfläche (m²)"
              type="number"
              min="0"
              step="any"
              bind:value={b.flaeche_m2}
              readonly={!!b.gebaeude_id}
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
        {#if b.gebaeude_id}<p class="meta">Fläche und Typ aus Gebäude/Geschosse. <button type="button" class="link" onclick={() => { b.gebaeude_id = undefined; }}>Verknüpfung lösen</button></p>{/if}
        {#if ergebnis?.bauteile[i]}<p class="meta">
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
      {#if faktor === null}
        Der Anrechnungsansatz für Nichtvollgeschosse wird bei der Prüfung aus
        dem Regelwerk und gegebenenfalls der Planfestsetzung ermittelt.
      {:else}
        {faktor === 0
          ? "Nichtvollgeschosse werden nach diesem Regelwerk nicht auf die GFZ angerechnet."
          : `Aufenthaltsräume, Umfassungswände und zugehörige Erschließung in Nichtvollgeschossen werden mit ${z(faktor * 100)} % angerechnet.`}
        {faktor !== regel.nichtvollgeschoss_faktor
          ? "Der Ansatz folgt einer abweichenden Planfestsetzung (Stand der letzten Prüfung)."
          : "Der Ansatz folgt der BauNVO-Konfiguration (Stand der letzten Prüfung)."}
      {/if}
    </p>
    {#each aufstellung.geschosse as g, i}
      <fieldset>
        <legend>Geschoss {i + 1}</legend>
        <div class="raster">
          {#if gebaeude.length}<label
              >Gebäude<select
                bind:value={g.gebaeude_id}
                onchange={() => { g.geschoss_id = null; g.flaeche_aus_grundlagen = undefined; }}
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
                    g.flaeche_aus_grundlagen = undefined;
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
            >Bezeichnung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Bezeichnung"
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
          {#if g.vollgeschoss || faktor !== 0}<label
              >Geschossfläche nach Außenmaßen (m²)<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Geschossfläche nach Außenmaßen (m²)"
                type="number"
                min="0"
                step="any"
                bind:value={g.aussenmass_m2}
                readonly={g.flaeche_aus_grundlagen === true}
                required
              /></label
            >{/if}
          {#if !g.vollgeschoss && faktor !== 0}
            <label
              >Aufenthaltsräume (m²)<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Aufenthaltsräume (m²)"
                type="number"
                min="0"
                step="any"
                bind:value={g.aufenthalt_m2}
                required
              /></label
            >
            <label
              >Zugehörige Umfassungswände (m²)<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Zugehörige Umfassungswände (m²)"
                type="number"
                min="0"
                step="any"
                bind:value={g.waende_m2}
                required
              /></label
            >
            <label
              >Zugehörige Erschließung / Treppenräume (m²)<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Zugehörige Erschließung / Treppenräume (m²)"
                type="number"
                min="0"
                step="any"
                bind:value={g.erschliessung_m2}
                required
              /></label
            >
          {/if}
          {#if !g.vollgeschoss && faktor !== 0 && g.teilflaechen_geprueft !== undefined}<label><input type="checkbox" bind:checked={g.teilflaechen_geprueft} />Aufenthaltsräume, Wände und Erschließung vollständig geprüft</label>{/if}
        </div>
        {#if g.flaeche_aus_grundlagen}<p class="meta">Geschossfläche aus Gebäude/Geschosse. <button type="button" class="link" onclick={() => { g.flaeche_aus_grundlagen = undefined; }}>Flächenverknüpfung lösen</button></p>{/if}
        {#if ergebnis && ergebnis.geschosse[i] !== undefined}<p class="meta">
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
