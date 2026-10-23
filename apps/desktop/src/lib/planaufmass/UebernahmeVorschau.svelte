<script lang="ts">
  import { onDestroy } from "svelte";
  import Dialog from "../Dialog.svelte";
  import type { Uebernahme } from "./Uebernahme.svelte";
  let {v}:{v:Uebernahme}=$props();
  onDestroy(()=>v.antwort(false));
</script>
{#if v.daten}<Dialog titel="Aufmaßwerte prüfen und übernehmen" breit onclose={()=>v.antwort(false)}>
  <p>{v.daten.speichern?"Nach Bestätigung werden die folgenden Zielobjekte gespeichert. Bestehende Revisionen werden dabei geprüft.":"Nach Bestätigung werden die Werte in den Entwurf eingesetzt. Speichern erfolgt anschließend im Zielbereich."}</p>
  <div class="tabellen-scroll"><table class="ergebnistabelle" aria-label="Vorschau der Aufmaßübernahme"><thead><tr><th>Zielfeld</th><th>Bisher</th><th>Neuer Wert</th><th>Quelle</th></tr></thead><tbody>{#each v.daten.zeilen as z}<tr><th scope="row">{z.ziel}</th><td>{z.vorher}</td><td>{z.nachher}</td><td>{z.quelle}</td></tr>{/each}</tbody></table></div>
  <div class="knoepfe"><button type="button" onclick={()=>v.antwort(true)}>{v.daten.speichern?"Übernehmen und speichern":"Im Entwurf einsetzen"}</button><button type="button" class="sekundaer" onclick={()=>v.antwort(false)}>Abbrechen</button></div>
</Dialog>{/if}
