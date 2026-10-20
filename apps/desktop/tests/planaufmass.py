"""Browserprüfung der Zeichenmaske; Core-Fachwerte separat in storage/tests/planaufmass.rs."""
from pathlib import Path
import base64, json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich

root = Path(__file__).resolve().parent
fixture = json.loads((root / "fixtures/projekt.json").read_text(encoding="utf-8"))
fixture["name"] = "PDF-Aufmaß – Browserprüfung"
out = Path(os.environ.get("BAUPROJEKT_PLAN_UI_ARTEFAKTE", str(root.parents[2] / "target/planaufmass-ui")))
out.mkdir(parents=True, exist_ok=True)

def pdf_fixture():
    stream = b"0.6 w 0.3 0.4 0.5 RG 70 70 300 200 re S 220 70 m 220 270 l S 70 170 m 220 170 l S"
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 400] /Resources << >> /Contents 4 0 R >>",
        b"<< /Length " + str(len(stream)).encode() + b" >>\nstream\n" + stream + b"\nendstream",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 400] /Resources << >> /Contents 4 0 R >>",
    ]
    data = b"%PDF-1.4\n"
    offsets = [0]
    for i, obj in enumerate(objects, 1):
        offsets.append(len(data))
        data += f"{i} 0 obj\n".encode() + obj + b"\nendobj\n"
    start = len(data)
    data += f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode()
    data += b"".join(f"{o:010d} 00000 n \n".encode() for o in offsets[1:])
    data += f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF".encode()
    return base64.b64encode(data).decode()

custom = r"""
const original=window.__TAURI_INTERNALS__.invoke;
window.planSaved=null;
const doc={document_id:"b316da8b-cbfb-4ae6-931b-8765fb6454b1",project_id:window.fixture.id,version:1,file_name:"Grundrisse.pdf",sha256:"0".repeat(64),size_bytes:1000,created_at:window.fixture.created_at,created_by:"Test",storage_key:"test.pdf"};
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==="dokumente_auflisten")return [doc];
 if(cmd==="pdf_vorschau")return {dokument:doc,pdf_base64:window.planPdf,felder:[],seiten:[1,2].map(n=>({nummer:n,breite_mm:211,hoehe_mm:141,rotation:0,platzierbar:true}))};
 if(cmd==="fachobjekte"&&args.art==="planaufmass")return window.planSaved?[structuredClone(window.planSaved)]:[];
 if(cmd==="planaufmass_pruefen")return args.plan.markierungen.map(m=>({id:m.id,laenge_m:args.plan.kalibrierung?30:null,flaeche_m2:args.plan.kalibrierung?50:null,horizontale_m:null,hoehe_m:null,neigung_grad:null,offen:[]}));
 if(cmd==="fachobjekt_speichern"&&args.art==="planaufmass"){
   window.planSaved={projekt_id:args.projektId,modul:args.art,objekt:args.id,revision:args.basis+1,schema_version:1,daten:structuredClone(args.daten)};
   return structuredClone(window.planSaved);
 }
 return original(cmd,args);
};
"""

errors = []
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, executable_path=os.environ.get("BAUPROJEKT_CHROMIUM", r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"))
    page = browser.new_page(viewport={"width": 1600, "height": 1150})
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.add_init_script("window.fixture=" + json.dumps(fixture) + ";window.planPdf=" + json.dumps(pdf_fixture()) + ";" + (root / "tauri-mock.js").read_text(encoding="utf-8") + custom)
    page.goto(os.environ.get("BAUPROJEKT_UI_URL", "http://127.0.0.1:1432"))
    page.locator(".projekttabelle").get_by_role("button", name=fixture["name"], exact=True).click()
    bereich(page, "PDF-Aufmaß")
    scope = page.locator(".projekt-sitzung:visible")
    scope.get_by_role("button", name="Neu anlegen", exact=True).click()
    svg = scope.get_by_role("application")
    expect(svg).to_be_visible()
    scope.get_by_role("button", name="Maßstab setzen", exact=True).click()
    scope.get_by_label("Punktfang", exact=True).uncheck()
    scope.get_by_role("button", name="Rechteck", exact=True).click()
    svg.click(position={"x": 90, "y": 80})
    svg.click(position={"x": 240, "y": 190})
    expect(scope.locator(".markierungen button")).to_have_count(1)
    page.screenshot(path=str(out / "nach-zeichnen.png"), full_page=True)
    assert scope.locator(".pdf-plan img").evaluate("(img)=>{const c=document.createElement('canvas');c.width=img.naturalWidth;c.height=img.naturalHeight;const ctx=c.getContext('2d');ctx.drawImage(img,0,0);const d=ctx.getImageData(0,0,c.width,c.height).data;let n=0;for(let i=0;i<d.length;i+=4)if(d[i]<200&&d[i+3]>0)n++;return n}") > 1000
    scope.get_by_label("Bedeutung", exact=True).select_option("nuf")
    scope.locator(".arbeitsplatz aside").get_by_label("Bezeichnung", exact=True).fill("Büro EG")
    expect(scope.locator(".messwerte")).to_contain_text("50")
    scope.get_by_role("button", name="Zwei Punkte kalibrieren", exact=True).click()
    svg.click(position={"x": 70, "y": 330})
    svg.click(position={"x": 170, "y": 330})
    scope.get_by_label("Bezugshöhe (m)", exact=True).fill("100")
    scope.get_by_role("button", name="Bezugspunkt setzen", exact=True).click()
    svg.click(position={"x": 70, "y": 330})
    expect(scope.locator(".markierungen button")).to_have_count(1)
    # Eine Markierung ist eine Maske; nur Fachstand speichern, kein PDF-Schreibaufruf.
    scope.get_by_role("button", name="Speichern", exact=True).click()
    page.wait_for_function("window.planSaved?.daten.markierungen[0]?.bezeichnung==='Büro EG'")
    assert page.evaluate("window.planSaved.daten.markierungen[0].bedeutung") == "nuf"
    calibration = page.evaluate("window.planSaved.daten.kalibrierung")
    assert calibration["art"] == "strecke" and calibration["meter"] == 10
    # Browser-Pointer liegen auf Subpixeln; gespeicherte Werte nicht dafür runden.
    assert all(abs(a-b)<0.02 for a,b in zip(calibration["von"]+calibration["bis"], [70,330,170,330])), calibration
    assert page.evaluate("window.planSaved.daten.hoehenbezug.hoehe_m") == 100
    assert all(abs(a-b)<0.02 for a,b in zip(page.evaluate("window.planSaved.daten.hoehenbezug.punkt"), [70,330]))
    expect(scope.get_by_label("Speicherstatus")).to_have_text("Gespeicherter Stand")
    # Sichtbarkeit und Sperren, weitere Ebene und Rückgängig.
    scope.locator(".ebene").get_by_label("An", exact=True).uncheck()
    expect(svg.locator("[data-markierung]")).to_have_count(0)
    scope.locator(".ebene").get_by_label("An", exact=True).check()
    scope.get_by_role("button", name="Ebene hinzufügen", exact=True).click()
    scope.get_by_role("button", name="Vergrößern", exact=True).click()
    expect(scope.locator(".plan-kopf")).to_contain_text("125 %")
    scope.get_by_label("Punktfang", exact=True).uncheck()
    scope.get_by_role("button", name="Kreis", exact=True).click()
    svg.click(position={"x": 340, "y": 150})
    svg.click(position={"x": 370, "y": 150})
    expect(scope.locator(".markierungen button")).to_have_count(2)
    scope.get_by_role("button", name="Rückgängig", exact=True).click()
    expect(scope.locator(".markierungen button")).to_have_count(1)
    scope.get_by_role("button", name="Wiederholen", exact=True).click()
    expect(scope.locator(".markierungen button")).to_have_count(2)
    scope.locator(".ebene").nth(1).get_by_label("Sperre", exact=True).check()
    expect(scope.locator(".arbeitsplatz aside").get_by_label("Bedeutung", exact=True)).to_be_disabled()
    scope.locator(".ebene").nth(1).get_by_label("Sperre", exact=True).uncheck()
    # Strg+Mausrad zoomt um den Mauszeiger; Vollbild füllt das Fenster, Escape beendet es.
    svg.hover(position={"x": 100, "y": 100})
    page.keyboard.down("Control"); page.mouse.wheel(0, -300); page.keyboard.up("Control")
    expect(scope.locator(".plan-kopf .zoom")).not_to_have_text("125 %")
    scope.get_by_role("button", name="Vollbild", exact=True).click()
    expect(scope.locator(".editor.vollbild")).to_be_visible()
    assert page.evaluate("document.querySelector('.editor.vollbild').getBoundingClientRect().height") > 1000
    page.keyboard.press("Escape")
    expect(scope.locator(".editor.vollbild")).to_have_count(0)
    scope.get_by_role("button", name="1:1", exact=True).click()
    expect(scope.locator(".plan-kopf .zoom")).to_have_text("100 %")
    # Ausschnitt: Rahmen aufziehen, als Schnitt kennzeichnen, Bezugslinie setzen, darin einen Höhenpunkt zeichnen.
    scope.get_by_role("button", name="Ausschnitt anlegen", exact=True).click()
    svg.click(position={"x": 300, "y": 40})
    svg.click(position={"x": 560, "y": 300})
    expect(scope.locator(".ausschnitt")).to_have_count(1)
    scope.get_by_label("Ausschnittname", exact=True).fill("Schnitt B-B")
    scope.get_by_label("Ausschnittart", exact=True).select_option("schnitt")
    scope.locator(".ausschnitt-eigenschaften").get_by_label("Höhe (m)", exact=True).fill("72.64")
    scope.get_by_role("button", name="Bezugslinie setzen", exact=True).click()
    expect(scope.locator(".werkzeuge .pfad")).to_contain_text("Schnitt B-B")
    scope.get_by_role("button", name="1:1", exact=True).click()
    svg.click(position={"x": 320, "y": 250})
    svg.click(position={"x": 540, "y": 250})
    expect(scope.locator(".ausschnitt")).to_contain_text("Bezug 72,64 m ü. NHN")
    page.keyboard.press("h")
    expect(scope.get_by_role("button", name="Höhenpunkt", exact=True)).to_have_attribute("aria-pressed", "true")
    svg.click(position={"x": 400, "y": 150})
    expect(scope.locator(".markierungen button")).to_have_count(1)
    expect(svg.locator(".abdunkeln")).to_have_count(1)
    page.keyboard.press("Escape")
    expect(scope.get_by_role("button", name="Blatt 1", exact=True)).to_be_disabled()
    expect(scope.locator(".markierungen button")).to_have_count(3)
    # Entf entfernt die gewählte Markierung, Strg+Z holt sie zurück.
    scope.locator(".markierungen button").nth(2).click()
    page.keyboard.press("Delete")
    expect(scope.locator(".markierungen button")).to_have_count(2)
    page.keyboard.press("Control+z")
    expect(scope.locator(".markierungen button")).to_have_count(3)
    page.screenshot(path=str(out / "editor.png"), full_page=True)
    scope.get_by_role("button", name="Speichern", exact=True).click()
    page.wait_for_function("window.planSaved?.daten.ausschnitte?.[0]?.bezugslinie?.hoehe_m===72.64")
    ausschnitt = page.evaluate("window.planSaved.daten.ausschnitte[0]")
    assert ausschnitt["art"] == "schnitt" and ausschnitt["bezeichnung"] == "Schnitt B-B"
    assert all(abs(a-b)<0.02 for a,b in zip(ausschnitt["von"]+ausschnitt["bis"], [300,40,560,300])), ausschnitt
    assert all(abs(a-b)<0.02 for a,b in zip(ausschnitt["bezugslinie"]["von"]+ausschnitt["bezugslinie"]["bis"], [320,250,540,250])), ausschnitt
    assert page.evaluate("window.planSaved.daten.markierungen[2].ausschnitt_id") == ausschnitt["id"]
    assert page.evaluate("window.planSaved.daten.markierungen[2].geometrie.art") == "hoehenpunkt"
    assert "ausschnitt_id" not in page.evaluate("window.planSaved.daten.markierungen[0]")
    assert page.evaluate("window.planSaved.daten.ebenen.some(e=>e.bezeichnung==='Schnitt B-B')")
    # Seitenwechsel darf keinen alten Maßstab für die neue Seite weitergeben.
    scope.get_by_label("Seite", exact=True).fill("2")
    scope.get_by_label("Seite", exact=True).press("Tab")
    expect(scope.get_by_text("Maßstab fehlt", exact=True)).to_be_visible()
    scope.get_by_role("button", name="Speichern", exact=True).click()
    page.wait_for_function("window.planSaved?.daten.seite===2&&window.planSaved.daten.kalibrierung===null")
    assert page.evaluate("window.planSaved.daten.hoehenbezug") is None
    # Rahmen bleiben als Vorschlag, ihre Bezüge sind neu zu bestätigen.
    assert page.evaluate("window.planSaved.daten.ausschnitte.length") == 1
    assert "bezugslinie" not in page.evaluate("window.planSaved.daten.ausschnitte[0]")
    circle = page.evaluate("window.planSaved.daten.markierungen[1].geometrie")
    assert abs(circle["rand"][0] - circle["mitte"][0] - 24) < 0.01, circle
    forbidden = ["dokument_ablegen", "formular_im_projekt_speichern", "formular_vorlage_erstellen"]
    assert not page.evaluate("(names)=>window.calls.some(c=>names.includes(c.cmd))", forbidden)
    assert not errors, errors
    browser.close()
print("PDF-Aufmaß: Zeichnen, Ebenen, Sperre, Rückgängig, Zoom, Vollbild, Ausschnitt mit Bezugslinie, Kürzel, Speichern und Seitenwechsel erfolgreich.")
