<script lang="ts" generics="T">
  import { bereichAktiv } from "./ansichtsAbfrage.svelte";
  import Statusanzeige from "./arbeitsplatz/Statusanzeige.svelte";
  import { plattformApi, type Datensatz } from "./plattformApi";
  import { fehlertext } from "./api";
  import { projektNavigation } from "./projektNavigation.svelte";
  import { onMount, untrack, type Snippet } from "svelte";
  import { editorAnmelden } from "./editoren";
  import type { Ansicht } from "./projektSitzung";
  import type { FachSitzung } from "./FachSitzung.svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  let {
    titel,
    s,
    bereich,
    darf,
    children,
    zusatz,
    lesendBedienbar = false,
    zusatzBeschaeftigt,
    pruefen,
    kompakt = false,
    werkzeuge,
    oeffnen,
    neu,
  }: {
    titel: string;
    s: FachSitzung<T>;
    bereich: Ansicht;
    darf: boolean;
    lesendBedienbar?: boolean;
    zusatzBeschaeftigt?: () => boolean;
    pruefen?: () => boolean;
    children: Snippet<[T]>;
    zusatz?: Snippet<[T]>;
    /** Editor-Modus: bei geöffnetem Eintrag nur eine schmale Speicherleiste, Titel und Listenknöpfe verschwinden. */
    kompakt?: boolean;
    /** Zusätzliche Knöpfe rechts in der Speicherleiste (Symbolknöpfe des Editors). */
    werkzeuge?: Snippet;
    /** Übernimmt „Öffnen“ einer Zeile statt der Bearbeitung in dieser Liste (Zeichnungs-Tabs). */
    oeffnen?: (f: Datensatz<T>) => void;
    /** Übernimmt „Neu anlegen“ statt `s.neu()` (Zeichnungs-Tabs). */
    neu?: () => void;
  } = $props();
  const aktiv = bereichAktiv();
  const editor = $derived(kompakt && (!!s.eingabe || s.gerettet));
  $effect(() => { if (!aktiv()) untrack(() => void s.entwurfFlush()); });
  const formularId = $props.id();
  let formular = $state<HTMLFormElement>();
  let feldfehler = $state<
    { element: HTMLElement; name: string; meldung: string }[]
  >([]);
  let listeOffen = $state(false);
  $effect(() => {
    void s.id;
    untrack(() => (feldfehler = []));
  });
  function feldZeigen(element: HTMLElement) {
    for (let p = element.parentElement; p; p = p.parentElement)
      if (p instanceof HTMLDetailsElement) p.open = true;
    element.focus();
    element.scrollIntoView({block:"center"});
  }
  function validieren() {
    const fachlich = pruefen?.() ?? true;
    const controls = [
      ...(formular?.querySelectorAll<
        HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement
      >("input,select,textarea") ?? []),
    ];
    feldfehler = controls
      .filter((e) => !e.matches(":disabled") && !e.validity.valid)
      .map((element) => ({
        element,
        meldung: element.validity.valueMissing
          ? "Bitte diese Angabe ergänzen."
          : element.validity.rangeUnderflow
            ? `Der Wert muss mindestens ${(element as HTMLInputElement).min} betragen.`
            : element.validity.rangeOverflow
              ? `Der Wert darf höchstens ${(element as HTMLInputElement).max} betragen.`
              : "Bitte eine gültige Eingabe verwenden.",
        name:
          element.labels?.[0]?.textContent?.trim() ||
          element.getAttribute("aria-label") ||
          "Eingabe",
      }));
    for (const e of controls) {
      if (!e.matches(":disabled") && !e.validity.valid)
        e.setAttribute("aria-invalid", "true");
      else if (!e.closest('[data-feldzustand="fehler"]'))
        e.removeAttribute("aria-invalid");
    }
    feldfehler.forEach((f, i) =>
      f.element.setAttribute("aria-errormessage", `${formularId}-fehler-${i}`),
    );
    if (feldfehler.length) feldZeigen(feldfehler[0].element);
    return fachlich && feldfehler.length === 0;
  }
  $effect(() => {
    s.vorSpeichern = validieren;
    return () => {
      s.vorSpeichern = undefined;
    };
  });
  $effect(()=>{if(s.hatAenderungen() && s.meldung === "Gespeichert.")s.meldung=null;});
  const gesperrt = $derived(s.laeuft || (zusatzBeschaeftigt?.() ?? false));
  let verlauf = $state<Datensatz<T>[]>([]);
  let verlaufId = $state(""), mehrVerlauf = $state(false);
  async function historie(id: string, weitere = false) {
    try {
      const seite = await plattformApi.verlaufsseite<T>(s.projektId, s.art, id, weitere ? verlauf.at(-1)?.revision ?? null : null);
      verlaufId = id;
      mehrVerlauf = seite.length === 50;
      verlauf = weitere ? [...verlauf, ...seite] : seite;
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  const navigation = projektNavigation();
  let sprung = 0;
  $effect(() => {
    const a = navigation?.auftrag;
    if (
      !a ||
      a.nummer === sprung ||
      a.ziel.bereich !== bereich ||
      !a.ziel.objekt ||
      !s.bereit ||
      gesperrt
    )
      return;
    untrack(() => {
      sprung = a.nummer;
      if (s.hatAenderungen() || s.gerettet) {
        if (s.id !== a.ziel.objekt)
          s.meldung =
            "Ein anderer Entwurf ist noch offen. Bitte zuerst speichern oder verwerfen; anschließend den Eintrag erneut aus der Projektübersicht öffnen.";
        return;
      }
      void s.laden().then(async (ok) => {
        if (!ok || navigation?.auftrag?.nummer !== a.nummer) return;
        if (s.hatAenderungen() || s.gerettet) return;
        const f = s.liste.find((f) => f.objekt === a.ziel.objekt) ?? await plattformApi.lesen<T>(s.projektId, s.art, a.ziel.objekt!);
        if (navigation?.auftrag?.nummer !== a.nummer || s.hatAenderungen() || s.gerettet) return;
        if (f) {
          s.meldung = null;
          if (oeffnen) oeffnen(f);
          else s.bearbeiten(f);
        } else
          s.fehler =
            "Der verknüpfte Eintrag ist nicht mehr in dieser Projektliste vorhanden. Liste aktualisieren.";
      }).catch(e => { s.fehler = fehlertext(e); });
    });
  });
  onMount(() => {
    void s.laden();
    const flush = () => { void s.entwurfFlush(); };
    document.addEventListener("visibilitychange", flush);
    window.addEventListener("pagehide", flush);
    return () => {
      document.removeEventListener("visibilitychange", flush);
      window.removeEventListener("pagehide", flush);
      s.beenden();
    };
  });
  $effect(() => s.entwurfSichern());
  // Komponenten besitzen eine feste Route innerhalb ihrer Projektakte.
  // svelte-ignore state_referenced_locally
  editorAnmelden(bereich, {
    hatAenderungen: () => s.hatAenderungen(),
    istBeschaeftigt: () =>
      s.istBeschaeftigt() || (zusatzBeschaeftigt?.() ?? false),
    speichern: () =>
      zusatzBeschaeftigt?.() ? Promise.resolve(false) : s.speichern(),
  });
  /** „Entwurf verwerfen“ löscht auch die lokale Sicherung: vorher nachfragen. */
  async function verwerfen() {
    if (
      (s.hatAenderungen() || s.gerettet) &&
      !(await ask(
        s.hatAenderungen()
          ? "Die ungespeicherten Änderungen verwerfen?"
          : "Den lokal gesicherten Entwurf endgültig verwerfen?",
        {
          title: titel,
          kind: "warning",
          okLabel: "Verwerfen",
          cancelLabel: "Weiter bearbeiten",
        },
      ))
    )
      return;
    await s.verwerfen();
  }
</script>

{#if !editor}<h2>{titel}</h2>{/if}
{#if s.fehler}<p class="fehlertext" role="alert">{s.fehler}</p>{/if}
{#if s.meldung}<p role="status">{s.meldung}</p>{/if}
{#if s.gerettet}<div class="hinweisbox">
    Ein lokaler Entwurf wurde nach der letzten Sitzung gefunden. <button
      class="link"
      onclick={() => s.wiederherstellen()}>Entwurf wiederherstellen</button
    ><button class="link" onclick={verwerfen}
      >Entwurf verwerfen</button
    >
  </div>{/if}
<div class="knoepfe links" class:ausgeblendet={editor}>
  <button
    disabled={!darf || gesperrt || s.hatAenderungen() || s.gerettet}
    onclick={() => (neu ? neu() : s.neu())}>Neu anlegen</button
  ><button class="sekundaer" disabled={gesperrt} onclick={() => s.laden()}
    >Liste aktualisieren</button
  >
</div>
{#if s.eingabe && s.liste.length && !editor}<button type="button" class="link" aria-expanded={listeOffen} onclick={()=>listeOffen=!listeOffen}>{listeOffen ? "Eintragsliste einklappen" : `Eintragsliste anzeigen (${s.liste.length})`}</button>{/if}
{#if !editor && (!s.eingabe || listeOffen)}
  <table class="ergebnistabelle">
    <thead
      ><tr
        ><th>Bezeichnung</th>{#if zusatz}<th>Grundlagen</th>{/if}<th>Stand</th
        ><th></th></tr
      ></thead
    ><tbody
      >{#each s.liste as f (f.objekt)}<tr
          ><td>{s.titel(f.daten)}</td>{#if zusatz}<td
              >{@render zusatz(f.daten)}</td
            >{/if}<td>{f.revision}</td><td
            ><button
              class="link"
              disabled={gesperrt || (!oeffnen && (s.hatAenderungen() || s.gerettet))}
              onclick={() => (oeffnen ? oeffnen(f) : s.bearbeiten(f))}>Öffnen</button
            ><button class="link" onclick={() => historie(f.objekt)}
              >Verlauf</button
            ></td
          ></tr
        >{:else}<tr
          ><td colspan={zusatz ? 4 : 3}
            >{s.laeuft
              ? "Wird geladen …"
              : "Noch keine Einträge vorhanden."}</td
          ></tr
        >{/each}</tbody
    >
  </table>
  {#if s.mehrVorhanden}<button class="sekundaer" disabled={gesperrt} onclick={() => s.laden(true)}>Weitere Einträge laden</button>{/if}
{:else if !s.liste.length && !editor}<button type="button" class="link" onclick={() => (listeOffen = true)}
    >Eintragsliste anzeigen</button
  >{/if}
{#if s.eingabe}<form
    bind:this={formular}
    novalidate
    onfocusout={(e) => {
      // Inline-Prüfung beim Verlassen eines Feldes, nicht erst beim Speichern.
      const el = e.target;
      if (
        !(el instanceof HTMLInputElement || el instanceof HTMLSelectElement || el instanceof HTMLTextAreaElement) ||
        el.matches(":disabled") ||
        el.closest('[data-feldzustand="fehler"]')
      )
        return;
      if (!el.validity.valid) el.setAttribute("aria-invalid", "true");
      else el.removeAttribute("aria-invalid");
    }}
    oninput={(e) => {
      const el = e.target as HTMLInputElement;
      if (
        el?.getAttribute?.("aria-invalid") === "true" &&
        el.validity?.valid &&
        !el.closest('[data-feldzustand="fehler"]')
      )
        el.removeAttribute("aria-invalid");
      for (const f of feldfehler)
        if ((f.element as HTMLInputElement).validity.valid) {
          f.element.removeAttribute("aria-invalid");
          f.element.removeAttribute("aria-errormessage");
        }
      feldfehler = feldfehler.filter(
        (f) =>
          f.element.isConnected &&
          !(f.element as HTMLInputElement).validity.valid,
      );
      feldfehler.forEach((f, i) =>
        f.element.setAttribute(
          "aria-errormessage",
          `${formularId}-fehler-${i}`,
        ),
      );
    }}
    class="fachformular"
    class:aktenformular={lesendBedienbar}
    onsubmit={async (e) => {
      e.preventDefault();
      if (!gesperrt) await s.speichern();
    }}
  >
    {#if !darf}<p class="lesemodus" role="status">
        Nur Leserecht · Die Angaben können hier nicht geändert werden.
      </p>{/if}
    {#if !editor}<p class="meta pflichtkonvention">
      Mit * gekennzeichnete Angaben sind erforderlich.
    </p>{/if}
    {#if feldfehler.length}<div class="fehlerzusammenfassung" role="alert">
        <strong>Bitte die erforderlichen Eingaben ergänzen.</strong>
        <ul>
          {#each feldfehler as f, i}<li id={`${formularId}-fehler-${i}`}>
              <button
                type="button"
                class="link"
                onclick={() => feldZeigen(f.element)}>{f.name}: {f.meldung}</button
              >
            </li>{/each}
        </ul>
      </div>{/if}
    <div class="knoepfe links speicherleiste" class:editor>
      {#if editor}<strong class="editortitel">{titel}</strong>{/if}
      <span aria-label="Speicherstatus" role="status">{#if s.laeuft}Wird gespeichert …{:else}<Statusanzeige status={s.hatAenderungen() ? "entwurf" : "gespeichert"}/>{/if}</span>
      <button
        disabled={!darf || gesperrt || !(s.hatAenderungen() || s.gerettet)}
        >Speichern</button
      ><button
        type="button"
        class="sekundaer"
        disabled={gesperrt}
        onclick={verwerfen}
        >{s.hatAenderungen() ? "Entwurf verwerfen" : "Schließen"}</button
      >
      {#if werkzeuge}<span class="editorwerkzeuge">{@render werkzeuge()}</span>{/if}
    </div>
    <fieldset disabled={(!darf && !lesendBedienbar) || gesperrt}>
      <legend class:ausgeblendet={editor}
        >{s.revision === 0 ? "Neuer Eintrag" : "Eintrag bearbeiten"}</legend
      >{@render children(s.eingabe)}
    </fieldset>
  </form>{/if}

{#if verlauf.length}<details open>
    <summary>Gespeicherte Fachstände</summary>
    <table class="ergebnistabelle">
      <thead><tr><th>Stand</th><th>Bezeichnung</th></tr></thead><tbody
        >{#each verlauf as f (f.revision)}<tr
            ><td>{f.revision}</td><td>{s.titel(f.daten)}</td></tr
          >{/each}</tbody
      >
    </table>
    <button class="link" onclick={() => { verlauf = []; mehrVerlauf = false; }}
      >Verlauf schließen</button
    >
  </details>{/if}

<style>
  .ausgeblendet {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .speicherleiste.editor {
    padding: 4px 8px;
    gap: 6px;
    align-items: center;
    background: var(--kopfzeile);
  }
  .speicherleiste.editor .editortitel {
    margin-right: 6px;
  }
  .speicherleiste.editor .editorwerkzeuge {
    display: inline-flex;
    gap: 2px;
    align-items: center;
    margin-left: 10px;
    padding-left: 10px;
    border-left: 1px solid var(--border);
  }
  .speicherleiste {
    position: sticky;
    top: 0;
    z-index: 3;
    background: var(--panel);
    padding: 1rem 0.5rem;
    border-bottom: 1px solid var(--border);
  }
  .fachformular :global(input),
  .fachformular :global(select),
  .fachformular :global(textarea) {
    scroll-margin-block-start: 7rem;
  }
  .fachformular {
    display: block;
  }
  .fachformular > fieldset {
    width: 100%;
    margin-block: 1.5rem;
  }
  .aktenformular > fieldset {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    border: 0;
    margin: 0;
    padding: 0;
  }
  .aktenformular > .knoepfe {
    margin-top: 1rem;
  }
  td > .link + .link {
    margin-left: 0.75rem;
  }
</style>

{#if mehrVerlauf}<button class="sekundaer" onclick={() => historie(verlaufId, true)}>Ältere Revisionen laden</button>{/if}
