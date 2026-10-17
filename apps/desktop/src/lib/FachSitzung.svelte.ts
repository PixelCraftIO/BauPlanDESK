import { untrack } from "svelte";
import { plattformApi, type Datensatz } from "./plattformApi";
import { fehlertext } from "./api";
import type { ObjektArt } from "./generated/plattform";
export class FachSitzung<T> {
  liste = $state<Datensatz<T>[]>([]);
  eingabe = $state<T | null>(null);
  original = $state<T | null>(null);
  id = $state<string | null>(null);
  revision = $state(0);
  laeuft = $state(false);
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
      this.eingabe !== null &&
      JSON.stringify(this.eingabe) !== JSON.stringify(this.original)
    );
  }
  istBeschaeftigt() {
    return this.laeuft;
  }
  async laden() {
    this.laeuft = true;
    try {
      this.liste = await plattformApi.liste<T>(this.projektId, this.art);
      if (!this.bereit) {
        this.gerettet = !!localStorage.getItem(this.schluessel);
      }
      this.bereit = true;
      this.fehler = null;
      return true;
    } catch (e) {
      this.fehler = fehlertext(e);
      return false;
    } finally {
      this.laeuft = false;
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
  verwerfen() {
    this.eingabe = null;
    this.original = null;
    this.id = null;
    this.gerettet = false;
    this.meldung = null;
    localStorage.removeItem(this.schluessel);
  }
  entwurfSichern() {
    if (!this.bereit) return;
    const dirty = this.hatAenderungen();
    const daten = this.eingabe;
    const id = this.id;
    const revision = this.revision;
    const original = this.original;
    untrack(() => {
      try {
        if (dirty)
          localStorage.setItem(
            this.schluessel,
            JSON.stringify({ schema: 1, id, revision, daten, original }),
          );
        else if (!this.gerettet) localStorage.removeItem(this.schluessel);
      } catch {
        this.meldung =
          "Lokale Entwurfssicherung ist nicht verfügbar. Bitte regulär speichern.";
      }
    });
  }
  wiederherstellen() {
    try {
      const s = JSON.parse(localStorage.getItem(this.schluessel) ?? "null");
      if (
        s?.schema !== 1 ||
        typeof s.id !== "string" ||
        !Number.isSafeInteger(s.revision) ||
        s.revision < 0 ||
        !s.daten
      )
        throw new Error("Entwurf nicht lesbar");
      this.id = s.id;
      this.revision = s.revision;
      this.eingabe = s.daten;
      this.original = s.original;
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
      localStorage.removeItem(this.schluessel);
      this.meldung = "Gespeichert.";
      return true;
    } catch (e) {
      this.fehler = fehlertext(e);
      return false;
    } finally {
      this.laeuft = false;
    }
  }
}
