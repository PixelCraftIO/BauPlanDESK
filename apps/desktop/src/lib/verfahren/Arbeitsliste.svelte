<script lang="ts">
  import type { Verfahrensakte } from "../generated/plattform";
  import type { DocumentVersion, Benutzer } from "../api";
  import { projektNavigation } from "../projektNavigation.svelte";
  let {
    akte,
    dateien,
    benutzer,
    benutzerId,
    onregister,
  }: {
    akte: Verfahrensakte;
    dateien: DocumentVersion[];
    benutzer: Benutzer[];
    benutzerId: string;
    onregister: (register: string, eintrag?: string) => void;
  } = $props();
  const navigation = projektNavigation();
  let filter = $state("offen"),
    suche = $state(""),
    abgabe = $state("");
  const dateiname = (id: string, v: number) =>
    dateien.find((d) => d.document_id === id && d.version === v)?.file_name ??
    "Dokument nicht auflösbar";
  const letzte = (id: string) =>
    Math.max(
      0,
      ...dateien.filter((d) => d.document_id === id).map((d) => d.version),
    );
  const aufgaben = $derived(
    [
      ...akte.nachforderungen.flatMap((n) =>
        n.punkte.map((p) => ({
          id: p.id,
          titel: p.titel,
          art: "Nachforderung",
          status: {
            offen: "Offen",
            in_arbeit: "In Arbeit",
            vorbereitet: "Antwort vorbereitet",
            eingereicht: "Antwort eingereicht",
            bestaetigt: "Behördlich bestätigt",
            entfallen: "Entfallen",
          }[p.status],
          offen: !["bestaetigt", "entfallen"].includes(p.status),
          verantwortlich: p.verantwortlich,
          datum: p.frist?.datum ?? "",
          register: "nachforderungen",
          belege: p.dokumente.length,
        })),
      ),
      ...akte.bescheide.flatMap((b) =>
        b.auflagen
          .filter((p) => ["auflage", "bedingung"].includes(p.art))
          .map((p) => ({
            id: p.id,
            titel: p.titel,
            art: "Nebenbestimmung",
            status: p.erledigt ? "Erfüllt" : "Offen",
            offen: !p.erledigt,
            verantwortlich: p.verantwortlich,
            datum: p.frist?.datum ?? "",
            register: "bescheide",
            belege: p.nachweise.length,
          })),
      ),
    ]
      .filter(
        (p) =>
          (filter !== "offen" || p.offen) &&
          (filter !== "ich" || p.verantwortlich === benutzerId) &&
          p.titel
            .toLocaleLowerCase("de")
            .includes(suche.toLocaleLowerCase("de")),
      )
      .sort((a, b) => (a.datum || "9999").localeCompare(b.datum || "9999")),
  );
  function drucken() {
    const a = akte.abgaben.find((a) => a.id === abgabe);
    if (!a || !navigation) return;
    navigation.druckauswahl = {
      titel: a.titel,
      dokumente: a.dokumente.map((d) => ({
        dokument_id: d.dokument.dokument_id,
        version: d.dokument.version,
      })),
    };
    navigation.springen({
      bereich: "druck",
      objekt: null,
      register: null,
      eintrag: null,
    });
  }
</script>

<section aria-label="Verfahrensbezogener Arbeitsplatz">
  <h3>Unterlagen und nächste Arbeitsschritte</h3>
  <p>
    Diese Ansicht bündelt die vorhandenen Anforderungen und Fachstatus.
    Änderungen erfolgen am jeweiligen Originaleintrag; es gibt keinen zweiten
    Erledigungsstatus.
  </p>
  <div class="knoepfe links">
    <button
      type="button"
      class="link"
      onclick={() => onregister("anforderungen")}
      >Anforderungen bearbeiten →</button
    ><button
      type="button"
      class="link"
      onclick={() => onregister("kommunikation")}
      >Originalschreiben / Belege →</button
    >
  </div>
  <div class="tabellen-scroll">
    <table class="ergebnistabelle">
      <thead
        ><tr
          ><th>Anforderung</th><th>Erforderlichkeit / Prüfung</th><th
            >Dokument / Version</th
          ><th></th></tr
        ></thead
      ><tbody>
        {#each akte.anforderungen as a (a.id)}<tr
            ><td>{a.titel}</td><td
              >{a.erforderlich} · {a.freigegeben
                ? "Freigegeben"
                : "Prüfung offen"}</td
            ><td
              >{#each a.dokumente as d}<div>
                  {dateiname(d.dokument_id, d.version)} · Version {d.version}{#if letzte(d.dokument_id) > d.version}<strong
                    >
                      · neuere Projektversion {letzte(d.dokument_id)} prüfen</strong
                    >{/if}
                </div>{:else}Keine Version zugeordnet{/each}</td
            ><td
              ><button
                type="button"
                class="link"
                onclick={() => onregister("anforderungen", a.id)}
                >Bearbeiten / Unterlage erstellen</button
              ></td
            ></tr
          >{:else}<tr
            ><td colspan="4"
              >Noch keine Anforderungen erfasst. Vorhabenprofil und
              Anforderungen vorbereiten.</td
            ></tr
          >{/each}
      </tbody>
    </table>
  </div>
  <h4>Nachforderungen und Pflichten aus Bescheiden</h4>
  <button
    type="button"
    class="link"
    onclick={() =>
      navigation?.springen({
        bereich: "aktensuche",
        objekt: null,
        register: null,
        eintrag: null,
      })}>Fristen, Wiedervorlagen und Kalenderexport öffnen →</button
  >
  <p>
    Eine vorbereitete Antwort ist noch keine Einreichung. Behördliche
    Bestätigung bleibt eine eigene Entscheidung mit Beleg. Hinweise ohne Pflicht
    werden hier nicht als Aufgabe gezählt.
  </p>
  <div class="raster">
    <label
      >Aufgabenfilter<select bind:value={filter}
        ><option value="offen">Offene Punkte</option><option value="ich"
          >Mir zugewiesen</option
        ><option value="alle">Alle Punkte</option></select
      ></label
    ><label
      >Arbeitsschritt suchen<input type="search" bind:value={suche} /></label
    >
  </div>
  <div class="tabellen-scroll">
    <table class="ergebnistabelle">
      <thead
        ><tr
          ><th>Art / Aufgabe</th><th>Führender Status</th><th>Frist</th><th
            >Zuständig</th
          ><th>Antwort-/Erfüllungsbelege</th><th></th></tr
        ></thead
      ><tbody
        >{#each aufgaben as p (p.id)}<tr
            ><td>{p.art} · {p.titel}</td><td>{p.status}</td><td
              >{p.datum || "Noch keine Frist erfasst"}</td
            ><td
              >{benutzer.find((b) => b.id === p.verantwortlich)?.anzeigename ??
                "Nicht zugewiesen"}</td
            ><td>{p.belege}</td><td
              ><button
                type="button"
                class="link"
                onclick={() => onregister(p.register, p.id)}
                >Originalpunkt bearbeiten →</button
              ></td
            ></tr
          >{:else}<tr
            ><td colspan="6"
              >Keine Punkte für diesen Filter. Das bestätigt keine
              Vollständigkeit der Akte.</td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>
  <h4>Druck aus einem eingefrorenen Abgabestand</h4>
  <label
    >Abgabestand für Druck auswählen<select bind:value={abgabe}
      ><option value="">Abgabestand wählen</option
      >{#each akte.abgaben as a}<option value={a.id}
          >{a.titel} · {a.erstellt_am}</option
        >{/each}</select
    ></label
  ><button type="button" disabled={!abgabe} onclick={drucken}
    >Diese Dokumentversionen zur Druckausgabe übernehmen →</button
  >
</section>
