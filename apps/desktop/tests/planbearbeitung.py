"""UI-Verträge der erweiterten Planbearbeitung; Fachberechnung wird separat getestet."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich
from plan_fixture import pdf_fixture, custom

root=Path(__file__).resolve().parent
fixture=json.loads((root/"fixtures/projekt.json").read_text(encoding="utf-8"))
fixture["name"]="Planbearbeitung – Regression"
out=root.parents[2]/"target/planbearbeitung"
out.mkdir(parents=True,exist_ok=True)
errors=[]
with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,executable_path=os.environ.get("BAUPROJEKT_CHROMIUM",r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"))
    page=browser.new_page(viewport={"width":1600,"height":1150})
    page.on("pageerror",lambda e:errors.append(str(e)))
    page.add_init_script("window.fixture="+json.dumps(fixture)+";window.planPdf="+json.dumps(pdf_fixture())+";"+(root/"tauri-mock.js").read_text(encoding="utf-8")+custom)
    page.goto(os.environ.get("BAUPROJEKT_UI_URL","http://127.0.0.1:1420"))
    page.locator(".projekttabelle").get_by_role("button",name=fixture["name"],exact=True).click()
    bereich(page,"Aufmaß")
    scope=page.locator(".projekt-sitzung:visible")
    scope.get_by_role("button",name="Neu anlegen",exact=True).click()
    svg=scope.get_by_role("application")
    expect(svg).to_be_visible()
    expect(scope.get_by_role("button",name="1:1",exact=True)).to_be_enabled()
    scope.get_by_role("button",name="1:1",exact=True).click()
    scope.get_by_role("button",name="Maßstab setzen",exact=True).click()
    scope.get_by_label("Punktfang",exact=True).uncheck()
    def tab(name): scope.get_by_role("tablist",name="Planpanel").get_by_role("tab",name=name,exact=True).click()
    def blatt(sc=None):
        sc=sc or scope
        sc.get_by_role("tablist",name="Planpanel").get_by_role("tab",name="Eigenschaften",exact=True).click()
        zb=sc.get_by_role("button",name="Zum Blatt",exact=True)
        if zb.count():zb.click()
    def click(x,y):
        svg.scroll_into_view_if_needed()
        q=svg.evaluate("""async(svg,p)=>{
          let q=new DOMPoint(p[0],p[1]).matrixTransform(svg.getScreenCTM());
          const s=svg.closest('.plan-scroll'),r=s.getBoundingClientRect();
          s.scrollLeft+=q.x-r.left-r.width/2;s.scrollTop+=q.y-r.top-r.height/2;
          await new Promise(requestAnimationFrame);
          q=new DOMPoint(p[0],p[1]).matrixTransform(svg.getScreenCTM());
          const hit=document.elementFromPoint(q.x,q.y);
          if(!hit||!(svg===hit||svg.contains(hit)))throw Error('Planpunkt verdeckt: '+JSON.stringify({x:q.x,y:q.y,hit:hit?.outerHTML}));
          return {x:q.x,y:q.y};
        }""",[x,y])
        page.mouse.click(q["x"],q["y"])
    def speichern():
        button=scope.get_by_role("button",name="Speichern",exact=True)
        if button.is_enabled(): button.click()
        expect(scope.get_by_label("Speicherstatus",exact=True)).to_contain_text("Gespeichert")
        expect(scope.get_by_role("button",name="1:1",exact=True)).to_be_enabled()
    scope.get_by_role("button",name="Rechteck",exact=True).click();click(90,80);click(240,190)
    expect(scope.get_by_role("heading",name="Eigenschaften",exact=True)).to_be_visible()
    name=scope.locator("aside").get_by_label("Bezeichnung",exact=True)
    name.fill("Fehler")
    expect(scope.locator("aside").get_by_role("alert")).to_contain_text("Kontur pruefen")
    name.fill("Büro EG")
    expect(scope.locator("aside").get_by_role("alert")).to_have_count(0)
    speichern()
    first=page.evaluate("structuredClone(window.planSaved)")
    # Eigenschaften und Kalibrierung gehören ebenfalls zum vollständigen Undo.
    name.fill("Anderer Name")
    scope.get_by_role("button",name="Rückgängig",exact=True).click()
    expect(name).to_have_value("Büro EG")
    page.keyboard.press("Control+Shift+z")
    expect(name).to_have_value("Anderer Name")
    scope.get_by_role("button",name="Rückgängig",exact=True).click()
    # Maßeingabe übernimmt Geometrie, löst aber kein äußeres Speichern aus.
    scope.get_by_role("button",name="Rechteck",exact=True).click();click(100,300)
    page.keyboard.type("2;1");expect(scope.get_by_label("Maßeingabe",exact=True)).to_have_value("2;1")
    before=page.evaluate("window.planSaved.revision")
    page.keyboard.press("Enter")
    expect(scope.get_by_label("Maßeingabe",exact=True)).to_have_count(0)
    assert page.evaluate("window.planSaved.revision")==before
    assert scope.locator("form form").count()==0
    speichern()
    assert len(page.evaluate("window.planSaved.daten.markierungen"))==2
    # Mehrfachauswahl und gemeinsame Verschiebung.
    tab("Objekte")
    scope.get_by_label("Büro EG zur Auswahl hinzufügen",exact=True).check()
    # Die Paletten rechts sind einzeln aufgeklappt; die Mehrfachauswahl steht unter „Eigenschaften“.
    tab("Eigenschaften")
    expect(scope.get_by_role("heading",name="2 Markierungen ausgewählt")).to_be_visible()
    old=page.evaluate("window.planSaved.daten.markierungen.map(m=>m.geometrie.von[0])")
    svg.focus();page.keyboard.press("ArrowRight");speichern()
    assert page.evaluate("window.planSaved.daten.markierungen.map(m=>m.geometrie.von[0])")==[x+1 for x in old]
    # Baum: Ebene „Aufmaß“ mit beiden Markierungen; die Ebene zeigt ihre Eigenschaften.
    tab("Objekte");baum=scope.get_by_role("tree",name="Ebenen und Objekte")
    expect(baum.get_by_role("treeitem",name="Ebene Aufmaß")).to_have_attribute("aria-expanded","true")
    baum.get_by_role("button",name="Aufmaß",exact=True).click()
    expect(scope.locator('section.palette[aria-label="Eigenschaften der Ebene"]')).to_be_visible()
    expect(scope.get_by_label("Ebenenname",exact=True)).to_have_value("Aufmaß")
    scope.get_by_role("button",name="Zum Blatt",exact=True).click()
    expect(scope.locator('section.palette[aria-label="Blatt und Einrichtung"]')).to_be_visible()
    # Ausschnitt anlegen und eine darin gebundene Kontur speichern.
    tab("Objekte");scope.locator("aside").get_by_role("button",name="Ausschnitt anlegen",exact=True).click();click(50,40);click(550,350)
    scope.get_by_label("Ausschnittname",exact=True).fill("Schnitt B-B")
    scope.locator(".ausschnitt").get_by_role("button",name="Öffnen",exact=True).click()
    scope.get_by_role("button",name="1:1",exact=True).click()
    scope.get_by_role("button",name="Rechteck",exact=True).click();click(300,100);click(450,240)
    name.fill("Kontur im Ausschnitt");speichern()
    saved=page.evaluate("structuredClone(window.planSaved)")
    assert saved["daten"]["ausschnitte"][0]["ebene_id"]
    ref=saved["daten"]["markierungen"][-1]["ausschnitt_id"]
    tab("Objekte");scope.locator(".ausschnitt-eigenschaften").get_by_role("button",name="Entfernen",exact=True).click()
    scope.get_by_role("button",name="Rückgängig",exact=True).click();speichern()
    restored=page.evaluate("window.planSaved.daten")
    assert restored["ausschnitte"][0]["id"]==ref and restored["markierungen"][-1]["ausschnitt_id"]==ref
    # Ein abgebrochener Rahmen-Drag verändert keine Daten.
    tab("Objekte");scope.locator(".ausschnitt").get_by_role("button",name="Ausschnitt Schnitt B-B wählen",exact=True).click()
    handle=svg.locator("[data-rahmenecke='0']")
    b=handle.bounding_box();assert b
    page.mouse.move(b["x"]+b["width"]/2,b["y"]+b["height"]/2);page.mouse.down();page.mouse.move(b["x"]+40,b["y"]+30)
    page.keyboard.press("Escape");page.mouse.up()
    expect(svg.locator(".rahmen > rect").first).to_have_attribute("x",str(saved["daten"]["ausschnitte"][0]["von"][0]).removesuffix(".0"))
    # Seitenminiaturen dienen der Betrachtung. Erst eine eigene Aktion legt ein neues Aufmaß an.
    scope.get_by_role("button",name="Seiten (2)",exact=True).click()
    scope.get_by_role("button",name="Seite 2",exact=True).click()
    blatt()
    expect(scope.get_by_label("Seite",exact=True)).to_have_value("1")
    expect(scope.get_by_text("Betrachtung von Seite 2.",exact=False)).to_be_visible()
    scope.get_by_role("button",name="Eigenes Aufmaß für diese Seite anlegen …",exact=True).click()
    expect(scope.get_by_label("Seite",exact=True)).to_have_value("2")
    speichern()
    assert len(page.evaluate("window.planSaved.daten.markierungen"))==0
    assert page.evaluate("window.planSaves.length")==2
    assert len(page.evaluate("window.planSaves.find(x=>x.objekt!==window.planSaved.objekt).daten.markierungen"))==3
    # Kreis, Zoom und Übernahmevorschau mit ausdrücklichem Speichern des Ziels.
    scope.get_by_role("button",name="Maßstab setzen",exact=True).click()
    scope.get_by_role("button",name="1:1",exact=True).click()
    scope.get_by_label("Punktfang",exact=True).uncheck()
    scope.get_by_role("button",name="Kreis",exact=True).click();click(320,150);click(350,150)
    name.fill("Gebäudeprojektion")
    scope.get_by_label("Bedeutung",exact=True).select_option("gebaeudegrundflaeche")
    scope.get_by_label("Gebäude",exact=True).select_option("7a1c0d4e-1b2c-4d5e-8f90-123456789abc")
    speichern();tab("Aufgaben")
    scope.get_by_role("button",name="Übernehmen",exact=True).click()
    dialog=page.get_by_role("dialog")
    expect(dialog.get_by_role("table",name="Vorschau der Aufmaßübernahme")).to_contain_text("50 m²")
    assert page.evaluate("window.gebaeudeSaved")==None
    dialog.get_by_role("button",name="Übernehmen und speichern",exact=True).click()
    page.wait_for_function("window.gebaeudeSaved?.daten.geometrie.grundflaeche_m2===50")
    bound=page.evaluate("window.gebaeudeSaved.daten.planbindungen[0]")
    assert bound["aufmass_id"]==page.evaluate("window.planSaved.objekt")
    scope.get_by_role("button",name="Vergrößern",exact=True).click()
    expect(scope.locator(".plan-kopf .zoom")).to_have_text("125 %")
    scope.get_by_role("button",name="1:1",exact=True).click()
    scope.get_by_role("button",name="Vollbild",exact=True).click()
    expect(scope.get_by_role("button",name="Aufmaß speichern",exact=True)).to_be_visible()
    page.screenshot(path=str(out/"planeditor-vollbild.png"),full_page=True)
    scope.get_by_role("button",name="Vollbild beenden",exact=True).click()
    # Vergleich und Übertragung erhalten die ursprüngliche Markierung/Verknüpfung.
    original_marker=page.evaluate("window.planSaved.daten.markierungen[0].id")
    scope.get_by_role("button",name="Prüfen ▾",exact=True).click()
    scope.get_by_role("menuitem",name="Planstände vergleichen …",exact=True).click()
    expect(scope.get_by_role("dialog",name="Planstände vergleichen")).to_be_visible()
    scope.get_by_label("Vergleichsunterlage",exact=True).select_option("d316da8b-cbfb-4ae6-931b-8765fb6454b2|1")
    scope.get_by_label("Blattlage und Konturen anhand der Überlagerung geprüft",exact=True).check()
    transfer=scope.get_by_role("button",name="Als neues Aufmaß auf Vergleichsunterlage übertragen …",exact=True)
    expect(transfer).to_be_enabled();transfer.click();speichern()
    assert page.evaluate("window.planSaved.daten.dokument.dokument_id")=="d316da8b-cbfb-4ae6-931b-8765fb6454b2"
    assert page.evaluate("window.planSaved.daten.kalibrierung")==None
    assert page.evaluate("window.planSaved.daten.markierungen[0].id")!=original_marker
    assert page.evaluate("window.gebaeudeSaved.daten.planbindungen[0].markierung_id")==original_marker
    assert page.evaluate("window.planSaves.length")==3
    page.screenshot(path=str(out/"planeditor.png"),full_page=True)
    # Leserecht lässt Zoom, Seitennavigation, Auswahl und lokale Sichtbarkeit zu.
    seed=page.evaluate("structuredClone(window.planSaved)")
    read=browser.new_page(viewport={"width":1600,"height":1150})
    read.on("pageerror",lambda e:errors.append(str(e)))
    read.add_init_script("window.fixture="+json.dumps(fixture)+";window.planPdf="+json.dumps(pdf_fixture())+";"+(root/"tauri-mock.js").read_text(encoding="utf-8")+custom+";window.planSaved="+json.dumps(seed)+";window.planSaves=[window.planSaved];window.testUser.rolle='lesen';")
    read.goto(os.environ.get("BAUPROJEKT_UI_URL","http://127.0.0.1:1420"))
    read.locator(".projekttabelle").get_by_role("button",name=fixture["name"],exact=True).click();bereich(read,"Aufmaß")
    rs=read.locator(".projekt-sitzung:visible");rs.get_by_role("button",name="Öffnen",exact=True).click()
    expect(rs.get_by_role("button",name="1:1",exact=True)).to_be_enabled()
    expect(rs.get_by_role("button",name="Speichern",exact=True)).to_be_disabled()
    rs.get_by_role("tablist",name="Planpanel").get_by_role("tab",name="Objekte",exact=True).click()
    auge=rs.get_by_role("tree",name="Ebenen und Objekte").get_by_role("button",name="Ebene Aufmaß sichtbar",exact=True)
    auge.click();expect(auge).to_have_attribute("aria-pressed","false")
    expect(rs.get_by_role("button",name="Speichern",exact=True)).to_be_disabled()
    auge.click();expect(auge).to_have_attribute("aria-pressed","true")
    rs.get_by_role("button",name="Seiten (2)",exact=True).click();rs.get_by_role("button",name="Seite 1",exact=True).click()
    blatt(rs)
    expect(rs.get_by_label("Seite",exact=True)).to_have_value("2")
    expect(rs.get_by_role("button",name="Zum Aufmaßblatt zurück",exact=True)).to_be_visible()
    read.close()
    assert not errors,errors
    browser.close()
print("Planbearbeitung: vollständiges Undo/Redo, Einzelfehler, Maßeingabe, Mehrfachauswahl, Ausschnitt-IDs, Cancel und sichere Blattwechsel erfolgreich.")
