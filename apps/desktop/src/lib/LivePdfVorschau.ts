import type { Feldwert } from "./api";
interface Auftrag {
  quelle: string;
  werte: Record<string, Feldwert>;
  stand: string;
  nummer: number;
  fruehestens: number;
}
/** Eine laufende Berechnung, höchstens ein nachfolgender Stand. Alte Antworten sind wirkungslos. */
export class LivePdfVorschau {
  private nummer = 0;
  private naechster: Auftrag | null = null;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private laeuft = false;
  private beendet = false;
  constructor(
    private berechnen: (
      pdf: string,
      werte: Record<string, Feldwert>,
    ) => Promise<string>,
    private fertig: (pdf: string, stand: string) => void,
    private fehler: (e: unknown) => void,
  ) {}
  planen(quelle: string, werte: Record<string, Feldwert>, stand: string) {
    this.naechster = {
      quelle,
      werte,
      stand,
      nummer: ++this.nummer,
      fruehestens: Date.now() + 180,
    };
    clearTimeout(this.timer);
    this.starten();
  }
  beenden() {
    this.beendet = true;
    this.nummer++;
    this.naechster = null;
    clearTimeout(this.timer);
  }
  private starten() {
    if (this.beendet || this.laeuft || !this.naechster) return;
    this.timer = setTimeout(
      () => void this.ausfuehren(),
      Math.max(0, this.naechster.fruehestens - Date.now()),
    );
  }
  private async ausfuehren() {
    const a = this.naechster;
    if (!a || this.beendet || this.laeuft) return;
    this.naechster = null;
    this.laeuft = true;
    try {
      const pdf = Object.keys(a.werte).length
        ? await this.berechnen(a.quelle, a.werte)
        : a.quelle;
      if (!this.beendet && a.nummer === this.nummer) this.fertig(pdf, a.stand);
    } catch (e) {
      if (!this.beendet && a.nummer === this.nummer) this.fehler(e);
    } finally {
      this.laeuft = false;
      this.starten();
    }
  }
}
