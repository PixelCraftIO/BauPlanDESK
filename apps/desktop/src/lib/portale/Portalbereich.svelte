<script lang="ts">
  import { datumDe } from "../api";
  import { untrack } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    mindestens,
    type Project,
    type Benutzer,
    type DocumentVersion,
  } from "../api";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import { verfahrenApi, heute } from "../verfahren/api";
  import { editorAnmelden } from "../editoren";
  import { projektNavigation } from "../projektNavigation.svelte";
  import { xbau } from "../xbau/api";
  import { portalApi } from "./api";
  import type {
    Fachstand,
    Verfahrensakte,
    Portalprofil,
    Portalabgabewunsch,
    Portalvorbereitung,
    Portalereigniswunsch,
    Portalrueckmeldungswunsch,
    Dokumentbezug,
    DateiZuordnung,
    Portalabgabeart,
    Nachrichtenstand,
    Dateiabgleich,
    Portalereignisart,
  } from "../generated/plattform";
  let { projekt, benutzer }: { projekt: Project; benutzer: Benutzer } =
    $props();
  const darf = $derived(
    benutzer.aktiv && mindestens(benutzer.rolle, "bearbeitung"),
  );
  const leitung = $derived(
    benutzer.aktiv && mindestens(benutzer.rolle, "projektleitung"),
  );
  const navigation = projektNavigation();
  const key = untrack(() => `bauprojekt.portal.${benutzer.id}.${projekt.id}`);
  const ereignisLeer = () => ({
    id: "",
    art: "abgabe_belegt" as Portalereignisart,
    datum: heute(),
    referenz: "",
    notiz: "",
    beleg: "",
    abgleich: "ungeklaert" as Dateiabgleich,
    bestaetigt: [] as string[],
    berichtigt: "",
  });
  const rueckLeer = () => ({
    id: "",
    betreff: "",
    datum: heute(),
    referenz: "",
    inhalt: "",
    original: "",
    abgabe: "",
    xbau: "",
  });
  function neu(a: Verfahrensakte) {
    return {
      vorgang: a.portal?.vorgaenge[0]?.id ?? "",
      profil:
        projekt.bundesland === "NW" ? "nrw-manuell@1" : "allgemein-manuell@1",
      kommune: "",
      url: projekt.bundesland === "NW" ? "https://bauportal.nrw/" : "",
      zielBestaetigt: false,
      vorgangId: "",
      titel: "Portalunterlagen",
      dateiKeys: [] as string[],
      mitXbau: false,
      xbauBezug: "",
      freezeId: "",
      freezeFingerprint: "",
      abgabe: "",
      art: "erstantrag" as Portalabgabeart,
      vorgaenger: "",
      zuordnungen: [] as DateiZuordnung[],
      forderungen: [] as string[],
      ereignis: ereignisLeer(),
      belegEntwuerfe: {} as Record<string, ReturnType<typeof ereignisLeer>>,
      rueck: rueckLeer(),
    };
  }
  type Entwurf = ReturnType<typeof neu>;
  function ladenEntwuerfe(): Record<string, Entwurf> {
    try {
      const wert: unknown = JSON.parse(localStorage.getItem(key) ?? "{}");
      if (!wert || typeof wert !== "object" || Array.isArray(wert)) return {};
      return Object.fromEntries(
        Object.entries(wert).filter(
          ([, d]) =>
            d &&
            typeof d === "object" &&
            Array.isArray(d.dateiKeys) &&
            Array.isArray(d.zuordnungen) &&
            Array.isArray(d.forderungen) &&
            d.ereignis &&
            Array.isArray(d.ereignis.bestaetigt) &&
            d.rueck,
        ),
      );
    } catch {
      return {};
    }
  }
  let entwuerfe = $state<Record<string, Entwurf>>(ladenEntwuerfe());
  let vid = $state("");
  let akten = $state<Datensatz<Verfahrensakte>[]>([]),
    dateien = $state<DocumentVersion[]>([]),
    profile = $state<Portalprofil[]>([]),
    nachrichten = $state<Nachrichtenstand[]>([]);
  let laeuft = $state(false),
    geladen = $state(false),
    fehler = $state(""),
    hinweis = $state(""),
    lokalFehler = $state(""),
    stufe = $state("ziel");
  let pruefung = $state<{
    key: string;
    vorgaenger: string;
    wert: Portalvorbereitung;
  } | null>(null);
  const akt = $derived(akten.find((a) => a.objekt === vid));
  const entwurf = $derived(entwuerfe[vid]);
  const vorgang = $derived(
    akt?.daten.portal?.vorgaenge.find((v) => v.id === entwurf?.vorgang),
  );
  const profil = $derived(
    profile.find((p) => `${p.id}@${p.version}` === entwurf?.profil),
  );
  const abgabe = $derived(
    akt?.daten.abgaben.find((b) => b.id === entwurf?.abgabe),
  );
  const portalabgabe = $derived(
    vorgang?.abgaben.find((b) => b.id === entwurf?.abgabe),
  );
  const vorher = $derived(
    vorgang?.abgaben.find((b) => b.id === entwurf?.vorgaenger),
  );
  const wunsch = $derived.by((): Portalabgabewunsch | null =>
    !entwurf?.abgabe
      ? null
      : {
          abgabe: entwurf.abgabe,
          art: entwurf.art,
          vorgaenger: entwurf.vorgaenger || null,
          dateien: entwurf.zuordnungen,
          forderungen: entwurf.forderungen.map((k) => {
            const [nachforderung, punkt] = k.split(":");
            return { nachforderung, punkt };
          }),
        },
  );
  const pruefKey = $derived(JSON.stringify([vid, entwurf?.vorgang, wunsch]));
  const referenzdateien = $derived(
    pruefung?.vorgaenger === entwurf?.vorgaenger
      ? pruefung.wert.referenzdateien
      : (vorher?.dateien ?? []),
  );
  const vergleichNamen = {
    neu: "Neu",
    identisch: "Inhalt unverändert",
    geaendert: "Neue Version",
    aelter: "Älterer Stand",
    anderes_dokument: "Anderes Dokument ersetzt Vorgänger",
  };
  const ereignisNamen: Record<Portalereignisart, string> = {
    abgabe_belegt: "Verbindliche Portalabgabe belegt",
    eingang_belegt: "Eingang belegt",
    zurueckgewiesen: "Zurückweisung belegt",
    ausgang_unklar: "Ausgang unklar",
  };
  const versandNamen: Record<string, string> = {
    versendet: "Abgabe belegt",
    eingegangen: "Eingang belegt",
    zurueckgewiesen: "Zurückgewiesen",
    ungeklaert: "Übergabe / Dateiabgleich ungeklärt",
  };
  function belegHatEingaben(e: ReturnType<typeof ereignisLeer>) {
    return !!(e.referenz || e.notiz || e.beleg || e.bestaetigt.length);
  }
  function abgabeWaehlen(id: string) {
    const d = entwuerfe[vid];
    if (!d || d.abgabe === id) return;
    d.belegEntwuerfe ??= {};
    if (d.abgabe) d.belegEntwuerfe[d.abgabe] = d.ereignis;
    d.abgabe = id;
    d.ereignis = d.belegEntwuerfe[id] ?? ereignisLeer();
  }
  function hatEingaben(d: Entwurf, id: string) {
    return !!(
      d.kommune ||
      d.dateiKeys.length ||
      (d.abgabe &&
        !akten
          .find((a) => a.objekt === id)
          ?.daten.portal?.vorgaenge.some((v) =>
            v.abgaben.some((b) => b.id === d.abgabe),
          )) ||
      belegHatEingaben(d.ereignis) ||
      Object.entries(d.belegEntwuerfe ?? {}).some(
        ([k, e]) => k !== d.abgabe && belegHatEingaben(e),
      ) ||
      d.rueck.betreff ||
      d.rueck.original ||
      d.rueck.inhalt
    );
  }
  const geaendert = $derived(
    darf && Object.entries(entwuerfe).some(([id, d]) => hatEingaben(d, id)),
  );
  $effect(() => {
    try {
      localStorage.setItem(key, JSON.stringify(entwuerfe));
      lokalFehler = "";
    } catch {
      lokalFehler =
        "Der Entwurf bleibt in der Sitzung erhalten, konnte aber nicht auf diesem Gerät gesichert werden.";
    }
  });
  editorAnmelden("portale", {
    hatAenderungen: () => geaendert,
    istBeschaeftigt: () => laeuft,
    speichern: async () => {
      fehler =
        "Portalentwürfe bitte im jeweiligen Arbeitsschritt abschließen oder verwerfen.";
      return false;
    },
    hinweis:
      "Den Portalentwurf im jeweiligen Arbeitsschritt abschließen oder verwerfen.",
  });
  let navNummer = 0;
  let pendingNavigation:
    | import("../generated/plattform").UebersichtsZiel
    | null = null;
  function sprung(z: import("../generated/plattform").UebersichtsZiel) {
    if (laeuft) {
      pendingNavigation = z;
      return;
    }
    const ziel = z.objekt ?? vid;
    if (!entwuerfe[ziel]) {
      if (geladen) {
        fehler = "Die angeforderte Verfahrensakte ist nicht verfügbar.";
        pendingNavigation = null;
      } else pendingNavigation = z;
      return;
    }
    if (vid !== ziel || entwuerfe[ziel].vorgang !== z.register) {
      hinweis = "";
      fehler = "";
    }
    vid = ziel;
    if (z.register) entwuerfe[vid].vorgang = z.register;
    if (z.eintrag) abgabeWaehlen(z.eintrag);
    stufe = "uebergabe";
    pendingNavigation = null;
  }
  $effect(() => {
    if (!laeuft)
      untrack(() => {
        if (pendingNavigation) sprung(pendingNavigation);
      });
  });
  $effect(() => {
    const a = navigation?.auftrag;
    if (!a || a.nummer === navNummer || a.ziel.bereich !== "portale") return;
    navNummer = a.nummer;
    untrack(() => sprung(a.ziel));
  });
  function dk(d: Dokumentbezug) {
    return `${d.dokument_id}:${d.version}`;
  }
  function dateikey(d: DocumentVersion) {
    return `${d.document_id}:${d.version}`;
  }
  function bezug(k: string): Dokumentbezug | null {
    const d = dateien.find((d) => dateikey(d) === k);
    return d ? { dokument_id: d.document_id, version: d.version } : null;
  }
  function uebernehmen(f: Fachstand) {
    akten = akten.map((a) =>
      a.objekt === f.objekt
        ? { ...f, daten: f.daten as unknown as Verfahrensakte }
        : a,
    );
  }
  async function laden() {
    laeuft = true;
    fehler = "";
    try {
      const [a, d, p, n] = await Promise.all([
        plattformApi.liste<Verfahrensakte>(projekt.id, "verfahren"),
        api.dokumente(projekt.id),
        portalApi.profile(),
        xbau.nachrichten(projekt.id),
      ]);
      akten = a;
      dateien = d;
      profile = p;
      nachrichten = n;
      for (const f of a) {
        if (!entwuerfe[f.objekt]) entwuerfe[f.objekt] = neu(f.daten);
      }
      if (a.some((a) => a.objekt === navigation?.verfahren))
        vid = navigation!.verfahren!;
      else if (!vid || !a.some((a) => a.objekt === vid))
        vid = a[0]?.objekt ?? "";
      geladen = true;
      if (pendingNavigation) sprung(pendingNavigation);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  $effect(() => {
    void projekt.id;
    untrack(laden);
  });
  function oeffnen(register: string) {
    if (!vid) return;
    navigation?.springen({
      bereich: "verfahren",
      objekt: vid,
      register,
      eintrag: null,
    });
  }
  function profilWaehlen() {
    if (entwurf) entwurf.url = profil?.portal_url ?? "";
  }
  async function vorgangAnlegen() {
    if (!akt || !entwurf || !profil) return;
    laeuft = true;
    fehler = "";
    try {
      if (!entwurf.vorgangId) entwurf.vorgangId = crypto.randomUUID();
      const f = await portalApi.vorgang(projekt.id, vid, akt.revision, {
        id: entwurf.vorgangId,
        profil_id: profil.id,
        profil_version: profil.version,
        ziel: {
          kommune: entwurf.kommune,
          stelle: akt.daten.behoerde.name,
          behoerdenkennung: akt.daten.behoerde.kennung,
          url: entwurf.url,
          zuordnung_bestaetigt: entwurf.zielBestaetigt,
        },
      });
      uebernehmen(f);
      entwurf.vorgang = entwurf.vorgangId;
      entwurf.vorgangId = "";
      entwurf.kommune = "";
      entwurf.zielBestaetigt = false;
      stufe = "unterlagen";
      hinweis =
        "Portalziel in der Verfahrensakte gespeichert. Keine Verbindung oder Antragstellung ausgelöst.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  function zuordnen() {
    if (!entwurf) return;
    const b = akt?.daten.abgaben.find((b) => b.id === entwurf.abgabe);
    entwurf.zuordnungen =
      b?.dokumente.map((d, i) => ({
        dokument: d.dokument,
        uploadname: `${String(i + 1).padStart(3, "0")}-${d.name}`,
        unterlagenart:
          vorgang?.profil.unterlagenarten.find((u) =>
            u.kategorien.some(
              (k) => k === d.kategorie?.id || k === d.kategorie?.hauptkategorie,
            ),
          )?.code ?? null,
        ersetzt: null,
      })) ?? [];
    pruefung = null;
  }
  async function einfrieren() {
    if (!akt || !entwurf) return;
    laeuft = true;
    fehler = "";
    try {
      const dokumente = entwurf.dateiKeys
        .map((k) => bezug(k))
        .filter((d): d is Dokumentbezug => !!d);
      if (!dokumente.length)
        throw new Error("Konkrete Dokumentversionen auswählen.");
      const finger = JSON.stringify([
        entwurf.titel,
        dokumente,
        entwurf.mitXbau,
        entwurf.xbauBezug,
      ]);
      if (finger !== entwurf.freezeFingerprint) {
        entwurf.freezeFingerprint = finger;
        entwurf.freezeId = crypto.randomUUID();
      }
      const f = await verfahrenApi.abgabe(projekt.id, vid, akt.revision, {
        id: entwurf.freezeId,
        titel: entwurf.titel,
        dokumente,
        xbau: entwurf.mitXbau,
        xbau_bezug: entwurf.mitXbau
          ? entwurf.xbauBezug || undefined
          : undefined,
        druck: false,
        anschreiben: "",
      });
      uebernehmen(f);
      abgabeWaehlen(entwurf.freezeId);
      entwurf.dateiKeys = [];
      entwurf.freezeId = "";
      entwurf.freezeFingerprint = "";
      zuordnen();
      hinweis =
        "Abgabestand eingefroren. Jetzt die Dateizuordnung prüfen und die Portalbereitstellung festhalten.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function pruefen() {
    if (!entwurf || !wunsch) return;
    const k = pruefKey;
    laeuft = true;
    fehler = "";
    try {
      const r = await portalApi.pruefen(
        projekt.id,
        vid,
        entwurf.vorgang,
        $state.snapshot(wunsch),
      );
      pruefung = { key: k, vorgaenger: entwurf.vorgaenger, wert: r };
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function festhalten() {
    if (!akt || !entwurf || !wunsch) return;
    laeuft = true;
    fehler = "";
    try {
      uebernehmen(
        await portalApi.festhalten(
          projekt.id,
          vid,
          entwurf.vorgang,
          akt.revision,
          $state.snapshot(wunsch),
        ),
      );
      entwurf.zuordnungen = [];
      pruefung = null;
      stufe = "uebergabe";
      hinweis =
        "Portalbereitstellung unveränderlich gespeichert. Es wurde nichts übermittelt.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function exportieren(d: Dokumentbezug | null = null) {
    if (!entwurf || !portalabgabe || laeuft) return;
    laeuft = true;
    fehler = "";
    try {
      const name = d
        ? portalabgabe.dateien.find((f) => dk(f.original.dokument) === dk(d))
            ?.uploadname
        : "Portalunterlagen.zip";
      const ziel = await save({
        title: d
          ? "Originaldatei für das Portal bereitstellen"
          : "Lokale Portalbereitstellung exportieren",
        defaultPath: name,
      });
      if (!ziel) return;
      if (d)
        await portalApi.datei(
          projekt.id,
          vid,
          entwurf.vorgang,
          portalabgabe.id,
          d,
          ziel,
        );
      else
        await portalApi.paket(
          projekt.id,
          vid,
          entwurf.vorgang,
          portalabgabe.id,
          ziel,
        );
      hinweis =
        "Unveränderte Dateien bereitgestellt. Export bestätigt keine Portalabgabe.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function portalOeffnen() {
    if (!entwurf) return;
    laeuft = true;
    fehler = "";
    try {
      await portalApi.oeffnen(projekt.id, vid, entwurf.vorgang);
      hinweis =
        "Portal im Systembrowser angefordert. Dort anmelden und den Antrag bearbeiten. Kein Versandstatus geändert.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function importieren(art: "beleg" | "rueck") {
    if (!entwurf || laeuft) return;
    laeuft = true;
    fehler = "";
    try {
      const pfad = await open({
        title: "Original im Projekt ablegen",
        multiple: false,
      });
      if (typeof pfad !== "string") return;
      const d = await api.dokumentImportieren(projekt.id, pfad);
      dateien = [...dateien, d];
      if (art === "beleg") entwurf.ereignis.beleg = dateikey(d);
      else entwurf.rueck.original = dateikey(d);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  function alleDateien(b: boolean) {
    if (entwurf)
      entwurf.ereignis.bestaetigt = b
        ? (portalabgabe?.dateien.map((d) => dk(d.original.dokument)) ?? [])
        : [];
  }
  async function ereignisSpeichern() {
    if (!akt || !entwurf || !portalabgabe) return;
    laeuft = true;
    fehler = "";
    try {
      const e = entwurf.ereignis;
      if (!e.id) e.id = crypto.randomUUID();
      const w: Portalereigniswunsch = {
        id: e.id,
        art: e.art,
        datum: e.datum,
        referenz: e.referenz,
        notiz: e.notiz,
        beleg: bezug(e.beleg),
        dateiabgleich: e.abgleich,
        bestaetigte_dateien: e.bestaetigt
          .map((k) => bezug(k))
          .filter((d): d is Dokumentbezug => !!d),
        berichtigt: e.berichtigt || null,
      };
      uebernehmen(
        await portalApi.ereignis(
          projekt.id,
          vid,
          entwurf.vorgang,
          portalabgabe.id,
          akt.revision,
          w,
        ),
      );
      entwurf.ereignis = ereignisLeer();
      hinweis =
        "Portalbeleg und Verfahrensstand gemeinsam gespeichert. Nur vollständig abgeglichene Dateilisten gelten als belegter Dateistand.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  function xbauWaehlen() {
    if (!entwurf) return;
    const n = nachrichten.find((n) => n.id === entwurf.rueck.xbau);
    if (n) {
      entwurf.rueck.original = dk(n.nachricht.original);
      if (!entwurf.rueck.betreff)
        entwurf.rueck.betreff = n.nachricht.pruefung.kopf.titel;
    }
  }
  async function rueckmeldungSpeichern() {
    if (!akt || !entwurf) return;
    laeuft = true;
    fehler = "";
    try {
      const r = entwurf.rueck;
      const original = bezug(r.original);
      if (!original) throw new Error("Original der Rückmeldung auswählen.");
      if (!r.id) r.id = crypto.randomUUID();
      const w: Portalrueckmeldungswunsch = {
        id: r.id,
        betreff: r.betreff,
        datum: r.datum,
        referenz: r.referenz,
        original,
        inhalt: r.inhalt,
        abgabe: r.abgabe || null,
        xbau_nachricht: r.xbau || null,
      };
      uebernehmen(
        await portalApi.rueckmeldung(
          projekt.id,
          vid,
          entwurf.vorgang,
          akt.revision,
          w,
        ),
      );
      entwurf.rueck = rueckLeer();
      hinweis =
        "Rückmeldung mit Originalbezug als Kommunikation gespeichert. Fristen, Forderungspunkte und Bescheidwirkung in der Verfahrensakte prüfen.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function xbauArchivieren() {
    if (!entwurf || !portalabgabe) return;
    laeuft = true;
    fehler = "";
    try {
      const n = await portalApi.xbau(
        projekt.id,
        vid,
        entwurf.vorgang,
        portalabgabe.id,
      );
      nachrichten = [...nachrichten.filter((x) => x.id !== n.id), n];
      hinweis =
        "Das unveränderte eingefrorene XBau-Artefakt ist im Nachrichtenjournal erfasst. Dies bestätigt keinen Portalimport.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function profilImportieren(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    laeuft = true;
    fehler = "";
    try {
      if (file.size > 256 * 1024)
        throw new Error("Portalprofil höchstens 256 KiB.");
      profile = await portalApi.profilHinzufuegen(
        JSON.parse(await file.text()),
      );
      hinweis =
        "Neue Profilversion im Büro hinterlegt. Bereits festgehaltene Portalvorgänge behalten ihren bisherigen Profilstand.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
      input.value = "";
    }
  }
</script>

<div class="portalbereich" data-portale>
  <div class="kopf">
    <p>
      Einreichungspakete und Portalbelege gehören zur ausgewählten
      Verfahrensakte. Die Anmeldung und Antragstellung erfolgen im
      Systembrowser.
    </p>
    <button class="sekundaer" onclick={laden} disabled={laeuft}
      >Aktuellen Stand laden</button
    >
  </div>
  {#if fehler}<p class="fehlertext" role="alert">
      {fehler}
    </p>{/if}{#if hinweis}<p class="hinweisbox" role="status">
      {hinweis}
    </p>{/if}{#if lokalFehler}<p class="fehlertext">
      {lokalFehler}
    </p>{/if}{#if laeuft}<p role="status">
      Portalvorgang wird bearbeitet …
    </p>{/if}
  {#if geladen && akten.length === 0}<p>
      Für die Portalunterstützung zuerst eine Verfahrensakte mit der zuständigen
      Behörde anlegen.
    </p>
    <button
      onclick={() =>
        navigation?.springen({
          bereich: "verfahren",
          objekt: null,
          register: null,
          eintrag: null,
        })}>Verfahrensakte öffnen</button
    >{/if}
  {#if akt && entwurf}
    <div class="raster">
      <label
        >Verfahrensakte<select
          aria-label="Verfahrensakte"
          onchange={() => {
            if (navigation) navigation.verfahren = vid || null;
          }}
          bind:value={vid}
          disabled={laeuft}
          >{#each akten as a}<option value={a.objekt}
              >{a.daten.bezeichnung} · {a.daten.behoerde.name}</option
            >{/each}</select
        ></label
      ><label
        >Portalvorgang<select
          aria-label="Portalvorgang"
          bind:value={entwurf.vorgang}
          disabled={laeuft}
          onchange={() => {
            abgabeWaehlen("");
            stufe = entwurf.vorgang ? "unterlagen" : "ziel";
          }}
          ><option value="">Neuen Portalvorgang anlegen</option
          >{#each akt.daten.portal?.vorgaenge ?? [] as v}<option value={v.id}
              >{v.ziel.kommune} · {v.profil.titel} · {v.profil.version}</option
            >{/each}</select
        ></label
      >
    </div>
    <nav class="schritte" aria-label="Schritte der Portalunterstützung">
      {#each [["ziel", "1 · Ziel"], ["unterlagen", "2 · Unterlagen"], ["uebergabe", "3 · Übergabe / Beleg"], ["rueckmeldungen", "4 · Rückmeldungen"]] as [id, titel]}<button
          class="sekundaer"
          disabled={laeuft}
          aria-pressed={stufe === id}
          onclick={() => (stufe = id)}>{titel}</button
        >{/each}
    </nav>
    {#if stufe === "ziel"}
      {#if vorgang}<fieldset>
          <legend>Festgehaltenes Portalziel</legend>
          <dl>
            <dt>Kommune</dt>
            <dd>{vorgang.ziel.kommune}</dd>
            <dt>Stelle</dt>
            <dd>{vorgang.ziel.stelle}</dd>
            <dt>Portal</dt>
            <dd>{vorgang.ziel.url}</dd>
            <dt>Profil</dt>
            <dd>{vorgang.profil.titel} · {vorgang.profil.version}</dd>
            <dt>Profilstand</dt>
            <dd>
              {vorgang.profil.freigabe === "fachlich_geprueft"
                ? "Im Büro fachlich geprüft"
                : "Annahmeregeln nicht bestätigt"}
            </dd>
            <dt>Quelle</dt>
            <dd>{vorgang.profil.quelle}</dd>
          </dl>
          <ul>
            {#each vorgang.profil.hinweise as h}<li>{h}</li>{/each}
          </ul>
          <button onclick={() => (stufe = "unterlagen")}
            >Unterlagen zuordnen</button
          >
        </fieldset>
      {:else}<fieldset disabled={!darf || laeuft}>
          <legend>Portalziel und Profil</legend>
          <p>
            Behörde: <strong>{akt.daten.behoerde.name}</strong> · {akt.daten
              .behoerde.kennung || "Keine technische Behördenkennung erfasst"}
          </p>
          <div class="raster">
            <label
              >Profil<select
                aria-label="Profil"
                bind:value={entwurf.profil}
                onchange={profilWaehlen}
                >{#each profile.filter((p) => p.bundesland === null || p.bundesland === projekt.bundesland) as p}<option
                    value={`${p.id}@${p.version}`}
                    >{p.titel} · {p.version}</option
                  >{/each}</select
              ></label
            ><label
              >Kommune<input
                aria-label="Kommune"
                bind:value={entwurf.kommune}
                maxlength="500"
              /></label
            ><label
              >Portaladresse<input
                aria-label="Portaladresse"
                type="url"
                bind:value={entwurf.url}
                placeholder="https://…"
                maxlength="2000"
              /></label
            >
          </div>
          {#if profil}<div class="hinweisbox">
              <strong
                >{profil.freigabe === "fachlich_geprueft"
                  ? "Büroprofil fachlich geprüft"
                  : "Portalannahme nicht bestätigt"}</strong
              >
              <ul>
                {#each profil.hinweise as h}<li>{h}</li>{/each}
              </ul>
            </div>{/if}<label class="check"
            ><input type="checkbox" bind:checked={entwurf.zielBestaetigt} /> Ich
            habe Zuständigkeit und Portalzugang für dieses Verfahren geprüft.</label
          ><button onclick={vorgangAnlegen} disabled={!entwurf.zielBestaetigt}
            >Portalziel festhalten</button
          >
        </fieldset>{/if}
      {#if leitung}<details>
          <summary>Weitere Portalprofile verwalten</summary>
          <p>
            Eine einzelne Profilversion als JSON hinzufügen. Bestehende
            Versionen bleiben unverändert. Eine Bürofreigabe ist keine amtliche
            Schnittstellenfreigabe.
          </p>
          <label
            >Portalprofil importieren<input
              type="file"
              accept="application/json,.json"
              onchange={profilImportieren}
              disabled={laeuft}
            /></label
          >
        </details>{/if}
    {:else if !vorgang}<p>Bitte zuerst ein Portalziel festhalten.</p>
    {:else if stufe === "unterlagen"}
      <p class="hinweisbox">
        {vorgang.profil.freigabe === "fachlich_geprueft"
          ? "Prüfung nach dem eingefrorenen Büroprofil."
          : "Allgemeine Dateibereitstellung: Annahmeregeln und Importmöglichkeiten des Portals sind noch zu prüfen."}
        Es wird kein Antrag versendet.
      </p>
      <fieldset disabled={!darf || laeuft}>
        <legend>Neuen Abgabestand zusammenstellen</legend><label
          >Titel<input
            aria-label="Titel des Abgabestands"
            bind:value={entwurf.titel}
            maxlength="500"
          /></label
        ><label
          >Konkrete Dokumentversionen<select
            aria-label="Konkrete Dokumentversionen"
            multiple
            size="7"
            bind:value={entwurf.dateiKeys}
            >{#each dateien as d}<option value={dateikey(d)}
                >{d.file_name} · Version {d.version}</option
              >{/each}</select
          ></label
        ><label class="check"
          ><input type="checkbox" bind:checked={entwurf.mitXbau} /> Zusätzlich ein
          XBau-Artefakt einfrieren</label
        >{#if entwurf.mitXbau}<label
            >XBau-Ausgabe<select
              aria-label="XBau-Ausgabe"
              bind:value={entwurf.xbauBezug}
              ><option value="">Erstantrag 0200</option
              >{#each nachrichten.filter( (n) => ["baugenehmigung.antrag.0200", "prozessnachrichten.formellePruefungBefundliste.1140"].includes(n.nachricht.pruefung.kopf.element), ) as n}<option
                  value={n.id}
                  >Nachreichung 0202 zu {n.nachricht.pruefung.kopf.titel} · {n.id.slice(
                    0,
                    8,
                  )}</option
                >{/each}</select
            ></label
          >
          <p class="meta">
            XBau-Unterstützung belegt keine Importmöglichkeit des Portals.
            Ausgefüllte Formulare und Berechnungen vorher als Projektdateien
            ablegen.
          </p>{/if}<button
          onclick={einfrieren}
          disabled={!entwurf.dateiKeys.length}
          >Dateistand verbindlich festhalten</button
        >
      </fieldset>
      <fieldset disabled={laeuft}>
        <legend>Eingefrorenen Stand für das Portal zuordnen</legend><label
          >Abgabestand<select
            aria-label="Abgabestand"
            value={entwurf.abgabe}
            onchange={(e) => {
              abgabeWaehlen(e.currentTarget.value);
              zuordnen();
            }}
            ><option value="">Stand auswählen</option
            >{#each akt.daten.abgaben as b}<option value={b.id}
                >{b.titel} · {b.dokumente.length} Dateien · {b.erstellt_am.slice(
                  0,
                  10,
                )}</option
              >{/each}</select
          ></label
        >
        {#if portalabgabe}<p>
            Dieser Stand besitzt bereits eine unveränderliche
            Portalbereitstellung.
          </p>
          <button onclick={() => (stufe = "uebergabe")}
            >Zur Übergabe und den Belegen</button
          >
        {:else if abgabe}<div class="raster">
            <label
              >Einreichungsart<select
                aria-label="Einreichungsart"
                bind:value={entwurf.art}
                disabled={!darf}
                onchange={() => {
                  if (entwurf.art === "erstantrag") entwurf.vorgaenger = "";
                }}
                ><option value="erstantrag">Erstantrag</option><option
                  value="nachreichung">Nachreichung / Änderung</option
                ></select
              ></label
            >{#if entwurf.art === "nachreichung"}<label
                >Belegter Vorgänger<select
                  aria-label="Belegter Vorgänger"
                  bind:value={entwurf.vorgaenger}
                  disabled={!darf}
                  ><option value="">Vorgänger auswählen</option
                  >{#each vorgang.abgaben as b}<option value={b.id}
                      >{akt.daten.abgaben.find((a) => a.id === b.abgabe)?.titel}
                      · {versandNamen[
                        akt.daten.abgaben.find((a) => a.id === b.abgabe)
                          ?.versandstatus ?? ""
                      ] ?? "Noch nicht belegt"}</option
                    >{/each}</select
                ></label
              >{/if}
          </div>
          <div class="tabelle">
            <table class="ergebnistabelle">
              <thead
                ><tr
                  ><th>Original / Kategorie</th><th>Uploadname</th><th
                    >Portal-Unterlagenart</th
                  ><th>Ersetzt ausdrücklich</th></tr
                ></thead
              ><tbody
                >{#each entwurf.zuordnungen as z, i}{@const d =
                    abgabe.dokumente.find(
                      (d) => dk(d.dokument) === dk(z.dokument),
                    )}<tr
                    ><td
                      >{d?.name} · Version {z.dokument.version}<br /><span
                        class="meta"
                        >{d?.kategorie?.name ??
                          "Kategorie damals nicht erfasst"}</span
                      ></td
                    ><td
                      ><input
                        aria-label={`Uploadname ${i + 1}`}
                        bind:value={z.uploadname}
                        disabled={!darf}
                      /></td
                    ><td
                      >{#if vorgang.profil.unterlagenarten.length}<select
                          aria-label={`Unterlagenart ${i + 1}`}
                          bind:value={z.unterlagenart}
                          disabled={!darf}
                          ><option value={null}>Bitte zuordnen</option
                          >{#each vorgang.profil.unterlagenarten as u}<option
                              value={u.code}>{u.name}</option
                            >{/each}</select
                        >{:else}<span class="meta">Nicht profiliert</span
                        >{/if}</td
                    ><td
                      ><select
                        aria-label={`Ersetzt ${i + 1}`}
                        value={z.ersetzt ? dk(z.ersetzt) : ""}
                        disabled={!darf || !vorher}
                        onchange={(e) =>
                          (z.ersetzt =
                            referenzdateien.find(
                              (d) =>
                                dk(d.original.dokument) ===
                                e.currentTarget.value,
                            )?.original.dokument ?? null)}
                        ><option value=""
                          >Automatischer Versionsvergleich</option
                        >{#each referenzdateien as d}<option
                            value={dk(d.original.dokument)}
                            >{d.original.name} · Version {d.original.dokument
                              .version}</option
                          >{/each}</select
                      ></td
                    ></tr
                  >{/each}</tbody
              >
            </table>
          </div>
          {#if akt.daten.nachforderungen.length}<details>
              <summary>Bezug zu Nachforderungspunkten</summary
              >{#each akt.daten.nachforderungen as n}{#each n.punkte as p}<label
                    class="check"
                    ><input
                      type="checkbox"
                      value={`${n.id}:${p.id}`}
                      bind:group={entwurf.forderungen}
                      disabled={!darf}
                    />{p.titel}</label
                  >{/each}{/each}
              <p class="meta">
                Die Auswahl dokumentiert den Bezug. Den Antwortstatus und die
                vollständigen Antwortunterlagen in der Verfahrensakte
                bearbeiten.
              </p>
            </details>{/if}
          <button onclick={pruefen}>Dateizuordnung prüfen</button
          >{#if pruefung?.key === pruefKey}<div role="status">
              <h3>Ergebnis der Profilprüfung</h3>
              {#if pruefung.wert.pruefung.fehler.length}<ul class="fehlertext">
                  {#each pruefung.wert.pruefung.fehler as h}<li>{h}</li>{/each}
                </ul>{/if}
              <ul>
                {#each pruefung.wert.pruefung.hinweise as h}<li>{h}</li>{/each}
              </ul>
              <ul>
                {#each pruefung.wert.dateien as d}<li>
                    {d.original.name}: {vergleichNamen[d.vergleich]}
                  </li>{/each}
              </ul>
            </div>
            <button
              onclick={festhalten}
              disabled={!darf || pruefung.wert.pruefung.fehler.length > 0}
              >Portalbereitstellung festhalten</button
            >{/if}{/if}
      </fieldset>
    {:else if stufe === "uebergabe"}
      <div class="raster">
        <label
          >Portalabgabe<select
            aria-label="Portalabgabe"
            value={entwurf.abgabe}
            onchange={(e) => abgabeWaehlen(e.currentTarget.value)}
            disabled={laeuft}
            ><option value="">Stand auswählen</option
            >{#each vorgang.abgaben as b}<option value={b.id}
                >{akt.daten.abgaben.find((a) => a.id === b.id)?.titel ??
                  b.id}</option
              >{/each}</select
          ></label
        ><button onclick={portalOeffnen} disabled={laeuft}
          >Portal im Systembrowser öffnen</button
        >
      </div>
      {#if portalabgabe}<fieldset disabled={laeuft}>
          <legend>Unveränderte Unterlagen bereitstellen</legend>
          <p>
            <strong
              >{versandNamen[abgabe?.versandstatus ?? ""] ??
                "Vorbereitet · keine Abgabe belegt"}</strong
            >
          </p>
          <p>
            Die lokale ZIP enthält Originaldateien und eine Dateiliste. Nur die
            vom Portal geforderten Dateien hochladen. Ein Portalentwurf oder
            Datei-Upload ist noch keine verbindliche Abgabe.
          </p>
          <div class="knoepfe links">
            <button onclick={() => exportieren()}
              >Bereitstellungs-ZIP exportieren …</button
            >{#if abgabe?.artefakte.some((a) => a.format === "xbau")}<button
                onclick={xbauArchivieren}
                disabled={!darf}>Eingefrorenes XBau im Journal erfassen</button
              >{/if}
          </div>
          <div class="tabelle">
            <table class="ergebnistabelle">
              <thead
                ><tr
                  ><th>Uploadname / Kategorie</th><th>Version</th><th
                    >Vergleich</th
                  ><th>Einzeldownload</th></tr
                ></thead
              ><tbody
                >{#each portalabgabe.dateien as d}<tr
                    ><td
                      >{d.uploadname}<br /><span class="meta"
                        >{d.original.kategorie?.name ??
                          "Kategorie nicht erfasst"}</span
                      ></td
                    ><td>{d.original.dokument.version}</td><td
                      >{vergleichNamen[d.vergleich]}</td
                    ><td
                      ><button
                        class="sekundaer"
                        onclick={() => exportieren(d.original.dokument)}
                        >Originaldatei exportieren …</button
                      ></td
                    ></tr
                  >{/each}</tbody
              >
            </table>
          </div>
        </fieldset>
        <fieldset disabled={!darf || laeuft}>
          <legend>Portalbeleg und Dateiabgleich erfassen</legend>
          <div class="raster">
            <label
              >Ereignis<select
                aria-label="Ereignis"
                bind:value={entwurf.ereignis.art}
                >{#each Object.entries(ereignisNamen) as [id, name]}<option
                    value={id}>{name}</option
                  >{/each}</select
              ></label
            ><label
              >Belegdatum<input
                aria-label="Belegdatum"
                type="date"
                bind:value={entwurf.ereignis.datum}
              /></label
            ><label
              >Portalreferenz<input
                aria-label="Portalreferenz"
                bind:value={entwurf.ereignis.referenz}
                maxlength="500"
              /></label
            ><label
              >Originalbeleg<select
                aria-label="Originalbeleg"
                bind:value={entwurf.ereignis.beleg}
                ><option value="">Beleg auswählen</option
                >{#each dateien as d}<option value={dateikey(d)}
                    >{d.file_name} · Version {d.version}</option
                  >{/each}</select
              ></label
            >
          </div>
          <button class="sekundaer" onclick={() => importieren("beleg")}
            >Belegdatei importieren …</button
          ><label
            >Dateiabgleich<select
              aria-label="Dateiabgleich"
              bind:value={entwurf.ereignis.abgleich}
              onchange={() => (entwurf.ereignis.bestaetigt = [])}
              ><option value="ungeklaert">Dateiliste noch ungeklärt</option
              ><option value="vollstaendig"
                >Vollständige Dateiliste abgeglichen</option
              ><option value="abweichend"
                >Abweichende / teilweise Dateiliste</option
              ></select
            ></label
          >
          {#if entwurf.ereignis.abgleich === "vollstaendig"}<label class="check"
              ><input
                type="checkbox"
                checked={entwurf.ereignis.bestaetigt.length ===
                  portalabgabe.dateien.length}
                onchange={(e) => alleDateien(e.currentTarget.checked)}
              /> Ich habe alle unten aufgeführten konkreten Dateiversionen mit der
              tatsächlichen Portalabgabe abgeglichen.</label
            >{/if}
          {#if entwurf.ereignis.abgleich !== "ungeklaert"}{#each portalabgabe.dateien as d}<label
                class="check"
                ><input
                  type="checkbox"
                  value={dk(d.original.dokument)}
                  bind:group={entwurf.ereignis.bestaetigt}
                />{d.uploadname} · Version {d.original.dokument.version}</label
              >{/each}{/if}
          <label
            >Bemerkung / Abweichung<textarea
              aria-label="Bemerkung / Abweichung"
              rows="3"
              bind:value={entwurf.ereignis.notiz}
              maxlength="3500"
            ></textarea></label
          ><label
            >Berichtigung von<select
              aria-label="Berichtigung von"
              bind:value={entwurf.ereignis.berichtigt}
              ><option value="">Neues Ereignis</option
              >{#each portalabgabe.ereignisse.filter((e) => !portalabgabe?.ereignisse.some((n) => n.berichtigt === e.id)) as e}<option
                  value={e.id}
                  >{datumDe(e.datum)} · {ereignisNamen[e.art]} · {e.referenz}</option
                >{/each}</select
            ></label
          >
          <div class="knoepfe links">
            <button onclick={ereignisSpeichern}
              >Beleg und Verfahrensstand speichern</button
            ><button
              class="sekundaer"
              onclick={() => (entwurf.ereignis = ereignisLeer())}
              >Belegentwurf verwerfen</button
            >
          </div>
        </fieldset>
        <h3>Ereignischronik</h3>
        {#each portalabgabe.ereignisse as e}<article class="chronik">
            <strong>{ereignisNamen[e.art]}</strong> · {datumDe(e.datum)} · {e.referenz ||
              "Keine Portalreferenz"}{#if portalabgabe.ereignisse.some((n) => n.berichtigt === e.id)}
              · berichtigt{/if}
            <p>{e.notiz}</p>
            <p class="meta">
              Manuell erfasst · Dateiabgleich: {e.dateiabgleich} · {e
                .bestaetigte_dateien.length} Versionen zugeordnet
            </p>
          </article>{:else}<p>
            Noch keine Portalabgabe oder Übermittlung belegt.
          </p>{/each}
      {:else}<p>Noch keine Portalbereitstellung ausgewählt.</p>{/if}
    {:else if stufe === "rueckmeldungen"}
      <fieldset disabled={!darf || laeuft}>
        <legend>Rückmeldung mit Original zuordnen</legend>
        <p>
          Eine Rückmeldung wird als Kommunikation gespeichert. Nachforderungen,
          Bescheide und rechtlich relevante Fristen anschließend anhand des
          Originals in der Verfahrensakte prüfen.
        </p>
        <div class="raster">
          <label
            >Betreff<input
              aria-label="Betreff"
              bind:value={entwurf.rueck.betreff}
              maxlength="500"
            /></label
          ><label
            >Datum<input
              aria-label="Rückmeldungsdatum"
              type="date"
              bind:value={entwurf.rueck.datum}
            /></label
          ><label
            >Externe Referenz<input
              aria-label="Externe Referenz"
              bind:value={entwurf.rueck.referenz}
              maxlength="500"
            /></label
          ><label
            >Bezug zur Abgabe<select
              aria-label="Bezug zur Abgabe"
              bind:value={entwurf.rueck.abgabe}
              ><option value="">Allgemeine Verfahrensrückmeldung</option
              >{#each vorgang.abgaben as b}<option value={b.id}
                  >{akt.daten.abgaben.find((a) => a.id === b.id)?.titel}</option
                >{/each}</select
            ></label
          ><label
            >Original der Rückmeldung<select
              aria-label="Original der Rückmeldung"
              bind:value={entwurf.rueck.original}
              ><option value="">Original auswählen</option
              >{#each dateien as d}<option value={dateikey(d)}
                  >{d.file_name} · Version {d.version}</option
                >{/each}</select
            ></label
          ><label
            >Optionaler XBau-Eingang<select
              aria-label="Optionaler XBau-Eingang"
              bind:value={entwurf.rueck.xbau}
              onchange={xbauWaehlen}
              ><option value="">Kein XBau-Bezug</option
              >{#each nachrichten.filter((n) => n.nachricht.richtung === "eingang") as n}<option
                  value={n.id}
                  >{n.nachricht.pruefung.kopf.titel} · {n.id.slice(
                    0,
                    8,
                  )}</option
                >{/each}</select
            ></label
          >
        </div>
        <button class="sekundaer" onclick={() => importieren("rueck")}
          >Originaldatei importieren …</button
        ><label
          >Inhalt / Einordnung<textarea
            aria-label="Inhalt / Einordnung"
            rows="5"
            bind:value={entwurf.rueck.inhalt}
            maxlength="60000"
          ></textarea></label
        >
        <div class="knoepfe links">
          <button onclick={rueckmeldungSpeichern}
            >Rückmeldung in der Verfahrensakte speichern</button
          ><button
            class="sekundaer"
            onclick={() => (entwurf.rueck = rueckLeer())}
            >Rückmeldungsentwurf verwerfen</button
          >
        </div>
      </fieldset>
      <div class="knoepfe links">
        <button class="sekundaer" onclick={() => oeffnen("kommunikation")}
          >Kommunikation öffnen</button
        ><button class="sekundaer" onclick={() => oeffnen("nachforderungen")}
          >Nachforderungen bearbeiten</button
        ><button class="sekundaer" onclick={() => oeffnen("bescheide")}
          >Bescheide / Auflagen bearbeiten</button
        >
      </div>
      {#each vorgang.rueckmeldungen as r}<article class="chronik">
          <strong>{r.betreff}</strong> · {datumDe(r.datum)} · {r.referenz}
          <p class="inhalt">{r.inhalt}</p>
        </article>{:else}<p>
          Noch keine Portalrückmeldungen zugeordnet.
        </p>{/each}
    {/if}
    <div class="entwurfende">
      <button
        class="sekundaer"
        onclick={() => {
          if (akt) entwuerfe[vid] = neu(akt.daten);
          pruefung = null;
        }}
        disabled={laeuft || !darf}
        >Ungespeicherten Portalentwurf dieses Verfahrens verwerfen</button
      >
    </div>
  {/if}
</div>

<style>
  .portalbereich {
    display: grid;
    gap: 1.2rem;
    max-width: 110rem;
  }
  .kopf {
    display: flex;
    gap: 1rem;
    align-items: center;
    justify-content: space-between;
  }
  fieldset {
    display: grid;
    gap: 1rem;
    min-width: 0;
    padding: 1.25rem;
  }
  label {
    display: grid;
    gap: 0.45rem;
  }
  input,
  select,
  textarea {
    width: 100%;
    min-width: 0;
  }
  .raster {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
    gap: 1rem;
    align-items: end;
  }
  .check {
    display: flex;
    gap: 0.6rem;
    align-items: center;
  }
  .check input {
    width: auto;
  }
  .schritte {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
  }
  .schritte button[aria-pressed="true"] {
    background: var(--accent-soft);
    border-color: var(--accent);
  }
  .tabelle {
    overflow-x: auto;
    min-width: 0;
  }
  .tabelle input,
  .tabelle select {
    min-width: 10rem;
  }
  .chronik {
    padding: 1rem;
    border-left: 3px solid var(--border);
    background: var(--panel);
  }
  .inhalt {
    white-space: pre-wrap;
  }
  p {
    margin: 0.3rem 0;
  }
  dl {
    display: grid;
    grid-template-columns: 10rem minmax(0, 1fr);
    gap: 0.6rem;
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  dt {
    color: var(--muted);
  }
  button {
    justify-self: start;
  }
  .entwurfende {
    border-top: 1px solid var(--border);
    padding-top: 1rem;
  }
  details {
    border: 1px solid var(--border);
    padding: 1rem;
    border-radius: 0.35rem;
  }
  @media (max-width: 700px) {
    .raster,
    dl {
      grid-template-columns: 1fr;
    }
    .kopf {
      display: grid;
    }
    fieldset {
      padding: 0.8rem;
    }
  }
</style>
