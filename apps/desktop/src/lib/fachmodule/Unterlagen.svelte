<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api, fehlertext, type Project, type DocumentVersion } from "../api";
  import type { Checkliste, Unterlage } from "../generated/plattform";
  import { plattformApi } from "../plattformApi";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  let {
    projekt,
    benutzerId,
    darf,
  }: { projekt: Project; benutzerId: string; darf: boolean } = $props();
  const s = untrack(
    () =>
      new FachSitzung<Unterlage>(
        projekt.id,
        "unterlagen",
        benutzerId,
        () => ({
          vorlage: null,
          bezeichnung: "",
          erforderlich: true,
          dokument: null,
          status: "entwurf",
          bemerkung: "",
        }),
        (u) =>
          `${u.bezeichnung} · ${u.dokument ? (u.status === "freigegeben" ? "Freigegeben" : "Zu prüfen") : "Dokument fehlt"}`,
      ),
  );
  let dateien = $state<DocumentVersion[]>([]);
  onMount(async () => {
    try {
      dateien = await api.dokumente(projekt.id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
  let vorlage = $state<Checkliste | null>(null);
  async function standard() {
    try {
      vorlage = await plattformApi.standardCheckliste();
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  async function importieren(e: Event) {
    try {
      const file = (e.currentTarget as HTMLInputElement).files?.[0];
      if (!file) return;
      if (file.size > 256 * 1024)
        throw new Error("Checkliste darf höchstens 256 KB groß sein");
      vorlage = JSON.parse(await file.text());
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  async function anwenden() {
    if (!vorlage) return;
    s.laeuft = true;
    s.fehler = null;
    try {
      const anzahl = await plattformApi.checklisteAnwenden(projekt.id, vorlage);
      await s.laden();
      s.meldung = `${anzahl} neue Unterlagen übernommen. Vorhandene Einträge bleiben erhalten.`;
      vorlage = null;
    } catch (e) {
      s.fehler = fehlertext(e);
    } finally {
      s.laeuft = false;
    }
  }
</script>

<div class="zeile">
  <button
    class="sekundaer"
    disabled={!darf || s.laeuft || s.hatAenderungen()}
    onclick={standard}>Bürocheckliste vorbereiten</button
  ><label
    >Eigene Checkliste laden<input
      type="file"
      accept="application/json,.json"
      disabled={!darf || s.laeuft || s.hatAenderungen()}
      onchange={importieren}
    /></label
  >
</div>
{#if vorlage}<aside class="hinweisbox">
    <h3>{vorlage.titel} · {vorlage.version}</h3>
    <p>{vorlage.hinweis}</p>
    {#if Array.isArray(vorlage.unterlagen)}<ul>
        {#each vorlage.unterlagen as p}<li>
            {p.bezeichnung}{p.erforderlich ? " · erforderlich" : ""}
          </li>{/each}
      </ul>{/if}<button disabled={!darf || s.laeuft} onclick={anwenden}
      >Unterlagen übernehmen</button
    ><button class="link" onclick={() => (vorlage = null)}>Abbrechen</button>
  </aside>{/if}

<Fachliste titel="Unterlagenregister" {s} bereich="unterlagen" {darf}>
  {#snippet children(u)}
    {#if u.vorlage}<p class="meta">
        Checkliste {u.vorlage.id} · Ausgabe {u.vorlage.version} · Punkt {u
          .vorlage.punkt}
      </p>{/if}
    <div class="raster">
      <label
        >Unterlage<input
          bind:value={u.bezeichnung}
          required
          maxlength="500"
        /></label
      ><label
        ><input type="checkbox" bind:checked={u.erforderlich} />Für dieses
        Verfahren erforderlich</label
      ><label
        >Dokumentstand<select
          value={u.dokument
            ? `${u.dokument.dokument_id}:${u.dokument.version}`
            : ""}
          onchange={(e) => {
            const d = dateien.find(
              (d) => `${d.document_id}:${d.version}` === e.currentTarget.value,
            );
            u.dokument = d
              ? { dokument_id: d.document_id, version: d.version }
              : null;
          }}
          ><option value="">Noch kein Dokument zugeordnet</option
          >{#each dateien as d (`${d.document_id}:${d.version}`)}<option
              value={`${d.document_id}:${d.version}`}
              >{d.file_name} · Version {d.version}</option
            >{/each}</select
        ></label
      ><label
        >Prüfstatus<select bind:value={u.status}
          ><option value="entwurf">Entwurf</option><option value="pruefung"
            >In Prüfung</option
          ><option value="freigegeben">Freigegeben</option></select
        ></label
      >
    </div>
    <label
      >Hinweis / begründete Abweichung<textarea
        bind:value={u.bemerkung}
        maxlength="4000"
      ></textarea></label
    >
    <p class="meta">
      Die Freigabe ist eine fachliche Entscheidung der Projektleitung. Ein
      zugeordnetes Dokument gilt erst nach bestätigter Einreichung als
      übermittelt.
    </p>{/snippet}
</Fachliste>
