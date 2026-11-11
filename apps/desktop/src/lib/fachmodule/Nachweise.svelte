<script lang="ts">
  import Statusanzeige from "../arbeitsplatz/Statusanzeige.svelte";
  import Statushilfe from "../arbeitsplatz/Statushilfe.svelte";
  import { nachweisZiel } from "../arbeitsplatz/arbeitscheck";
  import { ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  import Druckknopf from "../Druckknopf.svelte";
  import { projektNavigation } from "../projektNavigation.svelte";
  import { onDestroy, tick, untrack } from "svelte";
  import type { BaugrundstuecksPruefung, Project } from "../api";
  import { fehlertext } from "../api";
  import type { Erneuerung, NachweisAnsicht, JsonValue } from "../generated/plattform";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { plattformApi } from "../plattformApi";
  let {
    projekt,
    darf,
    darfFreigeben = false,
  }: {
    projekt: Project;
    benutzerId: string;
    darf: boolean;
    darfFreigeben?: boolean;
  } = $props();
  let liste = $state<NachweisAnsicht[]>([]),
    fehler = $state<string | null>(null),
    meldung = $state<string | null>(null),
    laedt = $state(false),
    aktion = $state(false);
  const laeuft = $derived(laedt || aktion);
  // Frühere Stände sind Archiv: Sie zählen nicht als Handlungsbedarf und
  // bleiben eingeklappt, bis jemand sie sehen will.
  let fruehereZeigen = $state(false);
  let erneuert = $state<Erneuerung[] | null>(null);
  const massgebend = $derived(liste.filter((n) => !n.ersetzt_durch));
  const fruehere = $derived(liste.length - massgebend.length);
  const sichtbar = $derived(fruehereZeigen ? liste : massgebend);
  const veraltet = $derived(
    massgebend.filter((n) => !n.aktuell && !n.prueffehler).length,
  );
  const freigebbar = $derived(
    massgebend.filter((n) => n.aktuell && n.stand.vollstaendig && !n.freigabe),
  );
  async function alleErneuern() {
    aktion = true;
    fehler = null;
    meldung = null;
    try {
      erneuert = await plattformApi.nachweiseErneuern(projekt.id);
      await laden();
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      aktion = false;
    }
  }
  async function alleFreigeben() {
    const anzahl = freigebbar.length;
    if (
      !(await confirm(
        `${anzahl} aktuelle, vollständige Nachweisstände fachlich freigeben? Die Freigabe bestätigt, dass das Büro die Ergebnisse geprüft hat.`,
        { title: "Nachweise freigeben", kind: "warning" },
      ))
    )
      return;
    aktion = true;
    fehler = null;
    try {
      for (const n of freigebbar)
        await plattformApi.freigeben(
          projekt.id,
          n.stand.id,
          "Fachliche Prüfung durch Projektleitung",
        );
      meldung = `${anzahl} Nachweisstände freigegeben.`;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      await laden();
      aktion = false;
    }
  }
  const standZu = (id: string) => liste.find((n) => n.stand.id === id);
  let ladeNr = 0;
  onDestroy(()=>ladeNr++);
  async function laden() {
    const nr = ++ladeNr, id = projekt.id;
    laedt = true;
    fehler = null;
    try {
      let neu:NachweisAnsicht[];
      try { neu=await plattformApi.nachweise(id); }
      catch(e){
        if(!fehlertext(e).includes("Quellen während der Nachweisprüfung geändert"))throw e;
        if(nr!==ladeNr||id!==projekt.id)return;
        // Ausschließlich die lesende Prüfung einmal wiederholen. Keine
        // Wiederholung von Speicherung, Freigabe oder fachlichen Fehlern.
        neu=await plattformApi.nachweise(id);
      }
      if (nr === ladeNr && id === projekt.id) liste = neu;
    } catch (e) {
      if (nr === ladeNr) fehler = fehlertext(e);
    } finally {
      if (nr === ladeNr) laedt = false;
    }
  }
  // Änderungen an Projektgrundlagen melden keine Fachobjekt-Ereignisse.
  let beobachteteVersion = untrack(()=>projekt.version);
  $effect(()=>{
    const version = projekt.version;
    if (version !== beobachteteVersion) {
      beobachteteVersion = version;
      untrack(vormerken);
    }
  });
  const navigation = projektNavigation();
  const vormerken = ansichtsAbfrage(() => projekt.id, ["fachobjekte", "dokumente", "nachweise"], laden);
  let letzterSprung = 0;
  $effect(() => {
    const a = navigation?.auftrag;
    if (
      !a ||
      a.nummer === letzterSprung ||
      a.ziel.bereich !== "nachweise" ||
      !a.ziel.eintrag
    )
      return;
    letzterSprung = a.nummer;
    void laden()
      .then(() => tick())
      .then(() =>
        document
          .querySelector(
            `[data-projekt-id="${projekt.id}"] [data-nachweis="${a.ziel.eintrag}"]`,
          )
          ?.scrollIntoView({ block: "center" }),
      );
  });

  async function freigeben(n: NachweisAnsicht) {
    aktion = true;
    try {
      await plattformApi.freigeben(
        projekt.id,
        n.stand.id,
        "Fachliche Prüfung durch Projektleitung",
      );
      await laden();
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      aktion = false;
    }
  }
  async function wiederholen(n: NachweisAnsicht) {
    aktion = true;
    try {
      const neu = await plattformApi.wiederholen(projekt.id, n.stand.id);
      meldung =
        JSON.stringify(neu) === JSON.stringify(n.stand.ergebnis)
          ? "Der archivierte Rechenstand wurde mit demselben Ergebnis wiederholt."
          : "Die Wiederholung weicht vom archivierten Ergebnis ab. Bitte fachlich prüfen.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      aktion = false;
    }
  }
  function nutzung(wert: JsonValue): BaugrundstuecksPruefung | null {
    return wert &&
      typeof wert === "object" &&
      !Array.isArray(wert) &&
      Array.isArray(wert.baugrundstuecke)
      ? (wert as unknown as BaugrundstuecksPruefung)
      : null;
  }
  function zahl(n: number | null | undefined) {
    return n === null || n === undefined
      ? "—"
      : n.toLocaleString("de-DE", { maximumFractionDigits: 3 });
  }
  function ergebniszeilen(wert: JsonValue): { name: string; wert: string }[] {
    if (!wert || Array.isArray(wert) || typeof wert !== "object") return [];
    if (typeof wert.rechenstand === "string" && Array.isArray(wert.waende))
      return wert.waende
        .filter((w) => w && typeof w === "object" && !Array.isArray(w))
        .map((w) => {
          const a = w as Record<string, JsonValue>;
          return {
            name: String(a.bezeichnung ?? "Wand"),
            wert: `H ${String(a.h_m ?? "—")} m · Tiefe ${String(a.tiefe_m ?? "—")} m · ${String(a.status ?? "offen")}`,
          };
        });
    if (Array.isArray(wert.kennzahlen))
      return wert.kennzahlen
        .filter((k) => k && typeof k === "object" && !Array.isArray(k))
        .map((k) => {
          const a = k as Record<string, JsonValue>;
          return {
            name: String(a.bezeichnung ?? "Kennzahl"),
            wert:
              a.wert === null
                ? "Nicht ermittelt"
                : `${Number(a.wert).toLocaleString("de-DE", { maximumFractionDigits: 4 })} ${{ m2: "m²", m3: "m³", grad: "°", prozent: "%", l_pro_s: "l/s", anzahl: "Stück" }[String(a.einheit)] ?? String(a.einheit)}`,
          };
        });
    const ergebnis = wert.ergebnis;
    if (
      ergebnis &&
      !Array.isArray(ergebnis) &&
      typeof ergebnis === "object" &&
      typeof ergebnis.tiefe_m === "number"
    )
      return [
        {
          name: "Abstandsflächentiefe",
          wert: ergebnis.tiefe_m.toLocaleString("de-DE") + " m",
        },
      ];
    return [];
  }
</script>

<h2>Nachweisstände und Freigaben</h2>
<p class="meta">
  Gespeicherte Eingaben, Berechnungen und Regeln bleiben als eigener Stand
  erhalten. Neue Stände werden in der jeweiligen Berechnung angelegt.
</p>
<div class="sammelaktionen">
  <button class="sekundaer" disabled={laeuft} onclick={laden}
    >Aktualität prüfen</button
  >
  {#if veraltet}<button
      type="button"
      disabled={laeuft || !darf}
      data-aktion="alle-erneuern"
      onclick={alleErneuern}
      >{veraltet === 1
        ? "Veralteten Nachweis erneuern"
        : `Alle ${veraltet} veralteten Nachweise erneuern`}</button
    >{/if}
  {#if freigebbar.length > 1 && darfFreigeben}<button
      type="button"
      class="sekundaer"
      disabled={laeuft}
      data-aktion="alle-freigeben"
      onclick={alleFreigeben}
      >Alle {freigebbar.length} vollständigen Stände freigeben</button
    >{/if}
  {#if fruehere}<label class="fruehere"
      ><input type="checkbox" bind:checked={fruehereZeigen} />
      {fruehere === 1 ? "1 früheren Stand" : `${fruehere} frühere Stände`} zeigen</label
    >{/if}
</div>
{#if veraltet}<p class="meta">
    Erneuert wird, was sich ohne neue fachliche Entscheidung neu berechnen
    lässt, Quellen zuerst. Wo sich Werte geändert haben, bleibt der Nachweis
    stehen und Sie erhalten den Grund mit einem Sprung zur Berechnung.
  </p>{/if}
{#if erneuert}{@const ok = erneuert.filter((e) => e.neu)}{@const hand =
    erneuert.filter((e) => !e.neu)}
  <section class="hinweisbox" data-erneuerung aria-live="polite">
    <strong
      >{ok.length
        ? `${ok.length} ${ok.length === 1 ? "Nachweis" : "Nachweise"} erneuert`
        : "Kein Nachweis ließ sich automatisch erneuern"}{hand.length
        ? ` · ${hand.length} ${hand.length === 1 ? "braucht" : "brauchen"} Ihre Prüfung`
        : ""}</strong
    >
    {#if ok.length}<p class="meta">
        Erneuert: {ok.map((e) => e.bezeichnung).join(" · ")}. Die fachliche
        Freigabe der neuen Stände steht noch aus.
      </p>{/if}
    {#if hand.length}<ul>
        {#each hand as e (e.alt)}{@const n = standZu(e.alt)}<li>
            <strong>{e.bezeichnung}</strong>: {e.grund}
            {#if n}<button
                type="button"
                class="link"
                onclick={() => navigation?.springen(nachweisZiel(n.stand.eingabe))}
                >Berechnung öffnen</button
              >{/if}
          </li>{/each}
      </ul>{/if}
    <button type="button" class="link" onclick={() => (erneuert = null)}
      >Ausblenden</button
    >
  </section>{/if}
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}{#if meldung}<p
    role="status"
  >
    {meldung}
  </p>{/if}
<Statushilfe/>
<div class="tabellen-scroll"><table class="ergebnistabelle">
  <thead
    ><tr
      ><th>Nachweis</th><th>Erstellt</th><th>Vollständigkeit</th><th
        >Aktualität</th
      ><th>Freigabe</th><th></th></tr
    ></thead
  ><tbody
    >{#each sichtbar as n (n.stand.id)}<tr data-nachweis={n.stand.id} class:frueher={!!n.ersetzt_durch}
        ><td>{n.stand.bezeichnung}</td><td
          >{new Date(n.stand.erstellt_am).toLocaleString("de-DE")}</td
        ><td
          >{n.stand.vollstaendig ? "Vollständig" : "Eingaben unvollständig"}</td
        ><td>{#if n.ersetzt_durch}<Statusanzeige status="archiviert"/><span class="meta"> · Früherer Stand, durch einen neueren ersetzt</span>{:else}<Statusanzeige status={n.prueffehler ? "offen" : n.aktuell ? "archiviert" : "veraltet"}/><span class="meta">{n.prueffehler ? " · Aktualität technisch nicht prüfbar" : n.aktuell ? " · Grundlagen aktuell" : " · Neu berechnen"}</span>{/if}{#if n.prueffehler}<p class="fehlertext" role="alert">{n.prueffehler}</p>{/if}</td
        ><td
          >{n.freigabe
            ? n.aktuell
              ? "Freigegeben"
              : "Freigabe des früheren Stands"
            : "Offen"}</td
        ><td
          ><button type="button" class="link" onclick={()=>navigation?.springen(nachweisZiel(n.stand.eingabe))}>{n.aktuell || n.ersetzt_durch ? "Berechnung öffnen" : "Grundlagen prüfen / neu berechnen"}</button>{#if !n.freigabe && !n.ersetzt_durch}<button
              class="link"
              disabled={!darfFreigeben ||
                !n.aktuell ||
                !n.stand.vollstaendig ||
                laeuft}
              onclick={() => freigeben(n)}>Freigeben</button
            >{/if}<button
            class="link"
            disabled={laeuft}
            onclick={() => wiederholen(n)}>Archivstand wiederholen</button
          ><Druckknopf
            projektId={projekt.id}
            quelle={{ art: "nachweis", id: n.stand.id }}
          /></td
        ></tr
      ><tr
        ><td colspan="6"
          ><details>
            <summary>Grundlagen und Regeln ansehen</summary
            >{#each ergebniszeilen(n.stand.ergebnis) as z}<p>
                {z.name}: <strong>{z.wert}</strong>
              </p>{/each}
            {#if nutzung(n.stand.ergebnis)}{#each nutzung(n.stand.ergebnis)!.baugrundstuecke as g}<h4
                >
                  {g.grundstueck.bezeichnung}
                </h4>
                <p>
                  GRZ {zahl(g.pruefung.kennzahlen.grz)} · GFZ {zahl(
                    g.pruefung.kennzahlen.gfz,
                  )} · BMZ {zahl(g.pruefung.kennzahlen.bmz)}
                </p>
                <table class="ergebnistabelle">
                  <thead
                    ><tr
                      ><th>Prüfung</th><th>Ermittelt</th><th>Vorgabe</th><th
                        >Bewertung</th
                      ></tr
                    ></thead
                  ><tbody
                    >{#each g.pruefung.nachweise as e}<tr
                        ><td
                          >{e.art}{e.gegenstand ? " · " + e.gegenstand : ""}</td
                        ><td>{zahl(e.ist)}</td><td>{zahl(e.soll)}</td><td
                          >{e.eingehalten === null
                            ? "Nicht abschließend prüfbar"
                            : e.eingehalten
                              ? "Eingehalten"
                              : "Abweichung – prüfen"}</td
                        ></tr
                      >{/each}</tbody
                  >
                </table>{/each}{/if}
            <ul>
              {#each n.stand.regeln as r}<li>
                  {r.id} · Ausgabe {r.version} · {r.pruefstatus}
                </li>{/each}
            </ul>
            <p class="meta">
              Die Aktualität eines Berechnungslaufs ist von der fachlichen
              Freigabe und der Einhaltung einzelner Vorgaben zu unterscheiden.
            </p>
          </details></td
        ></tr
      >{:else}<tr
        ><td colspan="6">Noch keine Nachweisstände gespeichert.</td></tr
      >{/each}</tbody
  >
</table></div>

<style>
  .sammelaktionen {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 0.75rem;
  }
  .fruehere {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    margin-left: auto;
  }
  tr.frueher td {
    color: var(--muted, inherit);
  }
</style>
