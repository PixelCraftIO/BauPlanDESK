<script lang="ts">
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    landName,
    type Project,
    type ZuletztEintrag,
  } from "./api";
  import type { Projektstand } from "./generated/plattform";
  let {
    zuletzt,
    darfAnlegen,
    darfLoeschen,
    darfUebernehmen = false,
    onneu,
    onoeffnen,
    ongeloescht,
  }: {
    zuletzt: ZuletztEintrag[];
    darfAnlegen: boolean;
    darfLoeschen: boolean;
    /** Verwaltung: Projekte aus einer Sicherung übernehmen. */
    darfUebernehmen?: boolean;
    onneu: () => void;
    onoeffnen: (id: string) => void;
    ongeloescht: (id: string) => void;
  } = $props();
  let projekte = $state<Project[]>([]),
    suche = $state(""),
    fehler = $state<string | null>(null),
    hinweis = $state<string | null>(null),
    laedt = $state(false),
    loescht = $state<string | null>(null),
    uebernimmt = $state(false);
  let sortierung = $state<"name" | "updated_at">("updated_at");
  let staende = $state<Record<string, Projektstand>>({}),
    standFilter = $state<"alle" | "laufend" | "abgeschlossen">("alle");
  const standPasst = (p: Project) =>
    standFilter === "alle" ||
    (staende[p.id]?.stufe === "abgeschlossen") ===
      (standFilter === "abgeschlossen");
  const liste = $derived(
    projekte
      .filter(standPasst)
      .filter((p) =>
        (p.name + " " + landName(p.bundesland))
          .toLowerCase()
          .includes(suche.trim().toLowerCase()),
      )
      .sort((a, b) =>
        sortierung === "name"
          ? a.name.localeCompare(b.name, "de")
          : b.updated_at.localeCompare(a.updated_at),
      ),
  );
  async function laden() {
    laedt = true;
    fehler = null;
    try {
      projekte = await api.projekte();
      // Der Stand ist Zusatzauskunft: Fehlt er, bleibt die Spalte leer.
      try {
        staende = Object.fromEntries(
          (await api.projektstaende()).map((s) => [s.projekt_id, s]),
        );
      } catch {
        staende = {};
      }
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laedt = false;
    }
  }
  async function loeschen(p: Project) {
    if (
      !(await ask(
        `„${p.name}“ endgültig löschen?

Die Projektakte mit allen Dokumenten, Versionen, Fachständen, Verfahrensdaten und Einreichungen wird entfernt. Das lässt sich nicht rückgängig machen. Der Löschvermerk bleibt im Protokoll; lesbare Projektordner auf dem Datenträger bleiben unverändert.`,
        {
          title: "Projekt löschen",
          kind: "warning",
          okLabel: "Endgültig löschen",
          cancelLabel: "Abbrechen",
        },
      ))
    )
      return;
    loescht = p.id;
    fehler = null;
    hinweis = null;
    try {
      const ergebnis = await api.projektLoeschen(p.id, p.version);
      if (ergebnis.dateien_verblieben.length)
        hinweis = `„${p.name}“ wurde gelöscht. ${ergebnis.dateien_verblieben.length} Datei(en) konnten nicht aus der Ablage entfernt werden: ${ergebnis.dateien_verblieben.join(", ")}`;
      ongeloescht(p.id);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      loescht = null;
      await laden();
    }
  }
  /**
   * Projekte aus einem Sicherungsarchiv (z. B. der Beispiel-ZIP aus dem
   * Handbuch) in das laufende Profil übernehmen, ohne Neustart. Ein einzelnes
   * übernommenes Projekt wird gleich geöffnet.
   */
  async function ausSicherung() {
    const pfad = await open({
      title: "Sicherung oder Beispielprojekt wählen",
      multiple: false,
      filters: [{ name: "Sicherungsarchiv", extensions: ["zip"] }],
    });
    if (typeof pfad !== "string") return;
    uebernimmt = true;
    fehler = null;
    hinweis = null;
    try {
      const neu = await api.projekteAusSicherungUebernehmen(pfad);
      hinweis =
        neu.length === 1
          ? `„${neu[0].name}“ wurde übernommen.`
          : `${neu.length} Projekte übernommen: ${neu.map((p) => `„${p.name}“`).join(", ")}.`;
      await laden();
      if (neu.length === 1) onoeffnen(neu[0].id);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      uebernimmt = false;
    }
  }
  $effect(() => {
    void zuletzt;
    void laden();
  });
</script>

<div class="verzeichnis-kopf">
  <div>
    <h1>Projektverzeichnis</h1>
    <p class="meta">Projektakten öffnen und parallel bearbeiten.</p>
  </div>
  <div class="knoepfe">
    {#if darfUebernehmen}<button
        class="sekundaer"
        onclick={ausSicherung}
        disabled={uebernimmt}
        title="Projekte aus einem Sicherungsarchiv, z. B. dem Beispielprojekt, in dieses Profil übernehmen. Eigene Projekte bleiben unverändert."
        >{uebernimmt ? "Wird übernommen …" : "Aus Sicherung übernehmen …"}</button
      >{/if}
    <button onclick={onneu} disabled={!darfAnlegen}>Neues Projekt</button>
  </div>
</div>
<div class="verzeichnis-filter">
  <label
    >Projekte suchen<input
      type="search"
      bind:value={suche}
      placeholder="Projektname oder Bundesland"
    /></label
  ><label
    >Sortierung<select bind:value={sortierung}
      ><option value="updated_at">Zuletzt geändert</option><option value="name"
        >Projektname</option
      ></select
    ></label
  ><label
    >Stand<select bind:value={standFilter} aria-label="Projektstand filtern"
      ><option value="alle">Alle Projekte</option><option value="laufend"
        >Laufende</option
      ><option value="abgeschlossen">Abgeschlossene</option></select
    ></label
  ><button class="sekundaer" onclick={laden} disabled={laedt}
    >Aktualisieren</button
  ><span class="meta">{liste.length} Projekte</span>
</div>
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
{#if hinweis}<p class="meta" role="status">{hinweis}</p>{/if}
<div class="tabellen-scroll">
  <table class="projekttabelle">
    <thead
      ><tr
        ><th>Projekt</th><th>Stand</th><th>Bundesland</th><th>Grundstück</th><th
          >Zuletzt geändert</th
        ><th>Aktion</th></tr
      ></thead
    ><tbody
      >{#each liste as p (p.id)}<tr
          ><td
            ><button class="link" onclick={() => onoeffnen(p.id)}
              >{p.name}</button
            ></td
          ><td
            >{#if staende[p.id]}{staende[p.id].bezeichnung}{#if staende[p.id].seit}<span
                  class="meta"
                  >{" "}· {new Date(
                    `${staende[p.id].seit}T12:00:00`,
                  ).toLocaleDateString("de-DE", {
                    day: "2-digit",
                    month: "2-digit",
                    year: "numeric",
                  })}</span
                >{/if}{:else}—{/if}</td
          ><td>{landName(p.bundesland)}</td><td class="zahl"
            >{#if p.planungsrecht.grundstuecksinformationen || p.planungsrecht.baugrundstuecke}{p
                .planungsrecht.grundstuecksinformationen?.flurstuecke.length ??
                p.planungsrecht.baugrundstuecke?.flurstuecke.length ??
                0} Flurstücke{:else}{p.grundstuecksflaeche_m2?.toLocaleString(
                "de-DE",
              ) ?? "—"}
              {p.grundstuecksflaeche_m2 == null
                ? ""
                : "m² (noch nicht aufgeschlüsselt)"}{/if}</td
          ><td>{new Date(p.updated_at).toLocaleDateString("de-DE", { day: "2-digit", month: "2-digit", year: "numeric" })}</td><td
            ><button class="sekundaer" onclick={() => onoeffnen(p.id)}
              >Öffnen</button
            >{#if darfLoeschen}
              <button
                class="sekundaer"
                aria-label={`${p.name} löschen`}
                disabled={loescht !== null}
                onclick={() => loeschen(p)}
                >{loescht === p.id ? "Löscht …" : "Löschen"}</button
              >{/if}</td
          ></tr
        >{:else}<tr
          ><td colspan="5"
            >{laedt
              ? "Projekte werden geladen …"
              : "Keine Projekte gefunden."}</td
          ></tr
        >{/each}</tbody
    >
  </table>
</div>
<p class="meta">
  Geöffnete Akten erscheinen oben als Projekttabs. Eingaben bleiben beim Wechsel
  erhalten.
</p>
