import { untrack } from "svelte";
import { entwurfspeicher } from "./entwurfspeicher";
import { plattformApi, type Datensatz } from "./plattformApi";
import { fehlertext } from "./api";
import type { ObjektArt } from "./generated/plattform";
export class FachSitzung<T> {
  liste = $state<Datensatz<T>[]>([]);
  mehrVorhanden = $state(false);
  private cursor: string | null = null;
  eingabe = $state<T | null>(null);
  original = $state<T | null>(null);
  id = $state<string | null>(null);
  revision = $state(0);
  private aktion = $state(false);
  private ladezahl = $state(0);
  private ladegeneration = 0;
  private entwurf: unknown = null;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private geschlossen = false;
  private eingabeSignatur = $derived(JSON.stringify(this.eingabe));
  private originalSignatur = $derived(JSON.stringify(this.original));
  get laeuft() {
    return this.aktion || this.ladezahl > 0;
  }
  set laeuft(wert: boolean) {
    this.aktion = wert;
  }
  fehler = $state<string | null>(null);
  meldung = $state<string | null>(null);
  bereit = $state(false);
  gerettet = $state(false);
  constructor(
    readonly projektId: string,
    readonly art: ObjektArt,
    readonly benutzerId: string,
    readonly leer: () => T,
    readonly titel: (d: T) => string,
    readonly entwurfObjekt = "",
  ) {}
  private get schluessel() {
    return `bauprojekt.entwurf.v1.${this.benutzerId}.${this.projektId}.${this.art}${this.entwurfObjekt ? "." + this.entwurfObjekt : ""}`;
  }
  hatAenderungen() {
    return (
      this.eingabe !== null && this.eingabeSignatur !== this.originalSignatur
    );
  }
  istBeschaeftigt() {
    return this.laeuft;
  }
  async laden(weitere = false) {
    const generation = ++this.ladegeneration;
    this.ladezahl++;
    try {
      const liste = await plattformApi.seite<T>(
        this.projektId,
        this.art,
        weitere ? this.cursor : null,
      );
      if (generation !== this.ladegeneration || this.geschlossen) return false;
      this.cursor = liste.at(-1)?.objekt ?? null;
      this.mehrVorhanden = liste.length === 100;
      this.liste = weitere
        ? [
            ...this.liste,
            ...liste.filter(
              (f) => !this.liste.some((g) => g.objekt === f.objekt),
            ),
          ]
        : liste;
      if (!this.bereit) {
        try {
          this.entwurf = await entwurfspeicher.lesen(this.schluessel);
          if (generation !== this.ladegeneration || this.geschlossen)
            return false;
          this.gerettet = !!this.entwurf;
        } catch {
          this.meldung =
            "Lokale Entwurfssicherung ist nicht verfügbar. Bitte regulär speichern.";
        }
      }
      this.bereit = true;
      this.fehler = null;
      return true;
    } catch (e) {
      if (generation === this.ladegeneration && !this.geschlossen)
        this.fehler = fehlertext(e);
      return false;
    } finally {
      this.ladezahl--;
    }
  }
  neu() {
    if (this.hatAenderungen() || this.gerettet) return;
    this.id = crypto.randomUUID();
    this.revision = 0;
    this.eingabe = this.leer();
    this.original = null;
    this.fehler = null;
    this.meldung = null;
  }
  bearbeiten(f: Datensatz<T>) {
    if (this.hatAenderungen() || this.gerettet) return;
    this.id = f.objekt;
    this.revision = f.revision;
    this.original = JSON.parse(JSON.stringify(f.daten)) as T;
    this.eingabe = JSON.parse(JSON.stringify(f.daten)) as T;
    this.fehler = null;
    this.meldung = null;
  }
  async verwerfen() {
    try {
      clearTimeout(this.timer);
      await entwurfspeicher.loeschen(this.schluessel);
      this.eingabe = null;
      this.original = null;
      this.id = null;
      this.entwurf = null;
      this.gerettet = false;
      this.meldung = null;
    } catch {
      this.meldung =
        "Der lokale Entwurf konnte nicht entfernt werden. Er bleibt bis zum erfolgreichen Verwerfen erhalten.";
    }
  }
  entwurfSichern() {
    if (!this.bereit || this.geschlossen) return;
    // Signaturen einmal je Änderung, nicht erneut pro Statusanzeige berechnen.
    const dirty = this.hatAenderungen();
    void this.eingabeSignatur;
    untrack(() => {
      clearTimeout(this.timer);
      if (dirty) this.timer = setTimeout(() => void this.entwurfFlush(), 300);
    });
  }
  async entwurfFlush() {
    clearTimeout(this.timer);
    if (!this.bereit || !this.hatAenderungen()) return;
    try {
      // Die bereits abgeleiteten JSON-Signaturen sind unveränderliche Schnappschüsse.
      // So entfällt das erneute Durchlaufen großer reaktiver Objektbäume für IDB.
      const wert = {
        schema: 2,
        id: this.id,
        revision: this.revision,
        datenJson: this.eingabeSignatur,
        originalJson: this.originalSignatur,
      };
      await entwurfspeicher.schreiben(this.schluessel, wert);
    } catch {
      this.meldung =
        "Lokale Entwurfssicherung ist nicht verfügbar. Bitte regulär speichern.";
    }
  }
  beenden() {
    void this.entwurfFlush();
    this.geschlossen = true;
    this.ladegeneration++;
  }
  wiederherstellen() {
    try {
      const s = this.entwurf as {
        schema?: number;
        id?: string;
        revision?: number;
        daten?: T;
        original?: T;
        datenJson?: string;
        originalJson?: string;
      } | null;
      if (
        !s ||
        ![1, 2].includes(s.schema ?? 0) ||
        typeof s.id !== "string" ||
        !Number.isSafeInteger(s.revision) ||
        s.revision! < 0
      )
        throw new Error("Entwurf nicht lesbar");
      const gespeichert = this.liste.find((f) => f.objekt === s.id);
      if (gespeichert && gespeichert.revision !== s.revision) {
        this.meldung =
          "Der gespeicherte Stand wurde seit diesem Entwurf geändert. Der Entwurf wird zur Prüfung geöffnet; beim Speichern bleibt die Revisionsprüfung aktiv.";
      }
      this.id = s.id;
      this.revision = s.revision!;
      const daten: T =
        s.schema === 2 ? JSON.parse(s.datenJson ?? "null") : s.daten;
      const original: T | null =
        s.schema === 2
          ? JSON.parse(s.originalJson ?? "null")
          : (s.original ?? null);
      if (!daten) throw new Error("Entwurf nicht lesbar");
      this.eingabe = daten;
      this.original = original;
      this.gerettet = false;
    } catch (e) {
      this.fehler = fehlertext(e);
    }
  }
  vorSpeichern?: () => boolean;
  async speichern(): Promise<boolean> {
    if (this.vorSpeichern && !this.vorSpeichern()) return false;
    if (this.laeuft) return false;
    if (!this.hatAenderungen()) return true;
    if (!this.eingabe || !this.id) return false;
    this.ladegeneration++;
    this.laeuft = true;
    this.fehler = null;
    this.meldung = null;
    try {
      const f = await plattformApi.speichern<T>(
        this.projektId,
        this.art,
        this.id,
        this.revision,
        JSON.parse(JSON.stringify(this.eingabe)) as T,
      );
      this.revision = f.revision;
      this.original = structuredClone(f.daten);
      this.eingabe = structuredClone(f.daten);
      this.liste = [...this.liste.filter((x) => x.objekt !== f.objekt), f];
      this.gerettet = false;
      clearTimeout(this.timer);
      this.meldung = "Gespeichert.";
      try {
        await entwurfspeicher.loeschen(this.schluessel);
        this.entwurf = null;
      } catch {
        this.meldung =
          "Gespeichert. Der lokale Entwurf konnte nicht bereinigt werden; bei einer Wiederherstellung den gespeicherten Stand prüfen.";
      }
      return true;
    } catch (e) {
      this.fehler = fehlertext(e);
      return false;
    } finally {
      this.laeuft = false;
    }
  }
}
