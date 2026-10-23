<script lang="ts">
  import { untrack } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import type { Project, GrundstueckKontext } from "../api";
  import { api, fehlertext } from "../api";
  import { grundstuecksinformationen } from "../grundstuecksdaten";
  import type {
    Gebaeude,
    NachweisAnsicht,
    FachErgebnis,
    Dachform,
  } from "../generated/plattform";
  import { plattformApi } from "../plattformApi";
  import { FachSitzung } from "../FachSitzung.svelte";
  import { editorRegister } from "../editoren";
  import Fachliste from "../Fachliste.svelte";
  import Planquellen from "../planaufmass/Planquellen.svelte";
  import PlanFlaeche from "../planaufmass/PlanFlaeche.svelte";
  import { aufmasseLaden, type Aufmassquelle } from "../planaufmass/quellen";
  import { bereichAktiv, ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  import { projektNavigation } from "../projektNavigation.svelte";
  import GrundlagenZahl from "./GrundlagenZahl.svelte";
  import GeschossMasse from "./GeschossMasse.svelte";
  import type { GebaeudeGrundlagen } from "../generated/plattform";
  let {
    projekt,
    benutzerId,
    darf,
  }: { projekt: Project; benutzerId: string; darf: boolean } = $props();
  const s = untrack(
    () =>
      new FachSitzung<Gebaeude>(
        projekt.id,
        "gebaeude",
        benutzerId,
        () => ({
          bezeichnung: "",
          // Bei genau einem Vorhabenflurstück ist die Lage eindeutig.
          flurstueck_ids: vorhaben.length === 1 ? [vorhaben[0].id] : [],
          grundstueck_ids: [],
          geschosse: [],
          quelle: null,
        }),
        (g) => g.bezeichnung,
      ),
  );
  let nachweise = $state<NachweisAnsicht[]>([]);
  let vorschau = $state<GebaeudeGrundlagen | null>(null);
  let vorschauFehler = $state("");
  let planBusy = $state(false),
    quellenGeneration = $state(0);
  const sichtbar = bereichAktiv();
  ansichtsAbfrage(
    () => projekt.id,
    ["fachobjekte", "dokumente"],
    async () => {
      quellenGeneration++;
    },
  );
  // Gespeicherte Aufmaße für die direkte Flächenübernahme; neu geladen, sobald
  // ein Aufmaß oder eine PDF gespeichert wird.
  let aufmasse = $state<Aufmassquelle[]>([]);
  $effect(() => {
    if (!sichtbar()) return;
    void quellenGeneration;
    let aktuell = true;
    aufmasseLaden(projekt.id)
      .then((a) => {
        if (aktuell) aufmasse = a;
      })
      .catch(() => {
        /* Planquellen zeigt Ladefehler */
      });
    return () => {
      aktuell = false;
    };
  });
  // Sprung aus der Projektübersicht zu veralteten Planquellen öffnet den Abschnitt.
  const navigation = projektNavigation();
  let planOffen = $state(false);
  $effect(() => {
    const a = navigation?.auftrag;
    if (a?.ziel.bereich === "gebaeude" && a.ziel.register === "planaufmass")
      planOffen = true;
  });
  async function geschossEntfernen(g: Gebaeude, id: string) {
    // Markierungen mit diesem Geschoss könnten danach nicht mehr gespeichert werden.
    const marken = aufmasse.flatMap((a) =>
      a.stand.daten.markierungen
        .filter((m) => m.geschoss_id === id)
        .map((m) => `„${a.stand.daten.bezeichnung} · ${m.bezeichnung}“`),
    );
    if (
      marken.length &&
      !(await ask(
        `Im PDF-Aufmaß sind diesem Geschoss zugeordnet: ${marken.join(", ")}. Nach dem Entfernen dort ein anderes Geschoss wählen, sonst lässt sich das Aufmaß nicht speichern. Geschoss trotzdem entfernen?`,
        { title: "Geschoss entfernen", kind: "warning" },
      ))
    )
      return;
    g.planbindungen = g.planbindungen?.filter(
      (b) => !b.ziel.startsWith(`geschosse.${id}.`),
    );
    const i = g.geschosse.findIndex((x) => x.id === id);
    if (i >= 0) g.geschosse.splice(i, 1);
  }
  const geschossGespeichert = (id: string) =>
    !!s.original?.geschosse.some((x) => x.id === id);
  const DACHFORMEN: [Dachform, string][] = [
    ["flachdach", "Flachdach"],
    ["satteldach", "Satteldach"],
    ["pultdach", "Pultdach"],
    ["walmdach", "Walmdach"],
    ["sonstige", "Sonstige Dachform"],
  ];
  const hoehenfelder = [
    ["nullpunkt_m_nhn", "Gebäudenullpunkt ±0,00 (m ü. NHN)"],
    ["gelaende_m", "Maßgebende mittlere Geländehöhe zu ±0,00 (m)"],
    ["fundamentsohle_m", "Fundamentsohle zu ±0,00 (m)"],
    ["traufe_m", "Traufhöhe zu ±0,00 (m)"],
    ["first_m", "Firsthöhe zu ±0,00 (m)"],
  ] as const;
  const z = (n: number | null) =>
    n === null
      ? "Noch unbekannt"
      : n.toLocaleString("de-DE", { maximumFractionDigits: 3 });
  $effect(() => {
    if (!sichtbar()) return;
    void quellenGeneration;
    const daten = JSON.stringify(s.eingabe);
    const id = s.id;
    vorschau = null;
    vorschauFehler = "";
    if (!s.eingabe || !id) return;
    let abgebrochen = false;
    const timer = setTimeout(async () => {
      try {
        const r = await plattformApi.gebaeudeAuswerten(
          s.projektId,
          id,
          JSON.parse(daten),
        );
        if (!abgebrochen) vorschau = r;
      } catch (e) {
        if (!abgebrochen) vorschauFehler = fehlertext(e);
      }
    }, 300);
    return () => {
      abgebrochen = true;
      clearTimeout(timer);
    };
  });
  async function quellstaende() {
    try {
      nachweise = await plattformApi.nachweise(projekt.id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  ansichtsAbfrage(() => projekt.id, ["nachweise"], quellstaende);
  function quellen(geschossId: string) {
    return nachweise.flatMap((n) => {
      if (
        !n.aktuell ||
        !n.stand.vollstaendig ||
        n.stand.eingabe.art !== "fach" ||
        n.stand.eingabe.plan.modul !== "vollgeschosse" ||
        n.stand.eingabe.plan.gebaeude_id !== s.id
      )
        return [];
      const input = n.stand.eingabe.plan.eingaben as {
        geschosse: { id: string; gebaeude_geschoss_id: string | null }[];
      };
      const r = n.stand.ergebnis as unknown as FachErgebnis;
      return r.kennzahlen
        .filter(
          (k) =>
            k.id.startsWith("geschoss:") &&
            input.geschosse.some(
              (s) =>
                `geschoss:${s.id}` === k.id &&
                s.gebaeude_geschoss_id === geschossId,
            ),
        )
        .map((k) => ({
          nachweis: n.stand.id,
          kennzahl: k.id,
          wert: k.wert,
          titel: `${n.stand.bezeichnung} · ${k.bezeichnung}`,
        }));
    });
  }
  // Gebäude stehen auf Flurstücken; das Baugrundstück ergibt sich erst aus der
  // Baugrundstücksermittlung und wird hier nur zur Information angezeigt.
  const flurstuecke = $derived(
    grundstuecksinformationen($state.snapshot(projekt.planungsrecht))
      .flurstuecke,
  );
  const vorhaben = $derived(
    flurstuecke.filter((f) => (f.rolle ?? "vorhaben") === "vorhaben"),
  );
  const flurName = (f: (typeof flurstuecke)[number]) =>
    `${f.gemarkung || "Gemarkung"} · Flur ${f.flur || "—"} · ${f.nummer || "Flurstück"}`;
  // Flurstücke stehen hier erst nach dem Speichern der Grundstücksinformationen.
  const editoren = editorRegister();
  const flurstueckEntwurf = $derived(
    !!editoren?.hatAenderungenIn("flurstuecke"),
  );
  let grund = $state<GrundstueckKontext[]>([]);
  let grundGeladen = $state(false);
  // Nach jeder Projektspeicherung (z. B. neue Baugrundstücksermittlung) neu laden.
  $effect(() => {
    void projekt.version;
    let aktuell = true;
    api
      .grundstueckskontext(projekt.id)
      .then((g) => {
        if (aktuell) {
          grund = g;
          grundGeladen = true;
        }
      })
      .catch(() => {
        // Ohne bestätigte Ermittlung entfällt nur die Anzeige des Baugrundstücks.
      });
    return () => {
      aktuell = false;
    };
  });
  /** Baugrundstücke, zu denen die gewählten Flurstücke derzeit gehören. */
  const einheiten = (g: Gebaeude) =>
    grund.filter((k) =>
      (g.flurstueck_ids ?? []).some((f) => k.flurstueck_ids.includes(f)),
    );
  /** Frühere Zuordnung zu Baugrundstücken (Altbestand ohne Flurstücke). */
  const altEinheiten = (g: Gebaeude) =>
    g.flurstueck_ids?.length
      ? []
      : grund.filter((k) => g.grundstueck_ids.includes(k.id));
  function flurstueckSetzen(g: Gebaeude, id: string, an: boolean) {
    const ids = g.flurstueck_ids ?? [];
    g.flurstueck_ids = an ? [...ids, id] : ids.filter((x) => x !== id);
    if (g.flurstueck_ids.length) g.grundstueck_ids = [];
    // Eine bewusste Auswahl gilt als geprüft.
    g.flurstuecke_zu_pruefen = undefined;
  }
</script>

<Fachliste
  titel="Gebäude und Geschosse"
  {s}
  bereich="gebaeude"
  {darf}
  zusatzBeschaeftigt={() => planBusy}
>
  {#snippet zusatz(g)}<span
      >{z(g.geometrie?.grundflaeche_m2 ?? null)}{g.geometrie
        ?.grundflaeche_m2 !== undefined
        ? " m² Grundfläche"
        : ""}</span
    ><br /><span class="meta"
      >{g.geschosse.length
        ? `${g.geschosse.length} Geschosse erfasst`
        : "Einzelgeschoss aus Grundfläche"}</span
    >{/snippet}
  {#snippet children(g)}
    <label
      >Gebäudebezeichnung<span class="pflicht" aria-hidden="true">
        *</span
      ><input
        aria-label="Gebäudebezeichnung"
        bind:value={g.bezeichnung}
        required
        maxlength="500"
      /></label
    >
    <p class="meta">
      Erfassen Sie zunächst, was bereits bekannt ist. Flächen und Höhen können
      später ergänzt werden; leere Felder bleiben unbekannt.
    </p>
    <section class="grundlagen-block" aria-label="Gebäudegrundfläche">
      <h3>Gebäudegrundfläche</h3>
      <div class="raster">
        <label
          >Gebäudetyp für GRZ<select
            value={g.geometrie?.typ ?? ""}
            onchange={(e) => {
              (g.geometrie ??= {}).typ = (e.currentTarget.value ||
                undefined) as NonNullable<Gebaeude["geometrie"]>["typ"];
            }}
          >
            <option value="">Noch nicht zugeordnet</option><option
              value="hauptanlage">Hauptgebäude / Wohnhaus</option
            ><option value="garage">Garage</option><option value="nebenanlage14"
              >Nebenanlage (§ 14)</option
            ><option value="unterirdisch">Unterirdische Anlage</option>
          </select></label
        >
        <GrundlagenZahl
          titel="Gebäudegrundfläche, horizontale Projektion (m²)"
          min={0}
          wert={g.geometrie?.grundflaeche_m2}
          readonly={g.planbindungen?.some(
            (b) => b.ziel === "geometrie.grundflaeche_m2",
          )}
          onwert={(v) => {
            (g.geometrie ??= {}).grundflaeche_m2 = v;
          }}
        />
      </div>
      {#if s.id}<PlanFlaeche
          projektId={projekt.id}
          gebaeudeId={s.id}
          {g}
          {aufmasse}
          gespeichert={s.original !== null}
          onplan={(neu) => (s.eingabe = neu)}
          onbusy={(b) => (planBusy = b)}
        />{/if}
      <p class="meta">
        Ohne einzeln erfasste Geschosse wird diese Grundfläche zugleich als
        Fläche eines vorläufigen Einzelgeschosses verwendet. Bei mehreren
        Geschossen bleibt sie der einmalige Gebäudeumgriff für die GRZ.
      </p>
    </section>
    <details class="grundlagen-block">
      <summary>Höhen und Bezugspunkt ergänzen</summary>
      <div class="raster">
        {#each hoehenfelder as [key, titel]}<GrundlagenZahl
            {titel}
            wert={g.geometrie?.[key]}
            readonly={g.planbindungen?.some(
              (b) => b.ziel === `geometrie.${key}`,
            )}
            onwert={(v) => {
              (g.geometrie ??= {})[key] = v;
            }}
          />{/each}
        <label
          >Dachform<select
            value={g.geometrie?.dachform ?? ""}
            onchange={(e) => {
              const v = e.currentTarget.value as Dachform | "";
              if (v) (g.geometrie ??= {}).dachform = v;
              else if (g.geometrie) delete g.geometrie.dachform;
            }}
          >
            <option value="">Nicht erfasst</option
            >{#each DACHFORMEN as [code, name]}<option value={code}
                >{name}</option
              >{/each}
          </select></label
        >
        <GrundlagenZahl
          titel="Dachneigung (Grad)"
          wert={g.geometrie?.dachneigung_grad}
          onwert={(v) => {
            (g.geometrie ??= {}).dachneigung_grad = v;
          }}
        />
      </div>
      {#if vorschau?.dach}<p class="meta">
          Dach: {vorschau.dach.herleitung}. Der Abstandsflächennachweis
          übernimmt Dachhöhe und Neigung mit „Höhen übernehmen“ für Giebel und
          Dachflächen.
        </p>{/if}
      <label
        >Herkunft der Flächen und Höhen<textarea
          maxlength="4000"
          value={g.geometrie?.herkunft ?? ""}
          placeholder="z. B. Vorentwurf, Aufmaß oder Vermessung mit Datum"
          oninput={(e) => {
            (g.geometrie ??= {}).herkunft = e.currentTarget.value;
          }}
        ></textarea></label
      >
      <p class="meta">
        Fundamentsohle und Fußbodenhöhe sind unterschiedliche Angaben. Welches
        Gelände und welche Deckenlage für eine Vollgeschossprüfung maßgebend
        sind, wird im Nachweis anhand der geltenden Vorschrift geprüft.
      </p>
    </details>
    <fieldset>
      <legend>Lage auf Flurstücken</legend>
      {#if flurstueckEntwurf}<p class="hinweisbox" role="status">
          Unter Grundstücksinformationen → Flurstücke und Grundbuch gibt es
          ungespeicherte Änderungen. Neue Flurstücke erscheinen hier erst nach
          dem Speichern.
        </p>{/if}
      <p class="meta">
        Flurstücke wählen, auf denen das Gebäude steht. Bei einem Gebäude über
        mehrere Flurstücke (z. B. mit Vereinigungsbaulast) alle betroffenen
        Flurstücke wählen.
      </p>
      {#each flurstuecke as f (f.id)}<label
          class:nebenflurstueck={(f.rolle ?? "vorhaben") !== "vorhaben"}
          ><input
            type="checkbox"
            checked={(g.flurstueck_ids ?? []).includes(f.id)}
            onchange={(e) => flurstueckSetzen(g, f.id, e.currentTarget.checked)}
          />{flurName(f)}{(f.rolle ?? "vorhaben") !== "vorhaben"
            ? " (außerhalb des Vorhabengrundstücks)"
            : ""}</label
        >{:else}<p class="meta">
          Noch keine Flurstücke erfasst. Unter Grundstücksinformationen →
          Flurstücke und Grundbuch ergänzen; die Zuordnung kann anschließend
          erfolgen.
        </p>{/each}
      {#each (g.flurstueck_ids ?? []).filter((id) => !flurstuecke.some((f) => f.id === id)) as id (id)}<label
          class="veraltet"
          ><input
            type="checkbox"
            checked
            onchange={() => flurstueckSetzen(g, id, false)}
          />Nicht mehr erfasstes Flurstück – Haken entfernen</label
        >{/each}
      {#if !g.flurstueck_ids?.length && g.grundstueck_ids.length}<div
          class="hinweisbox"
          role="status"
        >
          {#if altEinheiten(g).length}Bisher dem Baugrundstück „{altEinheiten(g)
              .map((k) => k.bezeichnung)
              .join("“, „")}“ zugeordnet.
            <button
              type="button"
              class="link"
              onclick={() => {
                g.flurstueck_ids = altEinheiten(g).flatMap(
                  (k) => k.flurstueck_ids,
                );
                g.grundstueck_ids = [];
                g.flurstuecke_zu_pruefen = true;
              }}>Dessen Flurstücke übernehmen</button
            > und auf die tatsächlich bebauten Flurstücke reduzieren.
          {:else}Die frühere Baugrundstückszuordnung besteht nicht mehr. Bitte
            die Flurstücke wählen.{/if}
        </div>{/if}
      {#if g.flurstuecke_zu_pruefen}<div class="hinweisbox" role="status">
          Die Flurstücke wurden aus der früheren Baugrundstückszuordnung
          übernommen. Bitte auf die tatsächlich bebauten Flurstücke reduzieren.
          <button
            type="button"
            class="link"
            onclick={() => {
              g.flurstuecke_zu_pruefen = undefined;
            }}>Zuordnung ist geprüft</button
          >
        </div>{/if}
      {#if grundGeladen && einheiten(g).length === 1}<p class="meta">
          Baugrundstück laut Ermittlung: {einheiten(g)[0].bezeichnung}.
          Grundfläche und Geschosse gehen automatisch in dessen GRZ, GFZ und
          Geschossigkeit ein.
        </p>
      {:else if grundGeladen && einheiten(g).length > 1}<p
          class="hinweisbox"
          role="status"
        >
          Die gewählten Flurstücke gehören derzeit zu mehreren Baugrundstücken.
          Bei einer Vereinigungsbaulast die Flurstücke unter Berechnungen →
          Baugrundstücke gemeinsam zuordnen (abweichende Zuordnung mit
          Grundlage); sonst die Flächen dort je Grundstück getrennt erfassen.
        </p>
      {:else if !g.flurstueck_ids?.length && !g.grundstueck_ids.length && vorhaben.length > 1}<p
          class="meta"
        >
          Ohne Flurstückszuordnung geht das Gebäude nur bei genau einem
          Baugrundstück in die Berechnungen ein.
        </p>{/if}
    </fieldset>
    <h3>Geschosse</h3>
    <button type="button" class="sekundaer" onclick={quellstaende}
      >Vollgeschossnachweise aktualisieren</button
    >
    {#if !g.geschosse.length}<section class="einzelgeschoss">
        <h4>Einzelgeschoss · aus Gebäudegrundfläche</h4>
        <p class="meta">
          Für eine eingeschossige Garage ist kein zusätzliches EG erforderlich.
          Mit geprüfter Einstufung geht das Einzelgeschoss automatisch in GFZ
          und Geschossigkeit ein.
        </p>
        <div class="raster">
          <label
            >Einstufung<select
              aria-label="Einstufung des Einzelgeschosses"
              value={g.geometrie?.einzelgeschoss_vollgeschoss === undefined
                ? ""
                : String(g.geometrie.einzelgeschoss_vollgeschoss)}
              onchange={(e) => {
                const v = e.currentTarget.value;
                (g.geometrie ??= {}).einzelgeschoss_vollgeschoss =
                  v === "" ? undefined : v === "true";
              }}
              ><option value="">Noch zu prüfen</option><option value="true"
                >Vollgeschoss</option
              ><option value="false">Nichtvollgeschoss</option></select
            ></label
          >
          <label
            >Grundlage der Einstufung<input
              value={g.geometrie?.einzelgeschoss_grundlage ?? ""}
              required={g.geometrie?.einzelgeschoss_vollgeschoss !== undefined}
              maxlength="4000"
              placeholder="Anwendbare Vorschrift und Herleitung"
              oninput={(e) => {
                (g.geometrie ??= {}).einzelgeschoss_grundlage =
                  e.currentTarget.value || undefined;
              }}
            /></label
          >
        </div>
        <details>
          <summary>Bekannte Höhen des Einzelgeschosses</summary><GeschossMasse
            einzelgeschoss
            masse={g.geometrie?.einzelgeschoss}
            onmasse={(m) => {
              (g.geometrie ??= {}).einzelgeschoss = m;
            }}
          />
        </details>
      </section>{/if}
    {#each g.geschosse as geschoss, i (geschoss.id)}<fieldset
        data-uebersicht-eintrag={geschoss.id}
      >
        <legend>Geschoss {i + 1}</legend>
        <div class="raster">
          <label
            >Bezeichnung<span class="pflicht" aria-hidden="true"> *</span><input
              aria-label="Bezeichnung"
              bind:value={geschoss.bezeichnung}
              required
            /></label
          ><label
            >Einstufung<select
              aria-label="Einstufung"
              bind:value={geschoss.vollgeschoss}
              disabled={!!geschoss.einstufungsquelle}
              ><option value={null}>Noch zu prüfen</option><option value={true}
                >Vollgeschoss</option
              ><option value={false}>Nichtvollgeschoss</option></select
            ></label
          ><label
            >Grundlage der Einstufung<input
              bind:value={geschoss.grundlage}
              required={geschoss.vollgeschoss !== null}
              placeholder="Anwendbare Vorschrift und Herleitung"
            /></label
          >
        </div>
        <GeschossMasse
          masse={geschoss.geometrie}
          onmasse={(m) => {
            geschoss.geometrie = m;
          }}
          geschosse={g.geschosse}
          id={geschoss.id}
          gebundeneFelder={g.planbindungen
            ?.filter((b) => b.ziel.startsWith(`geschosse.${geschoss.id}.`))
            .map((b) => b.ziel.split(".")[2]) ?? []}
        />
        {#if s.id}<PlanFlaeche
            projektId={projekt.id}
            gebaeudeId={s.id}
            {g}
            {aufmasse}
            geschossId={geschoss.id}
            geschossName={geschoss.bezeichnung || `Geschoss ${i + 1}`}
            gespeichert={geschossGespeichert(geschoss.id)}
            onplan={(neu) => (s.eingabe = neu)}
            onbusy={(b) => (planBusy = b)}
          />{/if}
        <button
          type="button"
          class="sekundaer"
          disabled={!s.id || planBusy || s.laeuft}
          onclick={async () => {
            if (s.hatAenderungen() && !(await s.speichern())) return;
            if (navigation && s.id) {
              navigation.vollgeschossauftrag = {
                gebaeude: s.id,
                geschoss: geschoss.id,
                bezeichnung: geschoss.bezeichnung,
              };
              navigation.rueckweg = {
                bereich: "gebaeude",
                objekt: s.id,
                register: null,
                eintrag: geschoss.id,
              };
              navigation.springen({
                bereich: "vollgeschosse",
                objekt: null,
                register: null,
                eintrag: null,
              });
            }
          }}>Vollgeschossstatus dieses Geschosses prüfen …</button
        >
        <label
          >Aus Vollgeschossnachweis übernehmen<select
            value={geschoss.einstufungsquelle
              ? `${geschoss.einstufungsquelle.nachweis_id}|${geschoss.einstufungsquelle.kennzahl}`
              : ""}
            onchange={(e) => {
              const q = quellen(geschoss.id).find(
                (q) => `${q.nachweis}|${q.kennzahl}` === e.currentTarget.value,
              );
              geschoss.einstufungsquelle = q
                ? { nachweis_id: q.nachweis, kennzahl: q.kennzahl }
                : undefined;
              if (q) geschoss.vollgeschoss = q.wert === 1;
            }}
            ><option value="">Manuelle Einstufung</option
            >{#each quellen(geschoss.id) as q}<option
                value={`${q.nachweis}|${q.kennzahl}`}>{q.titel}</option
              >{/each}</select
          ></label
        >
        {#if geschoss.einstufungsquelle}<p class="meta">
            Die GFZ-/Geschossigkeitsberechnung prüft diesen Quellnachweis bei
            jeder Verwendung auf Aktualität.
          </p>{/if}
        <button
          type="button"
          class="link"
          onclick={() => void geschossEntfernen(g, geschoss.id)}
          >Geschoss entfernen</button
        >
      </fieldset>{/each}
    <button
      type="button"
      class="sekundaer"
      onclick={() =>
        g.geschosse.push({
          id: crypto.randomUUID(),
          bezeichnung: "",
          vollgeschoss: null,
          grundlage: "",
        })}>Geschoss hinzufügen</button
    >
    {#if s.id}<details bind:open={planOffen}>
        <summary>Höhen und alle Verknüpfungen aus PDF-Aufmaß</summary>
        <p class="meta">
          Flächen lassen sich direkt an den Feldern Gebäudegrundfläche und
          Geschossfläche übernehmen. Hier werden zusätzlich Höhen verknüpft und
          alle Verknüpfungen aufgelistet. Gebäude und Geschosse zuerst
          speichern, dann Markierungen im PDF-Aufmaß zuordnen. Absolute Höhen in
          NHN werden auf den Gebäudenullpunkt umgerechnet.
        </p>
        <Planquellen
          projektId={projekt.id}
          objektId={s.id}
          plan={g}
          onplan={(neu) => (s.eingabe = neu as Gebaeude)}
          onbusy={(b) => (planBusy = b)}
        />
      </details>{/if}
    {#if vorschauFehler}<p role="alert" class="fehlertext">
        {vorschauFehler}
      </p>{/if}
    {#if vorschau}<section
        class="grundlagen-auswertung"
        aria-label="Abgeleitete Gebäudegrundlagen"
      >
        <h3>Aus den bekannten Angaben</h3>
        <p>
          Gebäudegrundfläche: <strong
            >{z(vorschau.grundflaeche_m2)}{vorschau.grundflaeche_m2 !== null
              ? " m²"
              : ""}</strong
          >
          · Summe der Geschossflächen:
          <strong
            >{z(
              vorschau.geschossflaechensumme_m2,
            )}{vorschau.geschossflaechensumme_m2 !== null ? " m²" : ""}</strong
          >
        </p>
        <p class="meta">
          Die Geschossflächensumme ist noch keine anrechenbare GFZ-Fläche.
          Hierfür werden Vollgeschosseinstufung und Planungsrecht im
          Berechnungsbereich geprüft.
        </p>
        <div class="tabellen-scroll">
          <table class="ergebnistabelle">
            <thead
              ><tr
                ><th>Ebene</th><th>Fläche (m²)</th><th>OK Decke zu ±0,00 (m)</th
                ><th>Decke über Gelände (m)</th></tr
              ></thead
            ><tbody>
              {#each vorschau.geschosse as s}<tr
                  ><th scope="row"
                    >{s.bezeichnung}{s.implizit ? " (vorläufig)" : ""}</th
                  ><td>{z(s.flaeche_m2)}</td><td>{z(s.decke_oberkante_m)}</td
                  ><td>{z(s.decke_ueber_gelaende_m)}</td></tr
                >{/each}
            </tbody>
          </table>
        </div>
        {#each vorschau.geschosse.filter((s) => s.decke_oberkante_m !== null) as s}<p
            class="meta"
          >
            {s.bezeichnung}: {s.herleitung}.
          </p>{/each}
        {#if vorschau.offen.length}<details>
            <summary>Noch offene Grundlagen ({vorschau.offen.length})</summary>
            <ul>
              {#each vorschau.offen as o}<li>{o}</li>{/each}
            </ul>
          </details>{/if}
      </section>{/if}
    <p class="meta">
      Gebäude und Geschosse besitzen feste Identitäten. Die
      Vollgeschosseinstufung wird mit ihrer fachlichen Grundlage erfasst.
    </p>
  {/snippet}
</Fachliste>

<style>
  .grundlagen-block {
    margin-block: 1.3rem;
    padding-block: 1rem;
    border-block-start: 1px solid var(--border);
  }
  summary {
    cursor: pointer;
    font-weight: 600;
  }
  .einzelgeschoss {
    margin-block: 1rem;
    padding: 1rem;
    border-inline-start: 3px solid var(--accent);
    background: var(--accent-soft);
  }
  .einzelgeschoss h4 {
    margin-top: 0;
  }
  .veraltet {
    color: var(--warning-text);
    font-style: italic;
  }
  .nebenflurstueck {
    color: var(--muted);
  }
  .grundlagen-auswertung {
    margin-block: 1.5rem;
    padding-block: 1rem;
    border-block: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
  }
  .tabellen-scroll {
    overflow-x: auto;
  }
</style>
