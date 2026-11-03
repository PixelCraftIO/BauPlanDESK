<script lang="ts">
  import { api, fehlertext } from "./api";
  let { darf }: { darf: boolean } = $props();
  let muster = $state(""),
    gespeichert = $state(""),
    fehler = $state<string | null>(null),
    meldung = $state<string | null>(null);
  const beispiel = $derived(
    muster
      .replaceAll("{projekt}", "2026-014")
      .replaceAll("{planart}", "GR")
      .replaceAll("{geschoss}", "EG")
      .replaceAll("{nr:3}", "003")
      .replaceAll("{nr:2}", "03")
      .replaceAll("{nr}", "3"),
  );
  async function laden() {
    try {
      muster = gespeichert = (await api.plannummernschema()).muster;
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  async function speichern() {
    fehler = meldung = null;
    try {
      muster = gespeichert = (await api.plannummernschemaSpeichern({ muster }))
        .muster;
      meldung =
        "Gespeichert. Bestehende Pläne erhalten die neue Nummer beim nächsten Speichern.";
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  $effect(() => {
    void laden();
  });
</script>

<section class="einstellungsabschnitt">
  <h2>Plannummern</h2>
  <p class="meta">
    Muster für die Planliste. Platzhalter: <code>{"{projekt}"}</code>
    (Projektkennzeichen), <code>{"{planart}"}</code> (LP, GR, SN, AN …),
    <code>{"{geschoss}"}</code>, <code>{"{nr}"}</code>, <code>{"{nr:2}"}</code>,
    <code>{"{nr:3}"}</code>. Leere Teile entfallen samt Trennzeichen.
  </p>
  <div class="raster">
    <label
      >Muster<input bind:value={muster} maxlength="100" disabled={!darf} /></label
    >
    <p class="meta">Beispiel: <strong>{beispiel}</strong></p>
  </div>
  <button onclick={speichern} disabled={!darf || muster === gespeichert}
    >Muster speichern</button
  >
  {#if meldung}<p class="meta" role="status">{meldung}</p>{/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
</section>
