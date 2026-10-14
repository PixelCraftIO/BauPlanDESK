<script lang="ts">
  import { onMount } from "svelte";
  import { open, ask } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, type AblageStatus } from "./api";
  let status = $state<AblageStatus | null>(null),
    laeuft = $state(false),
    fehler = $state<string | null>(null),
    hinweis = $state<string | null>(null);
  onMount(async () => {
    try {
      status = await api.ablageStatus();
    } catch (e) {
      fehler = fehlertext(e);
    }
  });
  export function istBeschaeftigt() {
    return laeuft;
  }
  async function waehlen() {
    if (!status?.wechsel_erlaubt) return;
    const target = await open({
      directory: true,
      multiple: false,
      title: "Neue Dateiablage wählen",
      defaultPath: status.root ?? undefined,
    });
    if (typeof target !== "string") return;
    if (
      !(await ask(
        `Alle verwalteten Dateiversionen in diesen Ordner kopieren und anschließend dort weiterarbeiten?\n\n${target}\n\nDie bisherige Ablage bleibt erhalten. Die Projektdatenbank wird nicht verschoben.`,
        {
          title: "Dateiablage wechseln",
          kind: "warning",
          okLabel: "Kopieren und wechseln",
          cancelLabel: "Abbrechen",
        },
      ))
    )
      return;
    laeuft = true;
    fehler = null;
    hinweis = null;
    try {
      const count = await api.ablageWechseln(target);
      status = await api.ablageStatus();
      hinweis = `${count} Dateiversionen geprüft und übernommen. Die neue Ablage ist aktiv.`;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

<section class="ablage-einstellungen">
  <h2>Dateiablage und Projektordner</h2>
  {#if fehler}<p class="fehlertext" role="alert">
      {fehler}
    </p>{/if}{#if hinweis}<p role="status">{hinweis}</p>{/if}
  {#if status}<dl class="daten">
      <dt>Betriebsart</dt>
      <dd>{status.modus}</dd>
      <dt>Verwaltete Dateien</dt>
      <dd>{status.root ?? "Objektspeicher laut Büro-Konfiguration"}</dd>
      <dt>Projektdaten</dt>
      <dd>{status.datenbank}</dd>
    </dl>
    <p class="meta">{status.hinweis}</p>
    <button
      class="sekundaer"
      disabled={!status.wechsel_erlaubt || laeuft}
      onclick={waehlen}
      >{laeuft
        ? "Dateien werden kopiert und geprüft …"
        : "Speicherort auswählen …"}</button
    >
    <p class="meta">
      Lesbare Projektordner lassen sich in der Dokumentenansicht ausgeben. Sie
      enthalten getrennte Verzeichnisse für Dokumente und CAD-Pläne, mit
      eindeutigen Dokumentversionen.
    </p>
    <details>
      <summary>Gemeinsame Ablage im Büro</summary>
      <p>
        Für mehrere Arbeitsplätze werden eine gemeinsame PostgreSQL-Datenbank
        und dieselbe NAS-/Serverablage benötigt. Jeder Arbeitsplatz verbindet
        sich mit derselben Büro-Konfiguration. Eine gemeinsame SQLite-Datei auf
        einer Freigabe ist dafür nicht vorgesehen.
      </p>
      <p>
        Die vorhandene Netzwerkeinrichtung wird in der Datei „config.json“ der
        Installation festgelegt. Im Netzwerkmodus wird der Ablageort zentral
        administriert, damit alle Benutzer dieselben Dokumente und Versionen
        sehen.
      </p>
    </details>
  {/if}
</section>
