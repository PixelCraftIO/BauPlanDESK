<script lang="ts">
  /** 3D-Ansicht eines IFC-Modells mit web-ifc (MPL-2.0) und Three.js (MIT).
   * Nur Darstellung und Auswahl: Auswertungen rechnet der Core. Beide
   * Bibliotheken werden erst beim Öffnen dieser Ansicht geladen. */
  import { onMount } from "svelte";
  let { laden, auswahl = "", markiert = [], ausgeblendet = [], onauswahl }: {
    /** Liefert die Modelldatei als Base64. */
    laden: () => Promise<string>;
    auswahl?: string;
    markiert?: string[];
    ausgeblendet?: string[];
    onauswahl?: (globalId: string) => void;
  } = $props();
  let host = $state<HTMLDivElement>();
  let status = $state("3D-Modell wird geladen …"), fehler = $state("");
  type Steuerung = { faerben: (auswahl: string, markiert: Set<string>, aus: Set<string>) => void; blick: (art: "gesamt" | "oben" | "sued") => void; ende: () => void };
  let steuerung = $state<Steuerung | null>(null);
  $effect(() => { steuerung?.faerben(auswahl, new Set(markiert), new Set(ausgeblendet)); });
  onMount(() => {
    let weg = false;
    void starten(() => weg).then(s => { if (weg) s?.ende(); else steuerung = s; }).catch(e => { if (!weg) { fehler = `3D-Ansicht nicht verfügbar: ${e instanceof Error ? e.message : String(e)}`; status = ""; } });
    return () => { weg = true; steuerung?.ende(); };
  });

  function klassenfarbe(W: typeof import("web-ifc"), typ: number): string {
    if (typ === W.IFCWALL || typ === W.IFCWALLSTANDARDCASE) return "#d9d4cc";
    if (typ === W.IFCSLAB) return "#a9b0b8";
    if (typ === W.IFCROOF) return "#a4553f";
    if (typ === W.IFCSPACE) return "#6fa8dc";
    if (typ === W.IFCWINDOW) return "#9fc5e8";
    if (typ === W.IFCDOOR) return "#8d6e63";
    return "#c9c2b6";
  }
  async function starten(abgebrochen: () => boolean): Promise<Steuerung | null> {
    const [THREE, { OrbitControls }, WebIFC, b64] = await Promise.all([
      import("three"), import("three/examples/jsm/controls/OrbitControls.js"), import("web-ifc"), laden(),
    ]);
    if (abgebrochen() || !host) return null;
    const bytes = Uint8Array.from(atob(b64), c => c.charCodeAt(0));
    const ifc = new WebIFC.IfcAPI();
    ifc.SetWasmPath("./web-ifc/");
    await ifc.Init(undefined, true);
    const modell = ifc.OpenModel(bytes, { COORDINATE_TO_ORIGIN: true });
    const szene = new THREE.Scene();
    szene.background = new THREE.Color(getComputedStyle(host).getPropertyValue("--leiste").trim() || "#f1f3f5");
    szene.add(new THREE.AmbientLight(0xffffff, 1.6));
    const sonne = new THREE.DirectionalLight(0xffffff, 1.8); sonne.position.set(30, 60, 40); szene.add(sonne);
    const teile = new Map<string, import("three").Mesh[]>();
    const gruppe = new THREE.Group();
    ifc.StreamAllMeshes(modell, mesh => {
      const zeile = ifc.GetLine(modell, mesh.expressID);
      const gid = String(zeile?.GlobalId?.value ?? mesh.expressID);
      const typ = Number(ifc.GetLineType(modell, mesh.expressID));
      // Räume halb durchsichtig, damit Bauteile sichtbar bleiben.
      const raum = typ === WebIFC.IFCSPACE;
      for (let i = 0; i < mesh.geometries.size(); i++) {
        const pg = mesh.geometries.get(i);
        const g = ifc.GetGeometry(modell, pg.geometryExpressID);
        const v = ifc.GetVertexArray(g.GetVertexData(), g.GetVertexDataSize());
        const ix = ifc.GetIndexArray(g.GetIndexData(), g.GetIndexDataSize());
        const pos = new Float32Array(v.length / 2), nor = new Float32Array(v.length / 2);
        for (let k = 0; k < v.length / 6; k++) { pos.set(v.subarray(k * 6, k * 6 + 3), k * 3); nor.set(v.subarray(k * 6 + 3, k * 6 + 6), k * 3); }
        const geo = new THREE.BufferGeometry();
        geo.setAttribute("position", new THREE.BufferAttribute(pos, 3));
        geo.setAttribute("normal", new THREE.BufferAttribute(nor, 3));
        geo.setIndex(new THREE.BufferAttribute(ix, 1));
        // Modelle ohne Darstellungsstil kommen weiß: dann nach Klasse einfärben.
        const ohneStil = pg.color.x === 1 && pg.color.y === 1 && pg.color.z === 1;
        const farbe = new THREE.Color(ohneStil ? klassenfarbe(WebIFC, typ) : new THREE.Color(pg.color.x, pg.color.y, pg.color.z));
        const deck = raum ? 0.18 : pg.color.w;
        const mat = new THREE.MeshLambertMaterial({ color: farbe, transparent: deck < 1, opacity: deck, side: THREE.DoubleSide, depthWrite: deck >= 1 });
        const m = new THREE.Mesh(geo, mat);
        m.matrix.fromArray(pg.flatTransformation); m.matrixAutoUpdate = false;
        m.userData = { gid, farbe: farbe.clone(), deck };
        gruppe.add(m);
        const l = teile.get(gid) ?? []; l.push(m); teile.set(gid, l);
        g.delete();
      }
    });
    ifc.CloseModel(modell);
    szene.add(gruppe);
    // Weltmatrizen vor dem Umfang: web-ifc liefert Einheitenskalierung in der Matrix.
    gruppe.updateMatrixWorld(true);
    const box = new THREE.Box3().setFromObject(gruppe);
    const mitte = box.getCenter(new THREE.Vector3()), groesse = Math.max(1, box.getSize(new THREE.Vector3()).length());
    const kamera = new THREE.PerspectiveCamera(45, 1, groesse / 1000, groesse * 20);
    const renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true });
    renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
    host.appendChild(renderer.domElement);
    renderer.domElement.setAttribute("aria-label", "3D-Ansicht des Modells");
    renderer.domElement.setAttribute("role", "img");
    const steuer = new OrbitControls(kamera, renderer.domElement);
    const zeichnen = () => renderer.render(szene, kamera);
    steuer.addEventListener("change", zeichnen);
    const blick = (art: "gesamt" | "oben" | "sued") => {
      // web-ifc liefert y nach oben. Abstand so, dass die Umkugel des Modells
      // in den engeren der beiden Öffnungswinkel passt.
      const halb = THREE.MathUtils.degToRad(kamera.fov / 2);
      const eng = Math.min(halb, Math.atan(Math.tan(halb) * kamera.aspect));
      const d = (groesse / 2) / Math.sin(eng) * 1.05;
      const richtung = art === "oben" ? new THREE.Vector3(0, 1, 0.0001) : art === "sued" ? new THREE.Vector3(0, 0.25, 1) : new THREE.Vector3(1, 0.8, 1.2);
      kamera.position.copy(mitte).add(richtung.normalize().multiplyScalar(d));
      steuer.target.copy(mitte); steuer.update(); zeichnen();
    };
    const groesseSetzen = () => {
      if (!host) return;
      const w = Math.max(200, host.clientWidth), h = Math.max(200, host.clientHeight);
      renderer.setSize(w, h, false); renderer.domElement.style.width = "100%"; renderer.domElement.style.height = "100%";
      kamera.aspect = w / h; kamera.updateProjectionMatrix(); zeichnen();
    };
    const beobachter = new ResizeObserver(groesseSetzen); beobachter.observe(host);
    groesseSetzen(); blick("gesamt");
    // Klick ohne Ziehen wählt das getroffene Element.
    const strahl = new THREE.Raycaster(); let unten: [number, number] | null = null;
    const druck = (e: PointerEvent) => { unten = [e.clientX, e.clientY]; };
    const los = (e: PointerEvent) => {
      if (!unten || Math.hypot(e.clientX - unten[0], e.clientY - unten[1]) > 4) return;
      const r = renderer.domElement.getBoundingClientRect();
      strahl.setFromCamera(new THREE.Vector2(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1), kamera);
      const treffer = strahl.intersectObjects(gruppe.children.filter(c => c.visible), false);
      // Undurchsichtige Bauteile vor Räumen.
      const t = treffer.find(t => (t.object.userData.deck ?? 1) >= 1) ?? treffer[0];
      if (t) onauswahl?.(t.object.userData.gid);
    };
    renderer.domElement.addEventListener("pointerdown", druck);
    renderer.domElement.addEventListener("pointerup", los);
    status = "";
    // Für Prüfungen und Hilfetexte: Anzahl dargestellter Elemente.
    host.dataset.elemente = String(teile.size);
    return {
      faerben(aus, mark, versteckt) {
        for (const [gid, ms] of teile) for (const m of ms) {
          const mat = m.material as import("three").MeshLambertMaterial;
          m.visible = !versteckt.has(gid);
          mat.color.copy(gid === aus ? new THREE.Color("#1d6fd6") : mark.has(gid) ? new THREE.Color("#d98a00") : m.userData.farbe);
          const deck = gid === aus ? Math.max(0.6, m.userData.deck) : m.userData.deck;
          mat.opacity = deck; mat.transparent = deck < 1;
        }
        zeichnen();
      },
      blick,
      ende() {
        beobachter.disconnect(); steuer.dispose();
        renderer.domElement.removeEventListener("pointerdown", druck);
        renderer.domElement.removeEventListener("pointerup", los);
        gruppe.traverse(o => { const m = o as import("three").Mesh; m.geometry?.dispose(); (m.material as import("three").Material | undefined)?.dispose?.(); });
        renderer.dispose(); renderer.domElement.remove();
      },
    };
  }
</script>

<div class="ansicht3d">
  <div class="blicke" role="toolbar" aria-label="Blickrichtung">
    <button type="button" class="sekundaer" disabled={!steuerung} onclick={() => steuerung?.blick("gesamt")}>Gesamt</button>
    <button type="button" class="sekundaer" disabled={!steuerung} onclick={() => steuerung?.blick("oben")}>Draufsicht</button>
    <button type="button" class="sekundaer" disabled={!steuerung} onclick={() => steuerung?.blick("sued")}>Ansicht Süd</button>
  </div>
  <div class="flaeche" bind:this={host}>
    {#if status}<p class="meta zustand" role="status">{status}</p>{/if}
    {#if fehler}<p class="fehlertext zustand" role="alert">{fehler}</p>{/if}
  </div>
</div>

<style>
  .ansicht3d{display:flex;flex-direction:column;height:100%;min-height:0}
  .blicke{display:flex;gap:4px;padding:4px;border-bottom:1px solid var(--border-innen);background:var(--leiste)}
  .blicke button{min-height:24px;padding:0 8px;font-size:.85rem}
  .flaeche{position:relative;flex:1;min-height:320px;background:var(--leiste)}
  .flaeche :global(canvas){display:block}
  .zustand{position:absolute;inset:auto 0 0 0;margin:0;padding:6px 10px;background:var(--panel);border-top:1px solid var(--border-innen)}
</style>
