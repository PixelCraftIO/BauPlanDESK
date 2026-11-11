<script lang="ts">
  import { projektNavigation } from "../projektNavigation.svelte";
  import { ziel } from "../arbeitsplatz/arbeitscheck";
  import { ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  import { untrack } from "svelte";
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
  const navigation = projektNavigation();
  async function dateienLaden() {
    try {
      dateien = await api.dokumente(projekt.id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  ansichtsAbfrage(() => projekt.id, ["dokumente"], dateienLaden);
  // Mit Anforderungen in einer Verfahrensakte ist das Register nachrangig.
  let mitAkte = $state(false);
  ansichtsAbfrage(
    () => projekt.id,
    ["fachobjekte"],
    async () => {
      try {
        const akten = await plattformApi.liste<{ anforderungen?: unknown[] }>(
          projekt.id,
          "verfahren",
        );
        mitAkte = akten.some((a) => a.daten.anforderungen?.length);
      } catch {
        mitAkte = false;
      }
    },
  );
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

{#if mitAkte}<aside class="hinweisbox">
    <strong>Die Verfahrensakte führt die Anforderungen dieses Projekts.</strong>
    <p>
      Welche Unterlagen und Nachweise der Antrag braucht, steht unter
      Verfahrensakte → Anforderungen und fließt von dort in Projektübersicht und
      Einreichungsprüfung. Das Unterlagenregister ist nur noch für zusätzliche
      Büropunkte gedacht; gleichnamige Einträge zählen nicht doppelt.
    </p>
    <button
      type="button"
      class="sekundaer"
      onclick={() => navigation?.springen(ziel("verfahren"))}
      >Verfahrensakte öffnen →</button
    >
  </aside>{/if}
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
    {#if navigation?.unterlagenauftrag?.ziel.bereich === "unterlagen" && navigation.unterlagenauftrag.ziel.objekt === s.id && navigation.unterlagenauftrag.dokument}<aside
        class="hinweisbox"
      >
        <button
          type="button"
          onclick={() => {
            if (navigation?.unterlagenauftrag?.dokument) {
              u.dokument = { ...navigation.unterlagenauftrag.dokument };
              u.status = "pruefung";
              navigation.unterlagenauftrag = null;
            }
          }}>Vorbereitete Dokumentversion zuordnen</button
        >
      </aside>{/if}
    {#if u.vorlage}<p class="meta">
        Checkliste {u.vorlage.id} · Ausgabe {u.vorlage.version} · Punkt {u
          .vorlage.punkt}
      </p>{/if}
    <div class="raster">
      <label
        >Unterlage<span class="pflicht" aria-hidden="true"> *</span><input
          aria-label="Unterlage"
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
    {#if u.dokument}
      {@const neueste = Math.max(
        ...dateien
          .filter((d) => d.document_id === u.dokument!.dokument_id)
          .map((d) => d.version),
      )}
      {#if neueste > u.dokument.version}<p class="hinweisbox">
          Zugeordnet ist Version {u.dokument.version}; im Projekt liegt Version {neueste}
          vor. Neue Version fachlich prüfen und bei Bedarf bewusst neu zuordnen und
          freigeben.
        </p>{/if}
      <button
        type="button"
        class="link"
        onclick={() =>
          navigation?.springen(
            ziel(
              "dokumente",
              u.dokument!.dokument_id,
              "historie",
              String(u.dokument!.version),
            ),
          )}>Zugeordnete Dokumentversion ansehen</button
      >
    {:else}<button
        type="button"
        class="link"
        onclick={() =>
          navigation?.unterlageErstellen(
            ziel("unterlagen", s.id),
            u.bezeichnung,
          )}>Formular im Projekt erstellen</button
      >{/if}
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
