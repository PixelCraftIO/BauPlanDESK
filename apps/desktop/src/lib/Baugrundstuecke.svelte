<script lang="ts">
  import { berechnungsvoraussetzung } from "./module";
  import { projektNavigation } from "./projektNavigation.svelte";
  import { entwurfspeicher } from "./entwurfspeicher";
  import Statusanzeige from "./arbeitsplatz/Statusanzeige.svelte";
  import Druckknopf from "./Druckknopf.svelte";
  import { berechnungsplan, berechnungsansaetze } from "./grundstuecksdaten";
  import type { Gebaeude } from "./generated/plattform";
  import type { Datensatz } from "./plattformApi";
  import { plattformApi } from "./plattformApi";
  import { abonnieren } from "./projektEreignisse";
  import { editorAnmelden } from "./editoren";
  import { onMount, untrack, tick } from "svelte";
  import {
    api,
    fehlertext,
    type Project,
    type BaugrundstuecksPlan,
    type BaugrundstueckGruppe,
    type BaugrundstuecksPruefung,
    type BaugrundstueckBerechnung,
    type Einbindungshinweis,
  } from "./api";
  import type { BerechnungsBereich } from "./projektSitzung";
  import { regelwerk } from "./baunvo";
  import ABZUEGE from "../../../../regelwerke/grundstueck-abzuege.json";
  import GrundstueckFormular from "./GrundstueckFormular.svelte";
  import FlaechenFormular from "./FlaechenFormular.svelte";
  let {
    projekt,
    benutzerId,
    darf,
    onprojekt,
    bereich = "grundstueck",
    bezugsflaechenAktuell = false,
  }: {
    projekt: Project;
    benutzerId: string;
    darf: boolean;
    onprojekt: (p: Project) => void;
    bereich?: BerechnungsBereich;
    bezugsflaechenAktuell?: boolean;
  } = $props();
  const navigation = projektNavigation();
  const voraussetzung = $derived(
    bereich === "grundstueck" ? null : berechnungsvoraussetzung(projekt),
  );
  let lokalHinweis = $state("");
  let lokalSignatur = $state("");
  let lokalerEntwurf = $state<BaugrundstuecksPlan | null>(null);
  const entwurfKey = $derived(
    `bauplan.nutzung.entwurf.${benutzerId || "nichtangemeldet"}.${projekt.id}`,
  );
  async function lokalSichern() {
    if (!darf || !benutzerId) return;
    try {
      const signatur = JSON.stringify(eingabe);
      await entwurfspeicher.schreiben(entwurfKey, {
        version: projekt.version,
        plan: JSON.parse(signatur),
      });
      lokalSignatur = signatur;
      lokalHinweis =
        "Entwurf auf diesem Gerät gesichert. Er ist noch kein bestätigter Projekt- oder Nachweisstand.";
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  function initial(): BaugrundstuecksPlan {
    const plan = structuredClone(
      berechnungsplan($state.snapshot(projekt.planungsrecht)),
    );
    const f = projekt.planungsrecht.bau_nvo;
    if (f) {
      for (const b of plan.berechnungen) {
        const a = b.eingabe.aufstellung;
        if (a && a.bau_nvo !== f) {
          a.bau_nvo = f;
          if (!a.nichtvollgeschoss_festsetzung.trim())
            a.nichtvollgeschoss_faktor = regelwerk(f).nichtvollgeschoss_faktor;
        }
      }
    }
    return plan;
  }
  let basisProjekt = $state(
    untrack(() => structuredClone($state.snapshot(projekt))),
  );
  let eingabe = $state(untrack(initial)),
    gespeichert = $state(untrack(initial)),
    basis = $state(untrack(() => projekt.version));
  const lokalGesichert = $derived(
    lokalSignatur !== "" && lokalSignatur === JSON.stringify(eingabe),
  );
  let gruppen = $state<BaugrundstueckGruppe[]>([]),
    gruppenStand = $state(""),
    ergebnis = $state<BaugrundstuecksPruefung | null>(null),
    ergebnisStand = $state(""),
    auswahl = $state(""),
    laeuft = $state(false),
    fehler = $state<string | null>(null),
    hinweis = $state<string | null>(null);
  const mitPlan = $derived(
    !["kein", "ungeklaert"].includes(projekt.planungsrecht.bebauungsplan_art),
  );
  const gruppenFingerprint = $derived(
    JSON.stringify([eingabe.flurstuecke, projekt.version]),
  );
  let gebaeude = $state<Datensatz<Gebaeude>[]>([]);
  // Gebäudeänderungen machen ein angezeigtes Ergebnis ebenfalls veraltet.
  const fingerprint = $derived(
    JSON.stringify([
      eingabe,
      projekt.version,
      gebaeude.map((g) => [g.objekt, g.revision]),
    ]),
  );
  const gruppenAktuell = $derived(gruppenStand === gruppenFingerprint),
    aktuell = $derived(ergebnisStand === fingerprint);
  const quellenGeaendert = $derived(
    JSON.stringify(
      eingabe.flurstuecke.map(
        ({
          id,
          gemarkung,
          flur,
          nummer,
          grundbuchbezirk,
          grundbuchblatt,
          laufende_nummer,
          flaeche_m2,
        }) => ({
          id,
          gemarkung,
          flur,
          nummer,
          grundbuchbezirk,
          grundbuchblatt,
          laufende_nummer,
          flaeche_m2,
        }),
      ),
    ) !==
      JSON.stringify(
        berechnungsplan($state.snapshot(projekt.planungsrecht)).flurstuecke.map(
          ({
            id,
            gemarkung,
            flur,
            nummer,
            grundbuchbezirk,
            grundbuchblatt,
            laufende_nummer,
            flaeche_m2,
          }) => ({
            id,
            gemarkung,
            flur,
            nummer,
            grundbuchbezirk,
            grundbuchblatt,
            laufende_nummer,
            flaeche_m2,
          }),
        ),
      ),
  );
  const geaendert = $derived(
    JSON.stringify(eingabe) !== JSON.stringify(gespeichert),
  );
  const gruppe = $derived(
    gruppen.find((g) => g.schluessel === auswahl) ?? gruppen[0],
  );
  const berechnung = $derived(
    eingabe.berechnungen.find((b) => b.schluessel === gruppe?.schluessel),
  );
  const nachweis = $derived(
    aktuell
      ? ergebnis?.baugrundstuecke.find(
          (g) => g.grundstueck.schluessel === gruppe?.schluessel,
        )?.pruefung
      : null,
  );
  const z = (n: number | null | undefined) =>
    n == null ? "—" : n.toLocaleString("de-DE", { maximumFractionDigits: 3 });
  const nachweisTitel: Record<string, string> = {
    grz: "GRZ I / GRZ",
    grz_gesamt: "GRZ II",
    gfz: "GFZ",
    bmz: "BMZ",
    grundflaeche: "Grundfläche",
    geschossflaeche: "Geschossfläche",
    baufensterflaeche: "Baufensterfläche (Information)",
    baufenster_lage: "Lage im Baufenster (§ 23 BauNVO, m² außerhalb)",
    vollgeschosse: "Vollgeschosse",
  };
  // Orientierungs- und Umgebungswerte zeigen nur die Abweichung; der Core wertet sie nicht als Verstoß.
  const quellenTitel: Record<string, string> = {
    festsetzung: "Festsetzung",
    orientierungswert: "Orientierungswert § 17 BauNVO",
    bau_nvo19: "§ 19 Abs. 4 BauNVO",
    umgebung: "Nähere Umgebung (§ 34 BauGB)",
    keine: "Keine Vorgabe",
  };
  const relevanteNachweise = $derived(
    (nachweis?.nachweise ?? []).filter((n) =>
      bereich === "grz"
        ? ["grz", "grz_gesamt", "grundflaeche", "baufenster_lage"].includes(
            n.art,
          )
        : bereich === "gfz"
          ? ["gfz", "geschossflaeche"].includes(n.art)
          : bereich === "geschosse"
            ? n.art === "vollgeschosse"
            : bereich === "volumen"
              ? n.art === "bmz"
              : false,
    ),
  );
  $effect(() => {
    const p = projekt;
    untrack(() => {
      if (!geaendert) {
        eingabe = initial();
        basisProjekt = structuredClone($state.snapshot(p));
        gespeichert = structuredClone($state.snapshot(eingabe));
        basis = p.version;
      }
    });
  });
  function anlegen(g: BaugrundstueckGruppe): BaugrundstueckBerechnung {
    const f = projekt.planungsrecht.bau_nvo ?? "2021";
    return {
      schluessel: g.schluessel,
      flurstueck_ids: g.flurstueck_ids,
      eigene_festsetzungen: null,
      eingabe: {
        gebiet: projekt.planungsrecht.festgesetzte_werte.baugebiet ?? "WA",
        vollgeschosse: null,
        mass: {
          grundstuecksflaeche_m2: g.bauland_m2,
          grundflaeche_m2: 0,
          grundflaeche_nebenanlagen_m2: 0,
          geschossflaeche_m2: 0,
          baumasse_m3: null,
        },
        aufstellung: {
          bau_nvo: f,
          bauteile: [],
          geschosse: [],
          nichtvollgeschoss_faktor: regelwerk(f).nichtvollgeschoss_faktor,
          nichtvollgeschoss_festsetzung: "",
          grundstueck: {
            basisflaeche_m2: g.eigentumsflaeche_m2,
            nicht_massgebend_m2: g.abzuege_m2,
            abzug_begruendung: "Flurstücksabzüge",
            gemeinschaftsanteile: [],
          },
        },
      },
    };
  }
  async function ermitteln(initialisierung = false) {
    const stand = gruppenFingerprint;
    const eingabestand = fingerprint;
    fehler = null;
    try {
      const g = await api.baugrundstueckeErmitteln(
        projekt.id,
        structuredClone($state.snapshot(eingabe)),
      );
      if (stand !== gruppenFingerprint) return false;
      const nurVorgaben =
        initialisierung && !geaendert && eingabestand === fingerprint;
      gruppen = g;
      gruppenStand = stand;
      if (!g.some((x) => x.schluessel === auswahl))
        auswahl = g[0]?.schluessel ?? "";
      const neu: BaugrundstueckGruppe[] = [];
      for (const gruppe of g)
        if (
          !eingabe.berechnungen.some((b) => b.schluessel === gruppe.schluessel)
        ) {
          eingabe.berechnungen.push(anlegen(gruppe));
          neu.push(gruppe);
        }
      if (nurVorgaben) gespeichert = structuredClone($state.snapshot(eingabe));
      // Eindeutige Nachfolge laut Core (z. B. Flurstück hinzugekommen): Eingaben
      // der bisherigen Berechnung in die neu angelegte übernehmen. Erscheint als
      // ungespeicherte Änderung; Aufteilungen und Zusammenlegungen bleiben manuell.
      const uebernommen = neu.flatMap((gruppe) =>
        gruppe.vorgaenger?.length === 1 &&
        eingabe.berechnungen.some((b) => b.schluessel === gruppe.vorgaenger![0])
          ? [eingabenUebernehmen(gruppe.vorgaenger[0], gruppe.schluessel)]
          : [],
      );
      if (uebernommen.length)
        hinweis = `Die Grundstücksbildung hat sich geändert. Eingaben von ${uebernommen.length} bisherigen Berechnung(en) wurden dem jeweiligen Nachfolger zugeordnet. Bitte prüfen und speichern.`;
      return true;
    } catch (e) {
      fehler = fehlertext(e);
      return false;
    }
  }
  onMount(async () => {
    try {
      const d = (await entwurfspeicher.lesen(entwurfKey)) as {
        version: number;
        plan: BaugrundstuecksPlan;
      } | null;
      if (
        d?.plan &&
        Array.isArray(d.plan.flurstuecke) &&
        Array.isArray(d.plan.berechnungen)
      )
        lokalerEntwurf = d.plan;
    } catch (e) {
      lokalHinweis = fehlertext(e);
    }
    if (eingabe.flurstuecke.length && (await ermitteln(true)))
      await gebaeudeEinbinden();
  });
  let gebaeudeHinweise = $state<Einbindungshinweis[]>([]);
  // Zugeordnete Gebäude übernimmt der Core; die Antwort gilt nur für den
  // Eingabestand, für den sie angefordert wurde.
  async function gebaeudeEinbinden() {
    if (bereich === "grundstueck" || bereich === "volumen" || !gruppenAktuell)
      return;
    const stand = fingerprint;
    try {
      const r = await api.baugrundstueckeGebaeudeEinbinden(
        projekt.id,
        request(),
        gruppen.length>1?gruppe?.schluessel??null:null,
      );
      if (stand !== fingerprint) return;
      for (const b of r.plan.berechnungen) {
        const ziel = eingabe.berechnungen.find(
          (x) => x.schluessel === b.schluessel,
        )?.eingabe.aufstellung;
        if (!ziel || !b.eingabe.aufstellung) continue;
        if (
          JSON.stringify(ziel.bauteile) !==
          JSON.stringify(b.eingabe.aufstellung.bauteile)
        )
          ziel.bauteile = b.eingabe.aufstellung.bauteile;
        if (
          JSON.stringify(ziel.geschosse) !==
          JSON.stringify(b.eingabe.aufstellung.geschosse)
        )
          ziel.geschosse = b.eingabe.aufstellung.geschosse;
      }
      gebaeudeHinweise = r.hinweise;
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  async function neuErmitteln() {
    laeuft = true;
    try {
      await ermitteln();
    } finally {
      laeuft = false;
    }
  }
  function request() {
    const e = structuredClone($state.snapshot(eingabe));
    for (const b of e.berechnungen) {
      if (b.eingabe.aufstellung && projekt.planungsrecht.bau_nvo)
        b.eingabe.aufstellung.bau_nvo = projekt.planungsrecht.bau_nvo;
    }
    return e;
  }
  let nachweisHinweis = $state<string | null>(null),
    nachweisVorgang = "",
    nachweisStand = "";
  let grundkontexte = $state<import("./api").GrundstueckKontext[]>([]);
  // Grundstückseinheiten hängen am gespeicherten Projektstand.
  $effect(() => {
    void projekt.version;
    untrack(() => {
      api
        .grundstueckskontext(projekt.id)
        .then((g) => (grundkontexte = g))
        .catch(() => (grundkontexte = []));
    });
  });
  function gebaeudeLaden() {
    return plattformApi
      .liste<Gebaeude>(projekt.id, "gebaeude")
      .then((g) => {
        gebaeude = g;
        return gebaeudeEinbinden();
      })
      .catch((e) => (fehler = fehlertext(e)));
  }
  $effect(() => {
    void bereich;
    untrack(() => void gebaeudeLaden());
  });
  // Gebäude/Geschosse in einem anderen Bereich dieser Akte geändert.
  $effect(() =>
    abonnieren(projekt.id, ["fachobjekte"], () => void gebaeudeLaden()),
  );
  // Nach geänderter Grundstücksbildung: übernommene Berechnungen (alt → neu),
  // deren Gebäudezuordnungen nach dem Speichern übertragen werden.
  let zuordnungen = $state<[string, string][]>([]);
  let uebernahmeZiel = $state<Record<string, string>>({});
  function eingabenUebernehmen(
    altSchluessel: string,
    zielSchluessel = uebernahmeZiel[altSchluessel],
  ) {
    const alt = eingabe.berechnungen.find(
      (b) => b.schluessel === altSchluessel,
    );
    const ziel = eingabe.berechnungen.find(
      (b) => b.schluessel === zielSchluessel,
    );
    if (!alt || !ziel) return false;
    const a = structuredClone($state.snapshot(alt));
    // Mit Gebäuden verknüpfte Einträge bindet der Core nach der Übertragung der
    // Zuordnung erneut ein; hier nur die eigenen Eingaben übernehmen.
    if (a.eingabe.aufstellung) {
      a.eingabe.aufstellung.bauteile = a.eingabe.aufstellung.bauteile.filter(
        (t) => !t.gebaeude_id,
      );
      a.eingabe.aufstellung.geschosse = a.eingabe.aufstellung.geschosse.filter(
        (g) => !g.gebaeude_id,
      );
      a.eingabe.aufstellung.grundstueck =
        ziel.eingabe.aufstellung?.grundstueck ??
        a.eingabe.aufstellung.grundstueck;
    }
    a.eingabe.mass.grundstuecksflaeche_m2 =
      ziel.eingabe.mass.grundstuecksflaeche_m2;
    ziel.eingabe = a.eingabe;
    ziel.eigene_festsetzungen = a.eigene_festsetzungen;
    eingabe.berechnungen = eingabe.berechnungen.filter(
      (b) => b.schluessel !== altSchluessel,
    );
    zuordnungen = [...zuordnungen, [altSchluessel, ziel.schluessel]];
    // Gebäude folgen ihren Flurstücken; nur Altbestand mit früherer
    // Baugrundstückszuordnung wird nach dem Speichern übertragen.
    hinweis = "Eingaben übernommen. Bitte prüfen und speichern.";
    return true;
  }
  async function zuordnungenUebertragen() {
    let anzahl = 0;
    for (const [alt, neu] of zuordnungen)
      anzahl += await plattformApi.gebaeudezuordnungUebertragen(
        projekt.id,
        alt,
        neu,
      );
    zuordnungen = [];
    if (anzahl) await gebaeudeLaden();
    return anzahl;
  }
  async function nachweisSpeichern() {
    if (voraussetzung) {
      fehler = voraussetzung;
      return;
    }
    // Ein Nachweis soll dem gespeicherten Stand entsprechen, den auch Formulare
    // und XBau verwenden: ungespeicherte Eingaben zuerst speichern und neu berechnen.
    if (geaendert) {
      await speichern();
      if (geaendert || fehler) return;
      await berechnen();
    }
    if (!ergebnis || !aktuell) return;
    laeuft = true;
    try {
      if (nachweisStand !== fingerprint) {
        nachweisVorgang = crypto.randomUUID();
        nachweisStand = fingerprint;
      }
      const fach =
        bereich === "geschosse"
          ? "geschossigkeit"
          : bereich === "volumen"
            ? "bmz"
            : bereich === "gfz"
              ? "gfz"
              : "grz";
      const nachweis = await plattformApi.nachweisErstellen(
        projekt.id,
        nachweisVorgang,
        `${fach.toLocaleUpperCase("de-DE")} – ${projekt.name}`,
        { art: "nutzung", fach, plan: request() },
      );
      nachweisHinweis =
        "Nachweisstand mit Eingaben und Regelpaketen gespeichert.";
      return nachweis;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function berechnen() {
    if (voraussetzung) {
      fehler = voraussetzung;
      return;
    }
    laeuft = true;
    fehler = null;
    try {
      if (!(await ermitteln())) return;
      await gebaeudeEinbinden();
      const stand = fingerprint;
      const r = await api.projektBaugrundstueckePruefen(projekt.id, request(),gruppen.length>1?gruppe?.schluessel??null:null);
      if (stand === fingerprint) {
        ergebnis = r;
        ergebnisStand = stand;
      }
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function speichern() {
    if (voraussetzung) {
      await lokalSichern();
      return;
    }
    laeuft = true;
    fehler = null;
    try {
      if (!(await ermitteln())) return;
      const plan = request();
      await api.projektBaugrundstueckePruefen(projekt.id, plan);
      const p = await api.projektBereichSpeichern(
        structuredClone($state.snapshot(basisProjekt)),
        "grundstuecke",
        {
          id: projekt.id,
          expected_version: basis,
          name: projekt.name,
          bundesland: projekt.bundesland,
          grundstuecksflaeche_m2: projekt.grundstuecksflaeche_m2,
          planungsrecht: {
            ...structuredClone($state.snapshot(projekt.planungsrecht)),
            baugrundstuecksermittlung: berechnungsansaetze(plan),
          },
        },
      );
      eingabe = structuredClone(
        berechnungsplan($state.snapshot(p.planungsrecht)),
      );
      basisProjekt = structuredClone($state.snapshot(p));
      gespeichert = structuredClone($state.snapshot(eingabe));
      basis = p.version;
      onprojekt(p);
      try {
        await entwurfspeicher.loeschen(entwurfKey);
      } catch (e) {
        lokalHinweis = `Projekt gespeichert; lokale Sicherung konnte nicht entfernt werden: ${fehlertext(e)}`;
      }
      await tick();
      await ermitteln();
      const uebertragen = await zuordnungenUebertragen();
      hinweis = uebertragen
        ? `Baugrundstücke und Berechnungseingaben gespeichert; ${uebertragen} Gebäudezuordnung(en) übertragen.`
        : "Baugrundstücke und Berechnungseingaben gespeichert.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function einheitSpeichern(){
    if(!berechnung||!darf||laeuft||!bezugsflaechenAktuell)return;
    const schluessel=berechnung.schluessel,entwurf=request();laeuft=true;fehler=null;
    try{
      const p=await api.baugrundstueckEinzelnSpeichern(projekt.id,basis,structuredClone($state.snapshot(berechnung)));
      const neu=berechnungsplan(p.planungsrecht);
      eingabe={...entwurf,berechnungen:entwurf.berechnungen.map(b=>b.schluessel===schluessel?structuredClone(neu.berechnungen.find(n=>n.schluessel===schluessel)!):b)};
      gespeichert=structuredClone(neu);basisProjekt=structuredClone(p);basis=p.version;ergebnis=null;onprojekt(p);
      hinweis="Ausgewähltes Baugrundstück gespeichert. Andere Einheiten wurden nicht neu geprüft; offene Entwürfe bleiben erhalten.";
    }catch(e){fehler=fehlertext(e)}finally{laeuft=false}
  }
  export function hatAenderungen() {
    return geaendert && !(voraussetzung && lokalGesichert);
  }
  export function istBeschaeftigt() {
    return laeuft;
  }
  export async function speichernBeimSchliessen() {
    if (laeuft) return false;
    if (geaendert) await speichern();
    return !hatAenderungen();
  }
  editorAnmelden("grundstueck", {
    hatAenderungen,
    istBeschaeftigt,
    speichern: speichernBeimSchliessen,
  });
</script>

{#if voraussetzung}<aside
    class="hinweisbox"
    aria-label="Voraussetzungen der Berechnung"
  >
    <strong>Berechnung gesperrt · Erfassung als Entwurf möglich</strong>
    <p>{voraussetzung}</p>
    <p>
      Alle Flächenansätze bleiben vorläufig. Keine automatische Bestätigung
      einer Rechtsfassung.
    </p>
    <button
      type="button"
      class="link"
      onclick={() =>
        navigation?.springen({
          bereich: "rechtsgrundlagen",
          objekt: null,
          register: null,
          eintrag: null,
        })}>Rechtsgrundlagen klären →</button
    >
  </aside>{/if}
{#if lokalerEntwurf}<aside class="hinweisbox">
    <p>
      Ein vorbereitender lokaler Entwurf ist vorhanden. Bei geänderten Quellen
      die Zuordnung neu prüfen.
    </p>
    <button
      type="button"
      disabled={!darf || laeuft}
      onclick={async () => {
        eingabe = JSON.parse(JSON.stringify(lokalerEntwurf));
        lokalerEntwurf = null;
        ergebnis = null;
        await neuErmitteln();
      }}>Lokalen Entwurf wiederherstellen</button
    ><button
      type="button"
      class="link"
      onclick={async () => {
        try {
          await entwurfspeicher.loeschen(entwurfKey);
          lokalerEntwurf = null;
        } catch (e) {
          fehler = fehlertext(e);
        }
      }}>Lokalen Entwurf entfernen</button
    >
  </aside>{/if}
{#if lokalHinweis}<p role="status">{lokalHinweis}</p>{/if}
{#if gruppen.length>1&&bereich!=="grundstueck"&&bereich!=="volumen"}<p class="hinweisbox">„Berechnen“ prüft die ausgewählte Grundstückseinheit unabhängig von Planquellen anderer Einheiten. Die vollständige Prüfung bleibt für die gemeinsame Ausgabe erforderlich. <button type="button" disabled={!darf||laeuft||!bezugsflaechenAktuell} onclick={()=>void einheitSpeichern()}>Nur ausgewähltes Baugrundstück speichern</button></p>{/if}
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}{#if hinweis}<p
    role="status"
  >
    {hinweis}
  </p>{/if}
{#if bereich !== "grundstueck" && !bezugsflaechenAktuell && eingabe.flurstuecke.length}<p
    class="hinweisbox"
  >
    Vor einem Nachweisstand die Baugrundstücksermittlung prüfen und speichern.
    Die Berechnung ist bis dahin eine Vorschau.
  </p>{/if}
{#if quellenGeaendert}<p class="hinweisbox" role="status">
    Die gespeicherten Flurstücksdaten wurden geändert. Der Berechnungsentwurf
    bleibt erhalten. Für eine neue Berechnung bitte die aktuellen Daten laden.
  </p>
  <button
    class="sekundaer"
    disabled={laeuft}
    onclick={async () => {
      eingabe = initial();
      gespeichert = initial();
      basisProjekt = structuredClone($state.snapshot(projekt));
      basis = projekt.version;
      ergebnis = null;
      await neuErmitteln();
    }}>Berechnungsentwurf verwerfen und aktuelle Daten laden</button
  >{/if}
{#if bereich === "grundstueck"}
  <p class="meta">
    Die gespeicherten Flurstücke sind die unveränderte Datengrundlage. Hier
    werden Zuordnung, Abzüge und Zuschläge für die Baugrundstücke ermittelt.
  </p>
  {#if !bezugsflaechenAktuell && eingabe.flurstuecke.length}<p
      class="hinweisbox"
    >
      Es liegen noch keine bestätigten aktuellen Bezugsflächen vor. Die
      Ermittlung prüfen und anschließend „Eingaben speichern“ wählen.
    </p>{/if}
  <h3>Datengrundlage: gespeicherte Flurstücke</h3>
  <div class="tabellen-scroll">
    <table class="ergebnistabelle">
      <thead
        ><tr
          ><th>Gemarkung / Flur</th><th>Flurstück</th><th>Flurstücksfläche</th
          ><th>Grundbuchzuordnung</th></tr
        ></thead
      ><tbody>
        {#each eingabe.flurstuecke as f (f.id)}<tr
            ><td>{f.gemarkung} / {f.flur || "—"}</td><td>{f.nummer}</td><td
              >{z(f.flaeche_m2)} m²</td
            ><td
              >{f.grundbuchbezirk && f.grundbuchblatt && f.laufende_nummer
                ? `${f.grundbuchbezirk} · Blatt ${f.grundbuchblatt} · lfd. Nr. ${f.laufende_nummer}`
                : "Optional – Flurstück wird einzeln angesetzt"}</td
            ></tr
          >{/each}
      </tbody>
    </table>
  </div>
  <p class="meta">
    Kataster- und Grundbuchangaben werden unter Grundstücksinformationen →
    Flurstücke und Grundbuch gepflegt. Ohne vollständige Grundbuchangaben wird
    jedes Flurstück einzeln berechnet. Für ein gemeinsames Baugrundstück können
    Sie vollständige Grundbuchangaben ergänzen oder den Flurstücken dieselbe
    besondere Zuordnungskennung mit Grundlage geben.
  </p>
  <fieldset disabled={!darf || laeuft}>
    <legend>1 · Flächenabzüge und Zuordnungen</legend>
    {#each eingabe.flurstuecke as f (f.id)}<details>
        <summary
          >{f.gemarkung || "Gemarkung"} · {f.nummer || "Flurstück"} · {f.abzuege
            .length} Abzüge</summary
        >
        <table class="ergebnistabelle">
          <thead
            ><tr><th>Abzugsart</th><th>m²</th><th>Grundlage</th><th></th></tr
            ></thead
          ><tbody
            >{#each f.abzuege as a, i}<tr
                ><td
                  ><select aria-label="Abzugsart" bind:value={a.art}
                    >{#each Object.entries(ABZUEGE).filter(([, r]) => mitPlan || !r.bebauungsplan_erforderlich) as [code, r]}<option
                        value={code}>{r.bezeichnung}</option
                      >{/each}</select
                  ></td
                ><td
                  ><input
                    aria-label="Abzugsfläche"
                    type="number"
                    min="0"
                    step="any"
                    bind:value={a.flaeche_m2}
                  /></td
                ><td
                  ><input
                    aria-label="Abzugsgrundlage"
                    bind:value={a.grundlage}
                  /></td
                ><td
                  ><button class="link" onclick={() => f.abzuege.splice(i, 1)}
                    >Entfernen</button
                  ></td
                ></tr
              >{/each}</tbody
          >
        </table>
        <button
          class="sekundaer"
          onclick={() =>
            f.abzuege.push({
              art: "sonstiger_abzug",
              flaeche_m2: 0,
              grundlage: "",
            })}>Abzug hinzufügen</button
        >
        <details>
          <summary>Abweichende rechtliche Grundstückszuordnung</summary
          >{#if f.abweichende_zuordnung}<div class="raster">
              <label
                >Zuordnungskennung<input
                  bind:value={f.abweichende_zuordnung.kennung}
                /></label
              ><label
                >Rechtliche Grundlage<input
                  bind:value={f.abweichende_zuordnung.grundlage}
                /></label
              >
            </div>
            <button
              class="link"
              onclick={() => (f.abweichende_zuordnung = null)}
              >Besondere Zuordnung entfernen</button
            >{:else}<button
              class="sekundaer"
              onclick={() =>
                (f.abweichende_zuordnung = { kennung: "", grundlage: "" })}
              >Besondere Zuordnung erfassen</button
            >{/if}
        </details>
      </details>{/each}
  </fieldset>
  <h3>2 · Baugrundstücksermittlung</h3>
  <button
    class="sekundaer"
    disabled={laeuft || !eingabe.flurstuecke.length}
    onclick={neuErmitteln}>Baugrundstücke ermitteln</button
  >
  {#if gruppen.length}<div class="tabellen-scroll">
      <table class="ergebnistabelle">
        <thead
          ><tr
            ><th>Baugrundstück</th><th>Flurstücke</th><th>Eigentumsfläche</th
            ><th>Abzüge</th><th>Bauland</th></tr
          ></thead
        ><tbody
          >{#each gruppen as g}<tr
              ><td>{g.bezeichnung}</td><td>{g.flurstueck_ids.length}</td><td
                >{z(g.eigentumsflaeche_m2)} m²</td
              ><td>− {z(g.abzuege_m2)} m²</td><td
                ><strong>{z(g.bauland_m2)} m²</strong></td
              ></tr
            >{/each}</tbody
        >
      </table>
    </div>{/if}
{/if}
{#if gruppen.length && !gruppenAktuell}<p class="hinweisbox">
    Flurstücke oder Grundlagen geändert. Die angezeigte Ermittlung ist veraltet;
    bitte neu ermitteln.
    <!-- Auch auf GRZ/GFZ-Seiten erreichbar; dort ist die Eingabe bis dahin gesperrt. -->
    <button
      type="button"
      class="sekundaer"
      disabled={laeuft}
      onclick={async () => {
        await neuErmitteln();
        await gebaeudeEinbinden();
      }}>Neu ermitteln</button
    >
  </p>{/if}
{#if !eingabe.flurstuecke.length}<p class="hinweisbox">
    GRZ, GFZ und Geschossigkeit werden je Baugrundstück aus den Flurstücken
    berechnet. Zuerst unter Grundstücksinformationen → Flurstücke und Grundbuch
    die Flurstücke erfassen und speichern, danach hier die
    Baugrundstücksermittlung speichern.
    {#if projekt.grundstuecksflaeche_m2}Die bisher erfasste Projektfläche dient
      nur als vorläufige Angabe. Gebäude, die ihr zugeordnet sind, werden bei
      genau einem Baugrundstück automatisch übernommen; bei mehreren unter
      Gebäude/Geschosse neu zuordnen.{/if}
  </p>{/if}
{#if gruppe && berechnung?.eingabe.aufstellung}
  <label
    >Baugrundstück<select aria-label="Baugrundstück" bind:value={auswahl}
      >{#each gruppen as g}<option value={g.schluessel}>{g.bezeichnung}</option
        >{/each}</select
    ></label
  >
  <fieldset disabled={!darf || laeuft || !gruppenAktuell}>
    {#if bereich === "grundstueck"}<GrundstueckFormular
        aufstellung={berechnung.eingabe.aufstellung}
        basisFallback={gruppe.eigentumsflaeche_m2}
        ergebnis={nachweis?.aufstellung?.grundstueck ?? null}
        {mitPlan}
        flurstuecksBasis={{
          basis: gruppe.eigentumsflaeche_m2,
          abzug: gruppe.abzuege_m2,
        }}
      />
    {:else if bereich === "volumen"}<label
        >Baumasse / Bauvolumen (m³)<input
          type="number"
          min="0"
          step="any"
          bind:value={berechnung.eingabe.mass.baumasse_m3}
        /></label
      >
    {:else}{#each gebaeudeHinweise.filter((h) => h.schluessel === gruppe.schluessel) as h}<p
          class="hinweisbox"
          role="status"
        >
          {h.text}
        </p>{/each}<FlaechenFormular
        {gebaeude}
        regelOffen={!!voraussetzung}
        einzigesGrundstueck={gruppen.length === 1}
        flurstueckIds={gruppe.flurstueck_ids}
        projektId={projekt.id}
        grundstueckId={grundkontexte.find(
          (g) => g.schluessel === gruppe.schluessel,
        )?.id}
        bereich={bereich === "grz" ? "grz" : "gfz"}
        aufstellung={berechnung.eingabe.aufstellung}
        festsetzungen={projekt.planungsrecht.festgesetzte_werte}
        ergebnis={nachweis?.aufstellung ?? null}
      />{/if}
  </fieldset>
{/if}
{#if eingabe.berechnungen.some((b) => !gruppen.some((g) => g.schluessel === b.schluessel)) && gruppenAktuell}<p
    class="hinweisbox"
  >
    Die Grundstückszuordnung hat sich geändert. Verwaiste Berechnungen müssen
    vor dem Speichern geprüft werden: entweder einem neuen Baugrundstück
    ausdrücklich zuordnen oder entfernen.
  </p>
  {#each eingabe.berechnungen.filter((b) => !gruppen.some((g) => g.schluessel === b.schluessel)) as alt (alt.schluessel)}<fieldset
      disabled={!darf || laeuft}
    >
      <legend>Bisherige Berechnung</legend>
      <p class="meta">
        {alt.eingabe.aufstellung?.bauteile.filter((t) => !t.gebaeude_id)
          .length ?? 0} eigene Bauteile,
        {alt.eingabe.aufstellung?.geschosse.filter((g) => !g.gebaeude_id)
          .length ?? 0} eigene Geschosse. Mit Gebäuden verknüpfte Flächen werden
        nach dem Speichern über die übertragene Gebäudezuordnung neu eingebunden.
      </p>
      <label
        >Übernehmen in<select
          aria-label="Übernehmen in"
          bind:value={uebernahmeZiel[alt.schluessel]}
          ><option value={undefined}>Baugrundstück wählen</option
          >{#each gruppen as g}<option value={g.schluessel}
              >{g.bezeichnung}</option
            >{/each}</select
        ></label
      >
      <button
        class="sekundaer"
        disabled={!uebernahmeZiel[alt.schluessel]}
        onclick={() => eingabenUebernehmen(alt.schluessel)}
        >Eingaben übernehmen (ersetzt die Eingaben des Ziels)</button
      >
    </fieldset>{/each}
  <button
    class="sekundaer"
    disabled={!darf || laeuft}
    onclick={() => {
      eingabe.berechnungen = eingabe.berechnungen.filter((b) =>
        gruppen.some((g) => g.schluessel === b.schluessel),
      );
    }}>Nicht zugeordnete Berechnungen entfernen</button
  >{/if}
{#each nachweis?.hinweise ?? [] as h}<p class="hinweisbox" role="status">
    {h}
  </p>{/each}
{#if relevanteNachweise.length}<div class="tabellen-scroll">
    <table class="ergebnistabelle berechnungswerte">
      <thead
        ><tr
          ><th>Nachweis</th><th>Ermittelt</th><th>Zulässig / Vorgabe</th><th
            >Grundlage</th
          ><th>Abweichung</th></tr
        ></thead
      ><tbody
        >{#each relevanteNachweise as n}<tr
            ><td
              >{nachweisTitel[n.art]}{#if n.gegenstand}<span class="meta">
                  · {n.gegenstand}</span
                >{/if}</td
            ><td>{z(n.ist)}</td><td>{z(n.soll)}</td><td
              >{quellenTitel[n.quelle]}</td
            ><td class:verstoss={n.eingehalten === false}
              >{n.art === "baufenster_lage" && n.ist == null
                ? "Nicht geprüft"
                : z(n.abweichung ?? n.ueberschreitung)}<br /><span
                >{n.eingehalten === null
                  ? "Nicht abschließend prüfbar"
                  : n.eingehalten
                    ? "✓ Vorgabe eingehalten"
                    : "! Abweichung prüfen"}</span
              ></td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>{/if}
{#if ergebnis}<Statusanzeige status={aktuell ? "berechnet" : "veraltet"} />{/if}
{#if ergebnis && !aktuell}<p class="hinweisbox" role="status">
    Eingaben geändert. Ergebnisse bitte neu berechnen.
  </p>{/if}
<div class="knoepfe links">
  <button
    disabled={!!voraussetzung || laeuft || !eingabe.flurstuecke.length}
    title={voraussetzung ?? "Berechnen und prüfen"}
    onclick={berechnen}
    >{bereich === "grundstueck"
      ? "Bezugsflächen berechnen"
      : "Berechnen"}</button
  ><button
    class="sekundaer"
    disabled={!darf || laeuft || (!geaendert && bezugsflaechenAktuell)}
    onclick={speichern}
    >{voraussetzung
      ? "Entwurf auf diesem Gerät sichern"
      : "Eingaben speichern"}</button
  >{#if geaendert}<span class="meta"
      >{voraussetzung && lokalGesichert
        ? "Entwurf lokal gesichert – noch kein bestätigter Projektstand"
        : "Ungespeicherte Eingaben"}</span
    >{/if}
</div>

{#if ergebnis}<button
    disabled={!!voraussetzung ||
      !darf ||
      !aktuell ||
      laeuft ||
      !bezugsflaechenAktuell}
    onclick={nachweisSpeichern}
    >{geaendert
      ? "Eingaben speichern und Nachweisstand archivieren"
      : "Nachweisstand speichern"}</button
  ><Druckknopf
    projektId={projekt.id}
    disabled={!!voraussetzung ||
      !darf ||
      !aktuell ||
      laeuft ||
      !bezugsflaechenAktuell}
    text="Nachweis erstellen …"
    vorbereiten={async () => {
      const n = await nachweisSpeichern();
      return n ? { art: "nachweis", id: n.id } : null;
    }}
  />{/if}
{#if nachweisHinweis}<p role="status">{nachweisHinweis}</p>{/if}

<style>
  table {
    margin: 1rem 0;
  }
  details {
    padding: 0.6rem 0;
  }
  fieldset {
    margin: 1rem 0;
  }
  th {
    text-align: left;
  }
</style>
