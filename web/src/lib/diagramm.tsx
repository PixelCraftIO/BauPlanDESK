// Ablaufdiagramme als Inline-SVG ohne Bibliothek. Ein Diagramm wird als Daten
// beschrieben (Kästen mit Lage, Pfeile mit Beschriftung) und hier gezeichnet.
// Farben kommen über CSS-Klassen aus globals.css und folgen damit dem hellen
// und dem dunklen Erscheinungsbild.

export type Art = "normal" | "akzent" | "daten" | "frage" | "hinweis";
export type Seite = "l" | "r" | "o" | "u";

export type Knoten = {
  id: string;
  x: number;
  y: number;
  /** Breite und Höhe; Standard 190 × 64. */
  w?: number;
  h?: number;
  titel: string;
  /** Kleine zweite und dritte Zeile unter dem Titel. */
  zeilen?: string[];
  art?: Art;
};

export type Kante = {
  von: string;
  nach: string;
  text?: string;
  /** Austrittsseite am Start bzw. Eintrittsseite am Ziel; sonst nach Lage. */
  aus?: Seite;
  ein?: Seite;
  /** Verschiebung des Ankers entlang der Seite (px), z. B. für zwei Pfeile an einer Kante. */
  ausVersatz?: number;
  einVersatz?: number;
  /** Zwischenpunkte für rechtwinklige Führung. */
  ueber?: [number, number][];
  gestrichelt?: boolean;
  akzent?: boolean;
  /** Lage der Beschriftung: Index des Abschnitts (Standard: längster). */
  textAbschnitt?: number;
  /** Verschiebung der Beschriftung gegenüber der Abschnittsmitte. */
  textVersatz?: [number, number];
};

export type Diagramm = {
  breite: number;
  hoehe: number;
  knoten: Knoten[];
  kanten: Kante[];
};

const W = 190;
const H = 64;

function mass(k: Knoten) {
  return { w: k.w ?? W, h: k.h ?? H };
}

function anker(k: Knoten, s: Seite, v = 0): [number, number] {
  const { w, h } = mass(k);
  switch (s) {
    case "l":
      return [k.x, k.y + h / 2 + v];
    case "r":
      return [k.x + w, k.y + h / 2 + v];
    case "o":
      return [k.x + w / 2 + v, k.y];
    case "u":
      return [k.x + w / 2 + v, k.y + h];
  }
}

/** Seite, auf der `ziel` von `k` aus liegt. */
function richtung(k: Knoten, ziel: [number, number]): Seite {
  const { w, h } = mass(k);
  const dx = ziel[0] - (k.x + w / 2);
  const dy = ziel[1] - (k.y + h / 2);
  if (Math.abs(dx) / w > Math.abs(dy) / h) return dx > 0 ? "r" : "l";
  return dy > 0 ? "u" : "o";
}

function mitte(k: Knoten): [number, number] {
  const { w, h } = mass(k);
  return [k.x + w / 2, k.y + h / 2];
}

/** Ungefähre Textbreite bei 11,5 px Systemschrift. */
function textbreite(t: string, px = 11.5) {
  return t.length * px * 0.56;
}

function KnotenBild({ k }: { k: Knoten }) {
  const { w, h } = mass(k);
  const art = k.art ?? "normal";
  const zeilen = k.zeilen ?? [];
  const zeilenhoehe = 15;
  const block = 17 + zeilen.length * zeilenhoehe;
  const start = k.y + h / 2 - block / 2 + 13;
  const cx = k.x + w / 2;
  return (
    <g className={`dg-knoten dg-${art}`}>
      {art === "frage" ? (
        <polygon
          points={`${cx},${k.y} ${k.x + w},${k.y + h / 2} ${cx},${k.y + h} ${k.x},${k.y + h / 2}`}
          className="dg-flaeche"
        />
      ) : (
        <rect x={k.x} y={k.y} width={w} height={h} rx={art === "daten" ? 10 : 3} className="dg-flaeche" />
      )}
      <text x={cx} y={start} textAnchor="middle" className="dg-titel">
        {k.titel}
      </text>
      {zeilen.map((z, i) => (
        <text key={i} x={cx} y={start + 17 + i * zeilenhoehe} textAnchor="middle" className="dg-zeile">
          {z}
        </text>
      ))}
    </g>
  );
}

function KantenBild({ kante, knoten, id }: { kante: Kante; knoten: Map<string, Knoten>; id: string }) {
  const a = knoten.get(kante.von);
  const b = knoten.get(kante.nach);
  if (!a || !b) throw new Error(`Diagramm: unbekannter Knoten in ${kante.von} → ${kante.nach}`);
  const ueber = kante.ueber ?? [];
  const aus = kante.aus ?? richtung(a, ueber[0] ?? mitte(b));
  const ein = kante.ein ?? richtung(b, ueber[ueber.length - 1] ?? mitte(a));
  const punkte = [anker(a, aus, kante.ausVersatz), ...ueber, anker(b, ein, kante.einVersatz)];

  let beschriftung = null;
  if (kante.text) {
    let index = kante.textAbschnitt ?? 0;
    if (kante.textAbschnitt === undefined) {
      let laengste = -1;
      for (let i = 0; i < punkte.length - 1; i++) {
        const l = Math.hypot(punkte[i + 1][0] - punkte[i][0], punkte[i + 1][1] - punkte[i][1]);
        if (l > laengste) {
          laengste = l;
          index = i;
        }
      }
    }
    const [p, q] = [punkte[index], punkte[index + 1]];
    const [vx, vy] = kante.textVersatz ?? [0, 0];
    const tx = (p[0] + q[0]) / 2 + vx;
    const ty = (p[1] + q[1]) / 2 + vy;
    const bw = textbreite(kante.text) + 10;
    beschriftung = (
      <g className="dg-beschriftung">
        <rect x={tx - bw / 2} y={ty - 9} width={bw} height={18} rx={2} />
        <text x={tx} y={ty + 4} textAnchor="middle">
          {kante.text}
        </text>
      </g>
    );
  }

  return (
    <g className={`dg-kante${kante.akzent ? " dg-kante-akzent" : ""}`}>
      <polyline
        points={punkte.map((p) => p.join(",")).join(" ")}
        fill="none"
        strokeDasharray={kante.gestrichelt ? "5 4" : undefined}
        markerEnd={`url(#${id}-${kante.akzent ? "pfeil-akzent" : "pfeil"})`}
      />
      {beschriftung}
    </g>
  );
}

export function Ablaufdiagramm({
  id,
  diagramm,
  titel,
  beschreibung,
}: {
  /** Eindeutig auf der Seite; Präfix für die Pfeilspitzen. */
  id: string;
  diagramm: Diagramm;
  /** Kurze Aussage des Bildes für Bildschirmleser. */
  titel: string;
  beschreibung: React.ReactNode;
}) {
  const knoten = new Map(diagramm.knoten.map((k) => [k.id, k]));
  return (
    <figure className="diagramm">
      <div className="diagramm-flaeche">
        <svg viewBox={`0 0 ${diagramm.breite} ${diagramm.hoehe}`} role="img" aria-label={titel}>
          <defs>
            <marker id={`${id}-pfeil`} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
              <path d="M0,0 L10,5 L0,10 z" className="dg-spitze" />
            </marker>
            <marker id={`${id}-pfeil-akzent`} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
              <path d="M0,0 L10,5 L0,10 z" className="dg-spitze-akzent" />
            </marker>
          </defs>
          {diagramm.kanten.map((k, i) => (
            <KantenBild key={i} kante={k} knoten={knoten} id={id} />
          ))}
          {diagramm.knoten.map((k) => (
            <KnotenBild key={k.id} k={k} />
          ))}
        </svg>
      </div>
      <figcaption>{beschreibung}</figcaption>
    </figure>
  );
}
