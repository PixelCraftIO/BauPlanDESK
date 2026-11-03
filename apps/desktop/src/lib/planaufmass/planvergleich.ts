/**
 * Planvergleich zweier Planstände als Pixelbild: entfallene Striche rot, neue
 * grün, unverändert grau; geänderte Stellen werden zu Bereichen für
 * Revisionswolken zusammengefasst. Ohne Laufzeit-Importe (node-Test).
 */

/** Dunkle Pixel (Strich) einer RGBA-Fläche. */
const dunkel = (d: Uint8ClampedArray, i: number, schwelle: number) => d[i + 3] > 0 && d[i] + d[i + 1] + d[i + 2] < schwelle * 3;

/**
 * Vergleicht zwei gleich große RGBA-Bilder. Ergebnis: Differenzbild (RGBA) und
 * Maske (0 = gleich/leer, 1 = nur alt, 2 = nur neu). `toleranz` (Pixel)
 * verzeiht kleine Verschiebungen durch Rasterung.
 */
export function differenz(alt: Uint8ClampedArray, neu: Uint8ClampedArray, breite: number, hoehe: number, schwelle = 160, toleranz = 1) {
  const n = breite * hoehe;
  const a = new Uint8Array(n), b = new Uint8Array(n);
  for (let i = 0; i < n; i++) { a[i] = dunkel(alt, i * 4, schwelle) ? 1 : 0; b[i] = dunkel(neu, i * 4, schwelle) ? 1 : 0; }
  const nahe = (m: Uint8Array, x: number, y: number) => {
    for (let dy = -toleranz; dy <= toleranz; dy++) for (let dx = -toleranz; dx <= toleranz; dx++) {
      const xx = x + dx, yy = y + dy;
      if (xx >= 0 && yy >= 0 && xx < breite && yy < hoehe && m[yy * breite + xx]) return true;
    }
    return false;
  };
  const maske = new Uint8Array(n), bild = new Uint8ClampedArray(n * 4);
  for (let y = 0; y < hoehe; y++) for (let x = 0; x < breite; x++) {
    const i = y * breite + x, o = i * 4;
    let farbe: [number, number, number, number] = [255, 255, 255, 0];
    if (a[i] && b[i]) farbe = [150, 150, 150, 255];
    else if (a[i]) { if (nahe(b, x, y)) farbe = [150, 150, 150, 255]; else { maske[i] = 1; farbe = [200, 20, 30, 255]; } }
    else if (b[i]) { if (nahe(a, x, y)) farbe = [150, 150, 150, 255]; else { maske[i] = 2; farbe = [20, 140, 50, 255]; } }
    bild[o] = farbe[0]; bild[o + 1] = farbe[1]; bild[o + 2] = farbe[2]; bild[o + 3] = farbe[3];
  }
  return { bild, maske };
}

/**
 * Fasst geänderte Pixel zu Rechtecken zusammen: Raster mit `zelle` Pixeln,
 * zusammenhängende Zellen (8er-Nachbarschaft, `luecke` Zellen Abstand erlaubt)
 * bilden einen Bereich. Bereiche unter `mindestens` Pixeln entfallen.
 */
export function bereiche(maske: Uint8Array, breite: number, hoehe: number, zelle = 12, luecke = 1, mindestens = 6): [number, number, number, number][] {
  const gx = Math.ceil(breite / zelle), gy = Math.ceil(hoehe / zelle);
  const zaehl = new Uint32Array(gx * gy);
  for (let y = 0; y < hoehe; y++) for (let x = 0; x < breite; x++) if (maske[y * breite + x]) zaehl[Math.floor(y / zelle) * gx + Math.floor(x / zelle)]++;
  const gesehen = new Uint8Array(gx * gy), out: [number, number, number, number][] = [];
  for (let s = 0; s < gx * gy; s++) {
    if (!zaehl[s] || gesehen[s]) continue;
    let x0 = gx, y0 = gy, x1 = -1, y1 = -1, summe = 0;
    const stapel = [s]; gesehen[s] = 1;
    while (stapel.length) {
      const c = stapel.pop()!, cx = c % gx, cy = Math.floor(c / gx);
      x0 = Math.min(x0, cx); y0 = Math.min(y0, cy); x1 = Math.max(x1, cx); y1 = Math.max(y1, cy); summe += zaehl[c];
      for (let dy = -1 - luecke; dy <= 1 + luecke; dy++) for (let dx = -1 - luecke; dx <= 1 + luecke; dx++) {
        const nx = cx + dx, ny = cy + dy;
        if (nx < 0 || ny < 0 || nx >= gx || ny >= gy) continue;
        const k = ny * gx + nx;
        if (zaehl[k] && !gesehen[k]) { gesehen[k] = 1; stapel.push(k); }
      }
    }
    if (summe >= mindestens) out.push([x0 * zelle, y0 * zelle, Math.min(breite, (x1 + 1) * zelle) - x0 * zelle, Math.min(hoehe, (y1 + 1) * zelle) - y0 * zelle]);
  }
  return out;
}
