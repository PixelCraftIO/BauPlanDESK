// Test transport only: the production bundle uses the real Tauri commands.
const copy = (value) =>
  value === undefined ? undefined : JSON.parse(JSON.stringify(value));
const initial = window.fixture;
window.testProjects = [
  initial,
  {
    ...copy(initial),
    id: "8eab0c37-592e-4893-939f-bb365a89dd11",
    name: "Umbau Bürogebäude",
  },
];
// Bestehende Fachablauf-Suiten verwenden bewusst eine gespeicherte, vollständig
// geöffnete Navigation. Die eigene UI-Suite prüft den kompakten Standard separat.
for (const p of window.testProjects) localStorage.setItem(`bauprojekt.navigation.v1.${p.id}`, JSON.stringify({grundlagen:true,berechnungen:true,nutzung:true,flaechen:true,weitere:true,unterlagen:true,verfahren:true,einreichungswerkzeuge:true}));
window.calls = [];
window.failSave = false;
window.saveDelay = 0;
window.testUser = {
  id: "user",
  anmeldename: "test",
  anzeigename: "Anna Weber",
  rolle: "administrator",
  aktiv: true,
  erstellt_am: initial.created_at,
};
window.savedApplications = {};
const address = {
  strasse: "",
  hausnummer: "",
  postleitzahl: "",
  ort: "",
  staat: "DE",
};
window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
window.testCallbacks = {};
window.testEvents = {};
let callbackId = 0;
window.emitNative = async (event, payload = null) => {
  for (const id of window.testEvents[event] ?? [])
    await window.testCallbacks[id]?.({ event, id, payload });
};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" } },
  transformCallback: (fn) => {
    const id = ++callbackId;
    window.testCallbacks[id] = fn;
    return id;
  },
  invoke: async (cmd, args = {}) => {
    window.calls.push({ cmd, args: copy(args) });
    if (cmd === "plugin:event|listen") {
      (window.testEvents[args.event] ??= []).push(args.handler);
      return args.handler;
    }
    if (cmd.startsWith("plugin:event|")) return;
    if(cmd==="update_status")return {eingerichtet:false,quelle:"",hinweis:"Updates sind in diesem Testbuild nicht eingerichtet.",verfuegbar:null};
    if(cmd==="abstandsregeln")return [];
    if(cmd==="beispielprojekt_status")return window.beispielStatus??{aktiv:false,kopie_vorhanden:false};
    if(cmd==="beispielprojekt_oeffnen"||cmd==="beispielprojekt_verlassen")return;
    if(cmd==="druckgestaltung")return window.druckGestaltung??{kopfzeile:"{Projekt}",fusszeile:"Stand: {Datum}",anschreiben:"Sehr geehrte Damen und Herren,\n\nanbei die Unterlagen.\n\nMit freundlichen Grüßen"};
    if(cmd==="druckgestaltung_speichern"){window.druckGestaltung=JSON.parse(JSON.stringify(args.gestaltung));return window.druckGestaltung;}
    // Live-Pixeländerungen werden separat mit echten PDF-Fixtures geprüft.
    if (cmd === "kategorienkatalog") return {revision:0,kategorien:[]};
    if (cmd === "dokumentkategorien") return [];
    if (cmd === "vorlagenkategorie") return {revision:0,kategorie:null};
    if (cmd === "formular_livevorschau") return args.pdfBase64;
    if (cmd === "plugin:window|destroy") {
      window.testDestroyed = true;
      return;
    }
    if (cmd === "plugin:dialog|save" || cmd === "plugin:dialog|open")
      return null;
    // Rückfragen (ask/confirm): standardmäßig bestätigen; `window.rueckfrage`
    // = false wählt „Abbrechen“. Gestellte Fragen stehen in `window.rueckfragen`.
    if (cmd === "plugin:dialog|message") {
      (window.rueckfragen ??= []).push(args.message);
      const [ja, nein] = args.buttons?.OkCancelCustom ??
        (args.buttons === "OkCancel" ? ["Ok", "Cancel"] : ["Yes", "No"]);
      return window.rueckfrage === false ? nein : ja;
    }
    if (cmd === "fachobjekt_lesen") return null;
    if (cmd === "planaufmass_werkzeugkiste")
      return [
        { id: "flurstueck", gruppe: "Grundstück", titel: "Flurstück", bedeutung: "flurstueck", geometrie: "flaeche", ebene: "Grundstück", hinweis: "Flurstücksgrenze aus Flurkarte oder Lageplan" },
        { id: "nuf", gruppe: "DIN 277", titel: "DIN 277 NUF", bedeutung: "nuf", geometrie: "flaeche", ebene: "DIN 277", hinweis: "Nutzungsfläche; Gebäude und Geschoss zuordnen" },
        { id: "traufe", gruppe: "Höhen", titel: "Traufe", bedeutung: "traufe", geometrie: "hoehenpunkt", ebene: "Höhen", hinweis: "Traufhöhe; Höhenpunkt mit Bezugslinie" },
      ];
    if (["fachobjekte","fachobjektseite","fachverlaufsseite","fachverlauf","nachweise","verfahren_planunterlagen"].includes(cmd)) return [];
    if(cmd==='bereichsstatus')return {projekt_version:initial.version,stichtag:'2026-10-16',bereiche:(window.testBereichsstatus??[{bereich:'rechtsgrundlagen',zustand:'geprueft',gruende:[],kennzeichenbar:false},{bereich:'flurstuecke',zustand:'zu_erledigen',gruende:['Mindestens ein Vorhabenflurstück erfassen'],kennzeichenbar:false},{bereich:'stellplaetze',zustand:'leer',gruende:[],kennzeichenbar:true}])};
    if(cmd==='bereich_benoetigt_setzen'){window.calls?.push?.({cmd,args});const b=(window.testBereichsstatus??=[{bereich:'rechtsgrundlagen',zustand:'geprueft',gruende:[],kennzeichenbar:false},{bereich:'flurstuecke',zustand:'zu_erledigen',gruende:['Mindestens ein Vorhabenflurstück erfassen'],kennzeichenbar:false},{bereich:'stellplaetze',zustand:'leer',gruende:[],kennzeichenbar:true}]).find(x=>x.bereich===args.bereich);if(b)b.zustand=args.benoetigt?'leer':'nicht_benoetigt';return {projekt_version:initial.version,stichtag:'2026-10-16',bereiche:window.testBereichsstatus};}
    if(cmd==='projektuebersicht')return {projekt_id:args.projektId,projekt_version:initial.version,ermittelt_am:initial.created_at,stichtag:'2026-10-16',zahlen:{offene_aufgaben:0,fehlende_unterlagen:0,nachweise_pruefen:0,offene_nachforderungen:0,offene_auflagen:0,ueberfaellig:0},punkte:[],verfahren:[],abgaben:[],empfaenger:[],hinweise:['Fiktive leere Übersicht für diesen Browsertransport.']};
    if (cmd === "projektstatus") return {ermittelt_am:initial.created_at,offene_aufgaben:0,ueberfaellige_aufgaben:0,erforderliche_unterlagen:0,fehlende_unterlagen:0,ungepruefte_unterlagen:0,nachweise:0,veraltete_nachweise:0,offene_freigaben:0,exportierte_pakete:0,bestaetigte_einreichungen:0,hinweise:[]};
    if (cmd === "ausgabeauftraege") return [];
    if(cmd==='sicherungsstatus')return {lokal:true,profil:'Testprofil',hinweis:'Lokale Sicherung'};
    if (cmd === "grundstueckskontext") return [];
    if (cmd === "projekt_bereich_speichern") {
      const a=args.aenderung,p=window.testProjects.find(p=>p.id===a.id);
      // Die Core-Vertragstests prüfen Merge/Konflikte; IPC-Fixture bleibt projektgebunden.
      return window.__TAURI_INTERNALS__.invoke("projekt_aktualisieren",{aenderung:{...a,expected_version:p.version}});
    }
    if (cmd === "arbeitskopien") return [];
    if (cmd === "baugrundstuecke_ermitteln") return [];
    if (cmd === "einreichungen") return [];
    if (cmd === "ablage_status")
      return {
        modus: "Einzelplatz",
        root: "/tmp/BauPlanIO",
        datenbank: "/tmp/bauprojekt.db",
        wechsel_erlaubt: window.testUser.rolle === "administrator",
        hinweis: "Testablage",
      };
    // Start.svelte fragt vor der Anwendung den Anmeldestatus ab; Standard ist
    // der Einzelplatzbetrieb mit angemeldetem Testbenutzer.
    if (cmd === "anmeldestatus")
      return window.testAnmeldung ?? {
        kennwortanmeldung: false,
        benutzer: window.testUser,
        kennwort_aendern: null,
        bisheriges_kennwort_noetig: false,
        optionen: null,
        kennwort_mindestlaenge: 10,
      };
    if (cmd === "app_info")
      return {
        version: "0.1.0",
        benutzer: window.testUser,
        modus: "Einzelplatz",
        storage: "Lokale Ablage",
        regelpakete: [
          { land: "BY", version: "1", rechtsstand: "2026", origin: "test" },
        ],
        cache: null,
      };
    if (cmd === "lizenz_status")
      return {
        zustand: { art: "testversion", tage_uebrig: 30 },
        schreiben_erlaubt: true,
        meldung: "",
        lizenz: null,
        arbeitsplaetze_max: 1,
        arbeitsplaetze: [],
        eigener_arbeitsplatz: null,
        abo: null,
      };
    if (cmd === "projekte_auflisten") return copy(window.testProjects);
    if (cmd === "zuletzt_geoeffnet") return [];
    if (cmd === "projekt_schliessen") return;
    if (cmd === "projekt_laden" || cmd === "projekt_oeffnen")
      return copy(window.testProjects.find((p) => p.id === args.id));
    // Eigenes Projektrecht (core-lib zugriff.rs); je Projekt überschreibbar.
    if (cmd === "projekt_recht")
      return window.testProjektRecht?.[args.id] ?? "leitung";
    if (
      [
        "bauherren",
        "beteiligte",
        "dokumente_auflisten",
        "sperren_auflisten",
        "xbau_codelisten",
        "protokoll",
        "kunden_auflisten",
      ].includes(cmd)
    )
      return [];
    if (cmd === "bauantrag")
      return copy(
        window.savedApplications[args.projektId] ?? {
          antrag: {
            vorgang: "",
            bauaufsicht: { name: "", kennung: "" },
            bezeichnung: "",
            gebuehrenpflichtiger: null,
            massnahmen: [],
            anzahl_gebaeude: 1,
            grundstueck: { anschrift: address, flurstuecke: [] },
            anlagen: [],
          },
          version: 1,
          geaendert_von: null,
          geaendert_am: null,
          fehlende_angaben: [],
        },
      );
    if (cmd === "gebaeudezuordnung_uebertragen") return 0;
    if (cmd === "flurstuecksverweise") return [];
    if (cmd === "flurstuecksverweise_entfernen") return 0;
    if (cmd === "projekt_baugrundstuecke_gebaeude_einbinden")
      return { plan: args.eingabe, hinweise: [] };
    if (cmd === "projekt_baugrundstuecke_pruefen")
      return {
        projekt_version: window.testProjects.find(
          (p) => p.id === args.projektId,
        ).version,
        baugrundstuecke: [],
      };
    if (cmd === "bauantrag_speichern") {
      if (window.saveDelay)
        await new Promise((r) => setTimeout(r, window.saveDelay));
      if (window.failSave)
        throw { art: "validierung", meldung: "Test: Speichern fehlgeschlagen" };
      return copy(
        (window.savedApplications[args.projektId] = {
          antrag: copy(args.antrag),
          version: args.version + 1,
          geaendert_von: "Anna Weber",
          geaendert_am: initial.updated_at,
          fehlende_angaben: [],
        }),
      );
    }
    if (cmd === "projekt_aktualisieren") {
      if (window.saveDelay)
        await new Promise((r) => setTimeout(r, window.saveDelay));
      if (window.failSave)
        throw { art: "validierung", meldung: "Test: Speichern fehlgeschlagen" };
      const a = args.aenderung,
        p = window.testProjects.find((p) => p.id === a.id);
      if (a.expected_version !== p.version)
        throw { art: "konflikt", meldung: "Versionskonflikt" };
      Object.assign(p, a, { version: p.version + 1 });
      return copy(p);
    }
    if (cmd === "abstandsflaeche")
      return {
        ergebnis: {
          tiefe_m: args.eingabe.wandhoehe_m * 0.4,
          faktor: 0.4,
          mindesttiefe_m: 3,
          rechtsgrundlage: "Test",
        },
        regelpaket: { land: "BY", version: "1" },
      };
    throw new Error("Unmocked command: " + cmd);
  },
};
