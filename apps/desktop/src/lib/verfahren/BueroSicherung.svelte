<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext } from "../api";
  let { darf }: { darf: boolean } = $props();
  let busy = $state(false),
    fehler = $state(""),
    meldung = $state(""),
    zielDb = $state(""),
    profil = $state<string | null>(null);
  async function sichern() {
    busy = true;
    fehler = "";
    try {
      const ziel = await save({
        title: "Vollständige PostgreSQL-/NAS-Sicherung",
        defaultPath: "BauPlanDESK-Buero.zip",
        filters: [{ name: "ZIP", extensions: ["zip"] }],
      });
      if (ziel) {
        await invoke("buero_sicherung_erstellen", { ziel });
        meldung =
          "Bürosicherung einschließlich Datenbank, zentraler Dateien und Vorlagen erstellt und hashgeprüft.";
      }
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  async function wiederherstellen() {
    busy = true;
    fehler = "";
    try {
      const pfad = await open({
        title: "PostgreSQL-Bürosicherung wählen",
        multiple: false,
        filters: [{ name: "ZIP", extensions: ["zip"] }],
      });
      if (typeof pfad !== "string") return;
      const parent = await open({
        title: "Übergeordnetes Verzeichnis für neues Büroprofil",
        directory: true,
        multiple: false,
      });
      if (typeof parent !== "string") return;
      profil = await invoke<string>("buero_sicherung_wiederherstellen", {
        pfad,
        ziel: parent + `/BauPlanDESK-Buero-${Date.now()}`,
        databaseUrl: zielDb,
      });
      meldung =
        "Büroprofil in eine leere Zieldatenbank und ein neues Ablageverzeichnis wiederhergestellt. Das aktive Profil bleibt bis zur ausdrücklichen Auswahl bestehen.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  async function aktivieren() {
    if (!profil) return;
    busy = true;
    try {
      await api.profilAktivieren(profil);
      meldung = "Wiederhergestelltes Büroprofil nach Neustart ausgewählt.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
</script>

<section>
  <h3>Büroserver: PostgreSQL und NAS sichern / wiederherstellen</h3>
  <p>
    Vollständiges Archiv aus Datenbanksnapshot, zentralen Dateien,
    Formularvorlagen und Einstellungen. Benötigt die PostgreSQL-Clientprogramme
    pg_dump, pg_restore und psql in der Hauptversion des Datenbankservers. S3
    benötigt die Betreiber-Sicherung des Objektspeichers.
  </p>
  {#if fehler}<p role="alert" class="fehlertext">
      {fehler}
    </p>{/if}{#if meldung}<p role="status">{meldung}</p>{/if}
  <fieldset disabled={!darf || busy}>
    <button type="button" class="sekundaer" onclick={sichern}
      >Bürosicherung erstellen …</button
    ><label
      >Neue, leere PostgreSQL-Zieldatenbank<input
        type="password"
        autocomplete="off"
        placeholder="postgres://benutzer:passwort@server/neue_datenbank"
        bind:value={zielDb}
      /></label
    ><button
      type="button"
      class="sekundaer"
      disabled={!zielDb.trim()}
      onclick={wiederherstellen}
      >Bürosicherung getrennt wiederherstellen …</button
    >{#if profil}<button type="button" class="sekundaer" onclick={aktivieren}
        >Wiederhergestelltes Büroprofil nach Neustart verwenden</button
      >{/if}
  </fieldset>
</section>
