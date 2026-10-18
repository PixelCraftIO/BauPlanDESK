<script lang="ts">
  import Druckknopf from "./Druckknopf.svelte";
  import { editorAnmelden } from "./editoren";
  import { untrack } from "svelte";
  import { ask, open, save } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    istKonflikt,
    istPlan,
    mindestens,
    rollenName,
    type Benutzer,
    type CmdError,
    type DocumentLock,
    type DocumentVersion,
  } from "./api";
  import EmailAnsicht from "./EmailAnsicht.svelte";
  import KategorieAuswahl from "./KategorieAuswahl.svelte";
  import {
    kategorieText,
    kategoriePasst,
    kategorienOptionen,
  } from "./kategorien";
  import type { DokumentKategorie, Dokumentkategorie } from "./api";
  import Ausgabeauftraege from "./Ausgabeauftraege.svelte";
  import Arbeitskopien from "./Arbeitskopien.svelte";
  import Dialog from "./Dialog.svelte";
  import FormularAusfuellen from "./FormularAusfuellen.svelte";
  import Historie from "./Historie.svelte";
  import Dokumentdaten from "./Dokumentdaten.svelte";
  import { abonnieren, melden } from "./projektEreignisse";
  let metadaten = $state<DocumentVersion | null>(null),
    metaDirty = $state(false),
    metaBusy = $state(false);
  async function metaSchliessen() {
    return (
      !metaBusy &&
      (!metaDirty ||
        (await ask(
          "Dokumentangaben wurden noch nicht gespeichert. Verwerfen?",
          {
            title: "Dokumentangaben",
            kind: "warning",
            okLabel: "Verwerfen",
            cancelLabel: "Weiter bearbeiten",
          },
        )))
    );
  }
  async function metaAbbrechen() {
    if (await metaSchliessen()) metadaten = null;
  }

  let {
    projektId,
    benutzer,
    aktiv = false,
    importAusloeser,
    onplan,
    onfehler,
  }: {
    projektId: string;
    benutzer: Benutzer;
    /** Sichtbar: beim Einblenden wird die Liste neu geladen. */
    aktiv?: boolean;
    importAusloeser: number;
    onplan: (dokumentId: string) => void;
    onfehler: (m: string) => void;
  } = $props();

  let arbeitsEditor = $state<Arbeitskopien>();
  let suche = $state("");
  let kategorien = $state<Dokumentkategorie[]>([]);
  let zuordnungen = $state<DokumentKategorie[]>([]);
  let kategorieFilter = $state(""),
    sortierung = $state("datei"),
    importKategorie = $state("");
  let auswahl = $state<string[]>([]),
    kategorieDialog = $state(false),
    neueKategorie = $state(""),
    kategorieFehler = $state("");
  const kategorieAnzahlen = $derived.by(() => {
    const zaehler = { alle: 0, ohne: 0, kategorien: new Map<string, number>() };
    for (const id of new Set(versionen.map((d) => d.document_id))) {
      zaehler.alle++;
      const kat = zuordnungen.find((d) => d.dokument_id === id)?.kategorie;
      if (!kat) {
        zaehler.ohne++;
        continue;
      }
      zaehler.kategorien.set(kat, (zaehler.kategorien.get(kat) ?? 0) + 1);
      const parent = kategorien.find((k) => k.id === kat)?.eltern_id;
      if (parent)
        zaehler.kategorien.set(
          parent,
          (zaehler.kategorien.get(parent) ?? 0) + 1,
        );
    }
    return zaehler;
  });
  const zuordnung = (id: string) =>
    zuordnungen.find((k) => k.dokument_id === id);
  function kategorieBearbeiten(ids: string[]) {
    auswahl = ids;
    neueKategorie =
      ids.length === 1 ? (zuordnung(ids[0])?.kategorie ?? "") : "";
    kategorieFehler = "";
    kategorieDialog = true;
  }
  async function kategorisieren(e: SubmitEvent) {
    e.preventDefault();
    kategorieFehler = "";
    laeuft = true;
    const auftraege = auswahl.map((id) => ({
      id,
      revision: zuordnung(id)?.revision ?? 0,
    }));
    const ziel = neueKategorie || null;
    let anzahl = 0;
    try {
      for (const a of auftraege) {
        await api.dokumentkategorieSpeichern(a.id, {
          revision: a.revision,
          kategorie: ziel,
        });
        auswahl = auswahl.filter((id) => id !== a.id);
        anzahl++;
      }
      kategorieDialog = false;
      hinweis = `Kategorie für ${anzahl} Unterlage(n) gespeichert.`;
    } catch (e) {
      kategorieFehler = `${anzahl} Zuordnung(en) gespeichert. Die übrigen Unterlagen bleiben ausgewählt. ${fehlertext(e)}`;
    } finally {
      laeuft = false;
      await laden();
    }
  }

  let versionen = $state<DocumentVersion[]>([]);
  let laeuft = $state(false);
  let formular = $state<DocumentVersion | null>(null);
  let email = $state<DocumentVersion | null>(null);
  let hinweis = $state<string | null>(null);
  let sperren = $state<Map<string, DocumentLock>>(new Map());
  let historie = $state<DocumentVersion | null>(null);
  let pdfDirty = $state(false),
    pdfBusy = $state(false),
    ordnerLaeuft = $state(false),
    ordnerHistorie = $state(false);
  async function pdfSchliessenErlaubt() {
    return (
      !pdfBusy &&
      (!pdfDirty ||
        (await ask("PDF-Eingaben wurden noch nicht gespeichert. Verwerfen?", {
          title: "PDF-Formular",
          kind: "warning",
          okLabel: "Verwerfen",
          cancelLabel: "Weiter bearbeiten",
        })))
    );
  }
  async function pdfAbbrechen() {
    if (await pdfSchliessenErlaubt()) {
      formular = null;
      pdfDirty = false;
    }
  }
  async function inBibliothek(d: DocumentVersion) {
    if (
      !(await ask(
        "Diese PDF als gemeinsame Vorlage in der Formularbibliothek ablegen? Enthaltene Eingaben werden mitkopiert. Verwenden Sie eine leere Vorlage ohne persönliche Daten.",
        { title: "Formularbibliothek", kind: "warning" },
      ))
    )
      return;
    laeuft = true;
    try {
      const v = await api.bibliothekVeroeffentlichen(d.document_id, d.version);
      hinweis = `In der Formularbibliothek abgelegt: ${v.name}`;
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      laeuft = false;
    }
  }
  async function projektordnerExportieren() {
    const root = await api.ablageStatus().catch(() => null);
    const ziel = await open({
      directory: true,
      multiple: false,
      title: "Projektordner ausgeben",
      defaultPath: root?.root ?? undefined,
    });
    if (typeof ziel !== "string") return;
    ordnerLaeuft = true;
    try {
      const r = await api.projektordnerExportieren(
        projektId,
        ziel,
        ordnerHistorie,
      );
      hinweis = `${r.dateien} Dateien (${r.neu} neu) ausgegeben: ${r.ordner}`;
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      ordnerLaeuft = false;
    }
  }

  const darfAblegen = $derived(
    benutzer.aktiv && mindestens(benutzer.rolle, "bearbeitung"),
  );
  const darfFremdeAufheben = $derived(
    benutzer.aktiv && mindestens(benutzer.rolle, "projektleitung"),
  );
  /** Gesperrt von jemand anderem: Ablegen ist dann nicht möglich. */
  const fremdGesperrt = (id: string) => {
    const s = sperren.get(id);
    return !!s && s.benutzer_id !== benutzer.id;
  };

  const istPdf = (d: DocumentVersion) =>
    d.file_name.toLowerCase().endsWith(".pdf");

  async function externOeffnen(doc: DocumentVersion) {
    laeuft = true;
    try {
      await api.dokumentExternOeffnen(doc.document_id, doc.version);
      hinweis = `${doc.file_name} als Arbeitskopie im zugeordneten Programm geöffnet.`;
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      laeuft = false;
      await arbeitsEditor?.aktualisieren();
    }
  }
  async function exportieren(doc: DocumentVersion) {
    const ziel = await save({
      title: "Dokument exportieren",
      defaultPath: doc.file_name,
      filters: istPdf(doc) ? [{ name: "PDF", extensions: ["pdf"] }] : [],
    });
    if (!ziel) return;
    try {
      await api.dokumentExportieren(doc.document_id, ziel);
      hinweis = `${doc.file_name} wurde exportiert.`;
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  async function formularFertig(ergebnis: DocumentVersion) {
    formular = null;
    pdfDirty = false;
    await laden();
    hinweis = `${ergebnis.file_name} wurde abgelegt.`;
    hinweis =
      ergebnis.ablage_hinweis ??
      `${ergebnis.file_name} wurde im Projekt gespeichert. ${ergebnis.projektordner_pfad ?? ""}`;
  }

  // Je Dokument nur die neueste Version anzeigen, ältere als Historie zählen.
  const dokumente = $derived.by(() => {
    const neueste = new Map<string, { doc: DocumentVersion; anzahl: number }>();
    for (const v of versionen) {
      const bisher = neueste.get(v.document_id);
      if (!bisher || v.version > bisher.doc.version) {
        neueste.set(v.document_id, {
          doc: v,
          anzahl: (bisher?.anzahl ?? 0) + 1,
        });
      } else {
        bisher.anzahl++;
      }
    }
    return [...neueste.values()]
      .filter(
        ({ doc }) =>
          kategoriePasst(
            kategorien,
            zuordnung(doc.document_id)?.kategorie,
            kategorieFilter,
          ) &&
          doc.file_name
            .toLocaleLowerCase("de")
            .includes(suche.toLocaleLowerCase("de")),
      )
      .sort((a, b) => {
        const kat =
          sortierung === "kategorie"
            ? kategorieText(
                kategorien,
                zuordnung(a.doc.document_id)?.kategorie,
              ).localeCompare(
                kategorieText(
                  kategorien,
                  zuordnung(b.doc.document_id)?.kategorie,
                ),
                "de",
              )
            : 0;
        return kat || a.doc.file_name.localeCompare(b.doc.file_name, "de");
      });
  });

  export async function aktualisieren() {
    await laden();
  }
  /** Stand der zuletzt geladenen Liste (Dokument und Version). */
  let listenStand = "";
  /**
   * Lädt Dateiliste, Sperren und Kategorien. Hat sich die Liste nach einer
   * eigenen Aktion geändert, erfahren es die übrigen Bereiche der Akte
   * (Bauantrag-Anlagen, Fachberechnungen); `melden = false` für Ladevorgänge,
   * die selbst auf eine Meldung oder das Einblenden reagieren.
   */
  async function laden(meldenNachAenderung = true) {
    try {
      const [v, s, k, z] = await Promise.all([
        api.dokumente(projektId),
        api.sperrenAuflisten(projektId),
        api.kategorienkatalog(),
        api.dokumentkategorien(projektId),
      ]);
      const stand = v.map((d) => `${d.document_id}:${d.version}`).sort().join();
      if (meldenNachAenderung && listenStand && stand !== listenStand)
        melden({ projektId, bereich: "dokumente", quelle: "dokumente" });
      listenStand = stand;
      versionen = v;
      kategorien = k.kategorien;
      zuordnungen = z;
      auswahl = auswahl.filter((id) => v.some((d) => d.document_id === id));
      sperren = new Map(s.map((x) => [x.document_id, x]));
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  /** Verständliche Meldung für die beiden typischen Mehrbenutzer-Fälle. */
  function ablegeFehler(e: unknown, dateiname: string) {
    if (istKonflikt(e)) {
      onfehler(
        `Inzwischen hat jemand eine neuere Version von „${dateiname}“ abgelegt. Ihre Datei wurde nicht gespeichert, damit nichts überschrieben wird. Sehen Sie sich die neue Version an und legen Sie Ihre Änderungen danach erneut ab.`,
      );
    } else if ((e as CmdError)?.art === "gesperrt") {
      onfehler(`„${dateiname}“ ist gesperrt. ${fehlertext(e)}`);
    } else {
      onfehler(fehlertext(e));
    }
  }

  async function sperrenUmschalten(doc: DocumentVersion) {
    const s = sperren.get(doc.document_id);
    try {
      if (!s) {
        await api.sperren(doc.document_id);
        hinweis = `${doc.file_name} ist für Sie gesperrt. Andere können es ansehen, aber keine neue Version ablegen.`;
      } else {
        if (
          s.benutzer_id !== benutzer.id &&
          !(await ask(
            `${doc.file_name} ist von ${s.benutzer_name} gesperrt. Sperre trotzdem aufheben?`,
            {
              title: "Fremde Sperre aufheben",
              kind: "warning",
              okLabel: "Aufheben",
              cancelLabel: "Abbrechen",
            },
          ))
        )
          return;
        await api.entsperren(doc.document_id);
        hinweis = `Sperre für ${doc.file_name} aufgehoben.`;
      }
    } catch (e) {
      onfehler(fehlertext(e));
    }
    await laden();
  }

  export async function importieren(dokumentId: string | null = null) {
    if (!darfAblegen) {
      onfehler(
        `Dokumente ablegen dürfen nur Benutzer mit der Rolle „Bearbeitung“ oder höher (Ihre Rolle: „${rollenName(benutzer.rolle)}“).`,
      );
      return;
    }
    const auswahl = await open({
      multiple: dokumentId === null,
      title: dokumentId ? "Neue Version auswählen" : "Dokumente importieren",
      filters: [
        {
          name: "Projektdateien",
          extensions: [
            "pdf",
            "dxf",
            "dwg",
            "xml",
            "png",
            "jpg",
            "jpeg",
            "xlsx",
            "xls",
            "docx",
            "doc",
            "odt",
            "ods",
            "csv",
            "txt",
            "eml",
            "msg",
          ],
        },
        { name: "Alle Dateien", extensions: ["*"] },
      ],
    });
    if (!auswahl) return;
    const pfade = Array.isArray(auswahl) ? auswahl : [auswahl];
    // Neue Version: auf der Version aufbauen, die hier angezeigt wird.
    // Hat jemand anderes inzwischen abgelegt, meldet core-lib einen Konflikt.
    const bisher = dokumentId
      ? dokumente.find((d) => d.doc.document_id === dokumentId)?.doc
      : undefined;
    laeuft = true;
    try {
      for (const pfad of pfade) {
        const v = await api.dokumentImportieren(
          projektId,
          pfad,
          dokumentId,
          bisher?.version ?? null,
          dokumentId ? null : importKategorie || null,
        );
        hinweis =
          v.ablage_hinweis ??
          `${v.file_name} wurde als Version ${v.version} abgelegt.`;
      }
    } catch (e) {
      ablegeFehler(e, bisher?.file_name ?? "Dokument");
    } finally {
      laeuft = false;
      await laden();
    }
  }

  $effect(() => {
    void projektId;
    untrack(() => laden(false));
  });
  // Andere Bereiche (Druckausgabe, Formulare, E-Mail-Anhänge) haben abgelegt.
  $effect(() =>
    abonnieren(projektId, ["dokumente"], (e) => {
      if (e.quelle !== "dokumente") void laden(false);
    }),
  );
  // Beim Einblenden neu laden, z. B. nach Ablagen an anderen Arbeitsplätzen.
  let warSichtbar = untrack(() => aktiv);
  $effect(() => {
    const sichtbar = aktiv;
    if (sichtbar && !warSichtbar) untrack(() => laden(false));
    warSichtbar = sichtbar;
  });

  // Menü "Datei → Dokument importieren" zählt diesen Wert hoch.
  let letzterAusloeser = untrack(() => importAusloeser);
  $effect(() => {
    if (importAusloeser > letzterAusloeser) {
      letzterAusloeser = importAusloeser;
      importieren();
    }
  });

  const groesse = (b: number) =>
    b < 1024 * 1024
      ? `${(b / 1024).toFixed(0)} KB`
      : `${(b / 1024 / 1024).toFixed(1)} MB`;
  export function istBeschaeftigt() {
    return (
      arbeitsEditor?.istBeschaeftigt() ||
      laeuft ||
      ordnerLaeuft ||
      !!formular ||
      !!email ||
      !!historie ||
      !!metadaten ||
      kategorieDialog
    );
  }
  editorAnmelden("dokumente", {
    hatAenderungen: () => false,
    istBeschaeftigt: istBeschaeftigt,
    speichern: async () => true,
    hinweis:
      "Einreichungsauswahl zuerst als Paket festhalten oder ausdrücklich verwerfen.",
  });
</script>

<div class="werkzeugleiste dateiaktionen">
  <button class="sekundaer" disabled={laeuft} onclick={() => laden()}
    >Dateiliste aktualisieren</button
  >
  <KategorieAuswahl
    {kategorien}
    bind:wert={importKategorie}
    label="Kategorie für neue Importe"
    disabled={laeuft || !darfAblegen}
  />
  <button
    onclick={() => importieren()}
    disabled={laeuft || !darfAblegen}
    title={darfAblegen
      ? ""
      : "Dazu ist mindestens die Rolle „Bearbeitung“ nötig."}
    >{laeuft ? "Importiere …" : "Dateien importieren …"}</button
  >
  <button
    class="sekundaer"
    onclick={projektordnerExportieren}
    disabled={ordnerLaeuft}
    >{ordnerLaeuft
      ? "Ordner wird erstellt …"
      : "Projektordner exportieren …"}</button
  >
  <label class="zeile"
    ><input type="checkbox" bind:checked={ordnerHistorie} /> Mit Dokumenthistorie</label
  >
</div>
<p class="meta">
  Projektordner sind Exportkopien mit „Dokumente“ und „Pläne“. Änderungen an
  diesen Kopien werden nicht automatisch in die Anwendung übernommen.
</p>

<div class="dateifilter werkzeugleiste">
<label class="dateisuche"
  >Dateien suchen<input
    type="search"
    bind:value={suche}
    placeholder="Dateiname, z. B. Lageplan oder .xlsx"
  /></label
>

  <label
    >Kategorie filtern<select
      aria-label="Kategorie filtern"
      bind:value={kategorieFilter}
      ><option value="">Alle Kategorien ({kategorieAnzahlen.alle})</option
      ><option value=":ohne">Ohne Kategorie ({kategorieAnzahlen.ohne})</option
      >{#each kategorienOptionen(kategorien) as k}<option value={k.id}
          >{k.bezeichnung} ({kategorieAnzahlen.kategorien.get(k.id) ??
            0}){k.aktiv ? "" : " (archiviert)"}</option
        >{/each}</select
    ></label
  >
  <label
    >Sortierung<select aria-label="Sortierung" bind:value={sortierung}
      ><option value="datei">Dateiname</option><option value="kategorie"
        >Nach Kategorie gruppieren</option
      ></select
    ></label
  >
</div>
<div class="knoepfe links" aria-label="Schnellfilter für Unterlagen">
  <button
    class="sekundaer"
    aria-pressed={kategorieFilter === ""}
    onclick={() => (kategorieFilter = "")}
    >Alle Unterlagen ({kategorieAnzahlen.alle})</button
  >
  <button
    class="sekundaer"
    aria-pressed={kategorieFilter === ":ohne"}
    onclick={() => (kategorieFilter = ":ohne")}
    >Ohne Kategorie ({kategorieAnzahlen.ohne})</button
  >
</div>
<div class="knoepfe links">
  <button
    class="sekundaer"
    disabled={laeuft ||
      !darfAblegen ||
      !dokumente.some(({ doc }) => !fremdGesperrt(doc.document_id))}
    onclick={() =>
      (auswahl = [
        ...new Set([
          ...auswahl,
          ...dokumente
            .filter(({ doc }) => !fremdGesperrt(doc.document_id))
            .map(({ doc }) => doc.document_id),
        ]),
      ])}>Sichtbare auswählen</button
  >
  <button
    class="sekundaer"
    disabled={laeuft || !darfAblegen || !auswahl.length}
    onclick={() => kategorieBearbeiten(auswahl)}
    >Kategorie zuordnen ({auswahl.length})</button
  >
  {#if auswahl.length}<button
      class="link"
      disabled={laeuft}
      onclick={() => (auswahl = [])}>Auswahl aufheben</button
    >{/if}
</div>

{#if hinweis}
  <p class="meta" role="status">{hinweis}</p>
{/if}

<div class="tabellen-scroll">
  <table class="ergebnistabelle" aria-label="Projektdateien">
    <thead
      ><tr
        ><th>Auswahl</th><th>Datei</th><th>Kategorie</th><th>Version</th><th
          >Größe</th
        ><th>Geändert</th><th>Sperre</th><th></th></tr
      ></thead
    >
    <tbody>
      {#each dokumente as { doc, anzahl }, index (doc.document_id)}
        {#if sortierung === "kategorie" && (index === 0 || zuordnung(doc.document_id)?.kategorie !== zuordnung(dokumente[index - 1].doc.document_id)?.kategorie)}
          <tr class="kategoriegruppe"
            ><th colspan="8" scope="rowgroup"
              >{kategorieText(
                kategorien,
                zuordnung(doc.document_id)?.kategorie,
              )}</th
            ></tr
          >
        {/if}
        <tr>
          <td
            ><input
              type="checkbox"
              aria-label={`${doc.file_name} auswählen`}
              disabled={laeuft ||
                !darfAblegen ||
                fremdGesperrt(doc.document_id)}
              checked={auswahl.includes(doc.document_id)}
              onchange={(e) =>
                (auswahl = e.currentTarget.checked
                  ? [...auswahl, doc.document_id]
                  : auswahl.filter((id) => id !== doc.document_id))}
            /></td
          >
          <td>{doc.file_name}</td>
          <td class="kategoriespalte"
            >{kategorieText(
              kategorien,
              zuordnung(doc.document_id)?.kategorie,
            )}{#if darfAblegen && !fremdGesperrt(doc.document_id)}<button
                class="link"
                aria-label={`Kategorie ändern: ${doc.file_name}`}
                disabled={laeuft}
                onclick={() => kategorieBearbeiten([doc.document_id])}
                >Ändern</button
              >{/if}</td
          >
          <td>{doc.version}{anzahl > 1 ? ` (${anzahl} Versionen)` : ""}</td>
          <td>{groesse(doc.size_bytes)}</td>
          <td
            >{new Date(doc.created_at).toLocaleString("de-DE")} · {doc.created_by}</td
          >
          <td>
            {#if sperren.get(doc.document_id)}
              {@const s = sperren.get(doc.document_id)!}
              <span
                class="sperre"
                class:eigene={s.benutzer_id === benutzer.id}
                title={`seit ${new Date(s.seit).toLocaleString("de-DE")}`}
              >
                🔒 {s.benutzer_id === benutzer.id
                  ? "von Ihnen"
                  : s.benutzer_name}
              </span>
            {/if}
          </td>
          <td class="aktionen">
            <button
              class="link"
              disabled={laeuft}
              title="Im zugeordneten Programm öffnen"
              onclick={() => externOeffnen(doc)}>Öffnen</button
            >
            {#if /\.(eml|msg)$/i.test(doc.file_name)}<button
                class="link"
                disabled={laeuft}
                onclick={() => (email = doc)}>E-Mail lesen</button
              >{/if}
            {#if istPdf(doc) && darfAblegen && !fremdGesperrt(doc.document_id)}<button
                class="link"
                onclick={() => {
                  pdfDirty = false;
                  formular = doc;
                }}>Formular bearbeiten</button
              >{/if}
            <details class="dateiaktionen">
              <summary aria-label={`Weitere Aktionen für ${doc.file_name}`}
                >Weitere …</summary
              >
              <div>
                <Druckknopf
                  {projektId}
                  quelle={{
                    art: "dokument",
                    dokument_id: doc.document_id,
                    version: doc.version,
                    seiten: [],
                  }}
                  disabled={!/\.(pdf|eml|msg|png|jpg|jpeg)$/i.test(
                    doc.file_name,
                  )}
                />
                <button class="link" onclick={() => exportieren(doc)}
                  >Exportieren …</button
                >
                {#if istPlan(doc.file_name)}<button
                    class="link"
                    onclick={() => onplan(doc.document_id)}>Plan ansehen</button
                  >{/if}
                {#if darfAblegen && !fremdGesperrt(doc.document_id)}<button
                    class="link"
                    onclick={() => importieren(doc.document_id)}
                    >Neue Version …</button
                  >{/if}
                <button
                  class="link"
                  onclick={() => {
                    metaDirty = false;
                    metadaten = doc;
                  }}>Dokumentangaben …</button
                >
                <button class="link" onclick={() => (historie = doc)}
                  >Historie …</button
                >
                {#if istPdf(doc) && darfAblegen}<button
                    class="link"
                    disabled={laeuft}
                    onclick={() => inBibliothek(doc)}
                    >In Formularbibliothek ablegen …</button
                  >{/if}
                {#if !sperren.get(doc.document_id)}{#if darfAblegen}<button
                      class="link"
                      onclick={() => sperrenUmschalten(doc)}>Sperren</button
                    >{/if}{:else if !fremdGesperrt(doc.document_id) || darfFremdeAufheben}<button
                    class="link"
                    onclick={() => sperrenUmschalten(doc)}>Entsperren</button
                  >{/if}
              </div>
            </details>
          </td>
        </tr>
      {:else}
        <tr
          ><td colspan="8" class="meta"
            >{versionen.length
              ? "Keine Dateien für die gewählte Suche oder Kategorie."
              : "Noch keine Projektdateien. Über „Dateien importieren“ hinzufügen."}</td
          ></tr
        >
      {/each}
    </tbody>
  </table>
</div>

{#if kategorieDialog}<Dialog
    titel="Kategorie zuordnen"
    schliessenErlaubt={!laeuft}
    onclose={() => (kategorieDialog = false)}
  >
    <form onsubmit={kategorisieren}>
      {#if kategorieFehler}<p class="fehlertext" role="alert">
          {kategorieFehler}
        </p>{/if}
      <p>
        {auswahl.length} ausgewählte Unterlage(n). Die Zuordnung gilt auch für künftige
        Dateiversionen.
      </p>
      <KategorieAuswahl
        {kategorien}
        bind:wert={neueKategorie}
        disabled={laeuft}
      />
      <div class="knoepfe">
        <button disabled={laeuft || !auswahl.length}>Zuordnung speichern</button
        ><button
          type="button"
          class="sekundaer"
          disabled={laeuft}
          onclick={() => (kategorieDialog = false)}>Abbrechen</button
        >
      </div>
    </form>
  </Dialog>{/if}

{#if email}<Dialog
    titel="Archivierte E-Mail"
    breit
    onclose={() => (email = null)}
    ><EmailAnsicht
      dokument={email}
      darf={darfAblegen}
      onimport={() => void laden()}
      onschliessen={() => (email = null)}
    /></Dialog
  >{/if}

{#if metadaten}<Dialog
    titel="Dokumentangaben"
    schliessenErlaubt={!metaBusy}
    onrequestclose={metaSchliessen}
    onclose={() => (metadaten = null)}
    ><Dokumentdaten
      dokument={metadaten}
      benutzerId={benutzer.id}
      darf={darfAblegen}
      onschliessen={metaAbbrechen}
      ongeaendert={laden}
      onstatus={(dirty, busy) => {
        metaDirty = dirty;
        metaBusy = busy;
      }}
    /></Dialog
  >{/if}

{#if historie}
  <Dialog
    titel={`Historie: ${historie.file_name}`}
    onclose={() => (historie = null)}
  >
    <Historie
      dokument={historie}
      darfWiederherstellen={darfAblegen && !fremdGesperrt(historie.document_id)}
      onwiederhergestellt={async (v) => {
        historie = null;
        await laden();
        hinweis = `Version ${v.version} von ${v.file_name} wurde aus einer älteren Version erstellt.`;
      }}
      onfehler={(e) => ablegeFehler(e, historie?.file_name ?? "Dokument")}
      onschliessen={() => (historie = null)}
    />
  </Dialog>
{/if}

{#if formular}
  <Dialog
    titel="Formular ausfüllen"
    schliessenErlaubt={!pdfBusy}
    onrequestclose={pdfSchliessenErlaubt}
    onclose={() => (formular = null)}
  >
    <FormularAusfuellen
      dokumentId={formular.document_id}
      onfertig={formularFertig}
      onabbrechen={pdfAbbrechen}
      onstatus={(dirty, busy) => {
        pdfDirty = dirty;
        pdfBusy = busy;
      }}
    />
  </Dialog>
{/if}

<Ausgabeauftraege {projektId} darf={darfAblegen} stand={versionen.length} />
<Arbeitskopien
  bind:this={arbeitsEditor}
  {projektId}
  {darfAblegen}
  onaktualisiert={laden}
  {onfehler}
/>

<style>
  .kategoriespalte .link {
    margin-inline-start: 0.5rem;
  }
  .kategoriegruppe th {
    background: var(--panel);
    padding-top: 1rem;
  }
  .dateisuche {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    max-width: 40rem;
    margin: 1rem 0;
  }
  .dateisuche input {
    flex: 1;
    min-width: 0;
  }
  .dateiaktionen summary {
    cursor: pointer;
    color: var(--accent);
    font-size: 0.85rem;
  }
  .dateiaktionen > div {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.7rem;
    border: 1px solid var(--border);
    background: var(--panel);
    min-width: 12rem;
  }
</style>
