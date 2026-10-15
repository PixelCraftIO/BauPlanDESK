<script lang="ts">
  import { onMount } from "svelte";
  import type { BaugrundstuecksPruefung, Project } from "../api";
  import { fehlertext } from "../api";
  import type { NachweisAnsicht, JsonValue } from "../generated/plattform";
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
    laeuft = $state(false);
  async function laden() {
    laeuft = true;
    try {
      liste = await plattformApi.nachweise(projekt.id);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  onMount(laden);
  async function freigeben(n: NachweisAnsicht) {
    laeuft = true;
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
      laeuft = false;
    }
  }
  async function wiederholen(n: NachweisAnsicht) {
    laeuft = true;
    try {
      const neu = await plattformApi.wiederholen(projekt.id, n.stand.id);
      meldung =
        JSON.stringify(neu) === JSON.stringify(n.stand.ergebnis)
          ? "Der archivierte Rechenstand wurde mit demselben Ergebnis wiederholt."
          : "Die Wiederholung weicht vom archivierten Ergebnis ab. Bitte fachlich prüfen.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  function nutzung(wert:JsonValue):BaugrundstuecksPruefung|null {return wert&&typeof wert==='object'&&!Array.isArray(wert)&&Array.isArray(wert.baugrundstuecke)?wert as unknown as BaugrundstuecksPruefung:null;}
  function zahl(n:number|null|undefined){return n===null||n===undefined?'—':n.toLocaleString('de-DE',{maximumFractionDigits:3});}
  function ergebniszeilen(wert: JsonValue): { name: string; wert: string }[] {
    if (!wert || Array.isArray(wert) || typeof wert !== "object") return [];
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
<button class="sekundaer" disabled={laeuft} onclick={laden}
  >Aktualität prüfen</button
>
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}{#if meldung}<p
    role="status"
  >
    {meldung}
  </p>{/if}
<table class="ergebnistabelle">
  <thead
    ><tr
      ><th>Nachweis</th><th>Erstellt</th><th>Vollständigkeit</th><th
        >Aktualität</th
      ><th>Freigabe</th><th></th></tr
    ></thead
  ><tbody
    >{#each liste as n (n.stand.id)}<tr
        ><td>{n.stand.bezeichnung}</td><td
          >{new Date(n.stand.erstellt_am).toLocaleString("de-DE")}</td
        ><td
          >{n.stand.vollstaendig ? "Vollständig" : "Eingaben unvollständig"}</td
        ><td>{n.aktuell ? "Aktuell" : "Grundlagen geändert – neu prüfen"}</td
        ><td
          >{n.freigabe
            ? n.aktuell
              ? "Freigegeben"
              : "Freigabe des früheren Stands"
            : "Offen"}</td
        ><td
          >{#if !n.freigabe}<button
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
          ></td
        ></tr
      ><tr
        ><td colspan="6"
          ><details>
            <summary>Grundlagen und Regeln ansehen</summary
            >{#each ergebniszeilen(n.stand.ergebnis) as z}<p>
                {z.name}: <strong>{z.wert}</strong>
              </p>{/each}
            {#if nutzung(n.stand.ergebnis)}{#each nutzung(n.stand.ergebnis)!.baugrundstuecke as g}<h4>{g.grundstueck.bezeichnung}</h4><p>GRZ {zahl(g.pruefung.kennzahlen.grz)} · GFZ {zahl(g.pruefung.kennzahlen.gfz)} · BMZ {zahl(g.pruefung.kennzahlen.bmz)}</p><table class="ergebnistabelle"><thead><tr><th>Prüfung</th><th>Ermittelt</th><th>Vorgabe</th><th>Bewertung</th></tr></thead><tbody>{#each g.pruefung.nachweise as e}<tr><td>{e.art}{e.gegenstand?' · '+e.gegenstand:''}</td><td>{zahl(e.ist)}</td><td>{zahl(e.soll)}</td><td>{e.eingehalten===null?'Nicht abschließend prüfbar':e.eingehalten?'Eingehalten':'Abweichung – prüfen'}</td></tr>{/each}</tbody></table>{/each}{/if}
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
</table>
