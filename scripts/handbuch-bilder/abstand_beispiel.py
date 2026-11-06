"""Zeichnet die Abbildung zum Rechenbeispiel „Aufschüttung und Abgrabung“
(Benutzerhandbuch Kapitel 6) als SVG und rechnet sie mit Chromium in
docs/handbuch/benutzerhandbuch/bilder/abstand-beispiel-aufschuettung.png um.

Die Zahlen stammen aus dem Test
crates/core-lib/src/abstandsflaechen/tests.rs
(handbuch_beispiel_aufschuettung_und_abgrabung); ändert sich dort etwas,
hier und im Handbuch nachziehen.

Aufruf: python3 scripts/handbuch-bilder/abstand_beispiel.py
(Chromium über BAUPROJEKT_CHROMIUM, sonst /opt/pw-browsers/chromium bzw. „chromium“)
"""

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

WURZEL = Path(__file__).resolve().parents[2]
ZIEL = WURZEL / "docs/handbuch/benutzerhandbuch/bilder/abstand-beispiel-aufschuettung.png"
W, H = 1060, 1060


def zeichnen():
    S=55  # px je m
    X0=190  # px für Station 0
    def x(s): return X0+s*S
    YB=650  # px für Höhe 0 (Ansicht)
    def y(h): return YB-h*S
    bestand=lambda s: 1+3*s/12
    out=[]
    a=out.append
    a(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" font-family="DejaVu Sans, Liberation Sans, Arial, sans-serif" font-size="14">')
    a(f'<rect width="{W}" height="{H}" fill="#fff"/>')
    a(f'<text x="24" y="34" font-size="20" font-weight="700" fill="#1a1a1a">Ansicht der Wand (Höhen in m über Bezugspunkt)</text>')
    # Höhenraster
    for h in range(0,11):
        a(f'<line x1="{x(-1.6)}" y1="{y(h)}" x2="{x(13.6)}" y2="{y(h)}" stroke="#eee"/>')
        a(f'<text x="{x(-1.75)}" y="{y(h)+5}" text-anchor="end" fill="#777" font-size="13">+{h},00</text>')
    # Gebäude
    a(f'')
    a(f'<polyline points="{x(0)},{y(5)} {x(0)},{y(10)} {x(12)},{y(10)} {x(12)},{y(2.75)}" fill="none" stroke="#1a1a1a" stroke-width="3"/>')
    a(f'<text x="{x(6)}" y="{y(10)-12}" text-anchor="middle" fill="#1a1a1a">Wandoberkante +10,00 (Flachdach / Attika), Wandlänge 12,00 m</text>')
    # Bestand gelb
    a(f'<line x1="{x(-1.6)}" y1="{y(bestand(-1.6))}" x2="{x(13.6)}" y2="{y(bestand(13.6))}" stroke="#e0a800" stroke-width="3" stroke-dasharray="10 6"/>')
    # geplant schwarz: Kuppe
    pts=[]
    import math
    for i in range(0,41):
        s=-1.6+(5.25+1.6)*i/40
        # Parabel mit Scheitel bei 0: h=5 - k*s^2, h(5.25)=4
        k=1/5.25**2
        pts.append(f'{x(s):.1f},{y(5-k*s*s):.1f}')
    pts += [f'{x(5.25)},{y(1)}', f'{x(9.25)},{y(1)}', f'{x(12)},{y(2.75)}', f'{x(13.6)},{y(2.75+1.6*1.75/2.75):.1f}']
    a(f'<polyline points="{" ".join(pts)}" fill="none" stroke="#1a1a1a" stroke-width="3"/>')
    # Flächen: Aufschüttung / Abgrabung andeuten
    # Wandteilgrenzen
    for s in (5.25,9.25):
        a(f'<line x1="{x(s)}" y1="{y(10)}" x2="{x(s)}" y2="{y(-0.3)}" stroke="#c0392b" stroke-width="2" stroke-dasharray="6 5"/>')
    a(f'<text x="{x(5.25)+6}" y="{y(9.2)}" fill="#c0392b" font-size="13">Grenze 5,25</text>')
    a(f'<text x="{x(9.25)+6}" y="{y(9.2)}" fill="#c0392b" font-size="13">Grenze 9,25</text>')
    # Höhenkoten
    def kote(s,h,t,col,dx=8,anchor="start",dy=-8):
        a(f'<circle cx="{x(s)}" cy="{y(h)}" r="4" fill="{col}"/>')
        a(f'<text x="{x(s)+dx}" y="{y(h)+dy}" fill="{col}" text-anchor="{anchor}" font-weight="600">{t}</text>')
    kote(0,5,"+5,00","#1a1a1a")
    kote(5.25,4,"+4,00","#1a1a1a",-8,"end")
    kote(5.25,1,"+1,00","#1a1a1a",8)
    kote(9.25,1,"+1,00","#1a1a1a",8)
    kote(12,2.75,"+2,75","#1a1a1a",-8,"end")
    kote(0,1,"+1,00","#b38600",8,"start",22)
    kote(5.25,bestand(5.25),"+2,31","#b38600",8,"start",22)
    kote(9.25,bestand(9.25),"+3,31","#b38600",8,"start",22)
    kote(12,4,"+4,00","#b38600",10,"start",24)
    # Wandhöhen Variante A in den Teilen
    for s0,s1,t in ((0,5.25,"Wandteil 1: H = 5,50 m"),(5.25,9.25,"Wandteil 2: H = 9,00 m"),(9.25,12,"Wandteil 3: H = 8,13 m")):
        n,h=t.split(": ")
        a(f'<text x="{x((s0+s1)/2)}" y="{y(7.4)}" text-anchor="middle" font-weight="700" fill="#1f4e79">{n}</text>')
        a(f'<text x="{x((s0+s1)/2)}" y="{y(7.4)+20}" text-anchor="middle" font-weight="700" fill="#1f4e79">{h}</text>')
    a(f'<text x="{x(6)}" y="{y(6.4)}" text-anchor="middle" fill="#1f4e79" font-size="13">(Variante A, geplantes Gelände maßgebend)</text>')
    # Stationsachse
    for s in (0,5.25,9.25,12):
        lab=f"{s:.2f}".replace(".",",")
        a(f'<text x="{x(s)}" y="{y(-0.8)}" text-anchor="middle" fill="#555" font-size="13">{lab} m</text>')
    # Legende
    ly=y(-1.5)
    a(f'<line x1="{x(-1.5)}" y1="{ly}" x2="{x(-0.6)}" y2="{ly}" stroke="#1a1a1a" stroke-width="3"/><text x="{x(-0.45)}" y="{ly+5}">Geplantes Gelände</text>')
    a(f'<line x1="{x(2.8)}" y1="{ly}" x2="{x(3.7)}" y2="{ly}" stroke="#e0a800" stroke-width="3" stroke-dasharray="10 6"/><text x="{x(3.85)}" y="{ly+5}">Bestand</text>')
    a(f'<line x1="{x(5.9)}" y1="{ly}" x2="{x(6.8)}" y2="{ly}" stroke="#c0392b" stroke-width="2" stroke-dasharray="6 5"/><text x="{x(6.95)}" y="{ly+5}">Wandteilgrenze (in der Ansicht gezeichnet)</text>')
    # Draufsicht
    PY=830
    a(f'<text x="24" y="{PY-30}" font-size="20" font-weight="700" fill="#1a1a1a">Draufsicht: Tiefe der Abstandsfläche vor der Wand (Variante A)</text>')
    D=40 # px je m Tiefe
    a(f'<line x1="{x(0)}" y1="{PY}" x2="{x(12)}" y2="{PY}" stroke="#1a1a1a" stroke-width="5"/>')
    a(f'<text x="{x(-0.2)}" y="{PY+5}" text-anchor="end" font-size="13">Außenwand</text>')
    poly=[(0,3),(5.25,3),(5.25,3.6),(9.25,3.6),(9.25,3.25),(12,3.25)]
    p=" ".join(f"{x(s)},{PY+t*D}" for s,t in poly)
    a(f'<polygon points="{x(0)},{PY} {p} {x(12)},{PY}" fill="#cfe0f3" stroke="#1f4e79" stroke-width="2"/>')
    a(f'<line x1="{x(0)}" y1="{PY+3*D}" x2="{x(12)}" y2="{PY+3*D}" stroke="#555" stroke-width="1.5" stroke-dasharray="4 4"/>')
    for s0,s1,t in ((0,5.25,"T = 3,00 m"),(5.25,9.25,"T = 3,60 m"),(9.25,12,"T = 3,25 m")):
        a(f'<text x="{x((s0+s1)/2)}" y="{PY+1.6*D}" text-anchor="middle" font-weight="700" fill="#1f4e79">{t}</text>')
    a(f'<text x="{x(12.2)}" y="{PY+3*D+5}" font-size="13" fill="#555">3,00 m ohne Wandteile</text>')
    a(f'<text x="{x(12.2)}" y="{PY+3*D+22}" font-size="13" fill="#555">(Kantenmittel H = 6,13 m)</text>')
    a('</svg>')
    SVG = "\n".join(out)
    return SVG


def chromium():
    for k in (os.environ.get("BAUPROJEKT_CHROMIUM"), "/opt/pw-browsers/chromium", "chromium", "chromium-browser", "google-chrome"):
        if not k:
            continue
        p = Path(k)
        if p.is_dir():
            treffer = sorted(p.glob("**/chrome"))
            if treffer:
                return str(treffer[0])
        elif p.is_file() or shutil.which(k):
            return k
    raise SystemExit("Chromium nicht gefunden (BAUPROJEKT_CHROMIUM setzen)")


def main():
    with tempfile.TemporaryDirectory() as d:
        svg = Path(d) / "bild.svg"
        svg.write_text(zeichnen())
        subprocess.run(
            [chromium(), "--headless", "--no-sandbox", "--hide-scrollbars", "--force-device-scale-factor=1.5",
             f"--window-size={W},{H}", f"--screenshot={ZIEL}", svg.as_uri()],
            check=True, capture_output=True,
        )
    if shutil.which("pngquant"):
        subprocess.run(["pngquant", "--quality", "80-95", "--strip", "--force", "--ext", ".png", str(ZIEL)])
    print(ZIEL)


if __name__ == "__main__":
    main()
