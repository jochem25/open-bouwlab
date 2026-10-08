/**
 * 3D-controle van de IFC-import: berekende vlakken en openingen uit de
 * engine-response, optioneel boven op het originele IFC-model (transparant).
 *
 * web-ifc alleen voor weergave van het originele model; import loopt via de engine.
 *
 * Techniek
 * - Instantiatiepatroon van de FloorCanvas3D/oude reconstructieviewer:
 *   OBC.Components + Worlds + SimpleScene/Camera/Renderer + Grids + camera-controls.
 * - Coordinaten: thermal-vertices zijn Z-up meters in projectcoordinaten;
 *   world = (x, z, -y) (zie FloorCanvas3D "COORDINATE CONVENTION").
 * - Kleuren: bestaande tokens `--domain-boundary-*` (themes.css), water via de
 *   bestaande 3D-kleur uit FloorCanvas3D (geen token), rood via `--theme-danger-color`.
 * - Origineel model: OBC.IfcLoader (fragments), lazy bij het aanzetten van de laag.
 *   COORDINATE_TO_ORIGIN uit en `coordinate=false`, zodat het model op zijn
 *   projectcoordinaten blijft staan en met de vlakken samenvalt. Na het laden
 *   vergelijken we het middelpunt met dat van de vlakken en melden een afwijking.
 * - Het File-object leeft alleen in paginastate (prop), nooit in een store.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import * as THREE from "three";
import * as OBC from "@thatopen/components";
import fragmentsWorkerUrl from "@thatopen/fragments/worker?url";

import {
  CATEGORY_LABEL,
  modelBbox,
  roomBbox,
  toWorld,
  triangulatePolygon,
  type Bbox,
  type SurfaceCategory,
  type ViewModel,
  type ViewSurface,
} from "../../lib/ifcImportGeometry";

interface Props {
  model: ViewModel;
  /** Het originele IFC-bestand van deze sessie; null na herladen. */
  file: File | null;
  selectedRoomId: string | null;
  selectedSurfaceId: string | null;
  onSelectSurface: (surface: ViewSurface | null) => void;
}

const WASM_PATH = "/wasm/";
const BACKGROUND = 0xf5f5f4;
const SELECT_COLOR = 0xf59e0b;
const DIMMED_OPACITY = 0.18;
const MODEL_OPACITY = 0.15;
const OPENING_OPACITY = 0.65;
const RED_MIX = 0.45;
const ALIGN_TOLERANCE_M = 2;
const CLICK_MOVE_PX = 4;
const WATER_COLOR = "#1a6b8a"; // = BOUNDARY_CONDITION_COLORS.water in FloorCanvas3D

const CATEGORY_VAR: Record<SurfaceCategory, [string, string]> = {
  exterior: ["--domain-boundary-exterior", "#2563eb"],
  ground: ["--domain-boundary-ground", "#92400e"],
  unheated: ["--domain-boundary-unheated", "#7c3aed"],
  adjacent: ["--domain-boundary-adjacent-room", "#16a34a"],
  water: ["", WATER_COLOR],
};

function cssColor(name: string, fallback: string): THREE.Color {
  const v = name
    ? getComputedStyle(document.documentElement).getPropertyValue(name).trim()
    : "";
  return new THREE.Color(v || fallback);
}

function worldBox(b: Bbox): THREE.Box3 {
  const box = new THREE.Box3();
  box.expandByPoint(new THREE.Vector3(...toWorld(b.min)));
  box.expandByPoint(new THREE.Vector3(...toWorld(b.max)));
  return box;
}

interface SurfaceEntry {
  surface: ViewSurface;
  mesh: THREE.Mesh;
  outline: THREE.LineLoop | null;
  base: THREE.Color;
}

function disposeGroup(group: THREE.Group): void {
  for (const child of [...group.children]) {
    group.remove(child);
    const obj = child as THREE.Mesh;
    obj.geometry?.dispose();
    const m = obj.material;
    if (Array.isArray(m)) m.forEach((x) => x.dispose());
    else m?.dispose();
  }
}

function buildGeometry(vertices: [number, number, number][]): THREE.BufferGeometry | null {
  const tri = triangulatePolygon(vertices);
  if (tri.indices.length === 0) return null;
  const pos = new Float32Array(tri.positions.length);
  for (let i = 0; i < tri.positions.length; i += 3) {
    const w = toWorld([tri.positions[i]!, tri.positions[i + 1]!, tri.positions[i + 2]!]);
    pos.set(w, i);
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.BufferAttribute(pos, 3));
  g.setIndex(tri.indices);
  g.computeVertexNormals();
  return g;
}

type ModelState =
  | { phase: "idle" }
  | { phase: "loading" }
  | { phase: "ready"; ms: number; note: string | null; offset: THREE.Vector3 | null }
  | { phase: "error"; message: string };

export function IfcImportViewer3D({
  model,
  file,
  selectedRoomId,
  selectedSurfaceId,
  onSelectSurface,
}: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const componentsRef = useRef<OBC.Components | null>(null);
  const worldRef = useRef<OBC.World | null>(null);
  const surfaceGroup = useRef(new THREE.Group());
  const openingGroup = useRef(new THREE.Group());
  const entries = useRef<SurfaceEntry[]>([]);
  const raycaster = useRef(new THREE.Raycaster());
  const ifcModelRef = useRef<{ object: THREE.Object3D } | null>(null);
  const downPos = useRef<{ x: number; y: number } | null>(null);
  const surfacesBox = useRef<THREE.Box3 | null>(null);

  const [showModel, setShowModel] = useState(false);
  const [showSurfaces, setShowSurfaces] = useState(true);
  const [showOpenings, setShowOpenings] = useState(true);
  const [buildMs, setBuildMs] = useState<number | null>(null);
  const [modelState, setModelState] = useState<ModelState>({ phase: "idle" });

  const colors = useRef<{ cat: Record<SurfaceCategory, THREE.Color>; danger: THREE.Color } | undefined>(undefined);

  const legend = useMemo(() => {
    const c = {} as Record<SurfaceCategory, string>;
    (Object.keys(CATEGORY_VAR) as SurfaceCategory[]).forEach((k) => {
      c[k] = `var(${CATEGORY_VAR[k][0] || "--x"}, ${CATEGORY_VAR[k][1]})`;
    });
    return c;
  }, []);

  // --- Scene init -----------------------------------------------------------
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const components = new OBC.Components();
    componentsRef.current = components;
    const world = components
      .get(OBC.Worlds)
      .create<OBC.SimpleScene, OBC.SimpleCamera, OBC.SimpleRenderer>();
    worldRef.current = world;
    world.scene = new OBC.SimpleScene(components);
    world.renderer = new OBC.SimpleRenderer(components, container);
    world.camera = new OBC.SimpleCamera(components);
    components.init();
    world.scene.setup();
    const scene = world.scene.three;
    scene.background = new THREE.Color(BACKGROUND);
    scene.children.filter((c) => c instanceof THREE.Light).forEach((l) => scene.remove(l));
    scene.add(new THREE.AmbientLight(0xffffff, 0.9));
    const d1 = new THREE.DirectionalLight(0xffffff, 0.6);
    d1.position.set(20, 30, 10);
    scene.add(d1);
    const d2 = new THREE.DirectionalLight(0xffffff, 0.25);
    d2.position.set(-15, 20, -15);
    scene.add(d2);
    scene.add(surfaceGroup.current);
    scene.add(openingGroup.current);

    colors.current = {
      cat: Object.fromEntries(
        (Object.keys(CATEGORY_VAR) as SurfaceCategory[]).map((k) => [
          k,
          cssColor(CATEGORY_VAR[k][0], CATEGORY_VAR[k][1]),
        ]),
      ) as Record<SurfaceCategory, THREE.Color>,
      danger: cssColor("--theme-danger-color", "#dc2626"),
    };

    const sGroup = surfaceGroup.current;
    const oGroup = openingGroup.current;
    return () => {
      sGroup.removeFromParent();
      oGroup.removeFromParent();
      disposeGroup(sGroup);
      disposeGroup(oGroup);
      components.dispose();
      componentsRef.current = null;
      worldRef.current = null;
      ifcModelRef.current = null;
    };
  }, []);

  const camera = useCallback(
    () => worldRef.current?.camera as OBC.SimpleCamera | undefined,
    [],
  );

  const fitBox = useCallback(
    (box: THREE.Box3 | null) => {
      if (!box || box.isEmpty()) return;
      void camera()?.controls.fitToBox(box, true, {
        paddingTop: 0.5,
        paddingBottom: 0.5,
        paddingLeft: 0.5,
        paddingRight: 0.5,
      });
    },
    [camera],
  );

  // --- Vlakken en openingen bouwen -----------------------------------------
  useEffect(() => {
    const cols = colors.current;
    if (!cols) return;
    const t0 = performance.now();
    const sGroup = surfaceGroup.current;
    const oGroup = openingGroup.current;
    disposeGroup(sGroup);
    disposeGroup(oGroup);
    entries.current = [];

    for (const s of model.surfaces) {
      const geom = buildGeometry(s.vertices);
      if (!geom) continue;
      const base = cols.cat[s.category].clone();
      if (s.red) base.lerp(cols.danger, RED_MIX);
      const mat = new THREE.MeshStandardMaterial({
        color: base,
        side: THREE.DoubleSide,
        transparent: true,
        opacity: 1,
      });
      const mesh = new THREE.Mesh(geom, mat);
      mesh.userData.surfaceId = s.id;
      sGroup.add(mesh);
      let outline: THREE.LineLoop | null = null;
      if (s.red) {
        const pts = s.vertices.map((v) => new THREE.Vector3(...toWorld(v)));
        outline = new THREE.LineLoop(
          new THREE.BufferGeometry().setFromPoints(pts),
          new THREE.LineBasicMaterial({ color: cols.danger }),
        );
        sGroup.add(outline);
      }
      entries.current.push({ surface: s, mesh, outline, base });
    }
    const glazing = cssColor("--domain-chart-glazing", "#2563eb");
    for (const o of model.openings) {
      const geom = buildGeometry(o.vertices);
      if (!geom) continue;
      const mat = new THREE.MeshStandardMaterial({
        color: glazing,
        side: THREE.DoubleSide,
        transparent: true,
        opacity: OPENING_OPACITY,
        polygonOffset: true,
        polygonOffsetFactor: -2,
        polygonOffsetUnits: -2,
      });
      const mesh = new THREE.Mesh(geom, mat);
      oGroup.add(mesh);
      const pts = o.vertices.map((v) => new THREE.Vector3(...toWorld(v)));
      oGroup.add(
        new THREE.LineLoop(
          new THREE.BufferGeometry().setFromPoints(pts),
          new THREE.LineBasicMaterial({ color: 0xffffff }),
        ),
      );
    }
    const ms = performance.now() - t0;
    setBuildMs(ms);
    console.info(
      `[IfcImportViewer3D] ${entries.current.length} vlakken, ${model.openings.length} openingen opgebouwd in ${ms.toFixed(0)} ms`,
    );
    const bb = modelBbox(model);
    surfacesBox.current = bb ? worldBox(bb) : null;
    fitBox(surfacesBox.current);
  }, [model, fitBox]);

  // --- Laag-zichtbaarheid ---------------------------------------------------
  useEffect(() => {
    surfaceGroup.current.visible = showSurfaces;
  }, [showSurfaces]);
  useEffect(() => {
    openingGroup.current.visible = showOpenings;
  }, [showOpenings]);
  useEffect(() => {
    if (ifcModelRef.current) ifcModelRef.current.object.visible = showModel;
  }, [showModel, modelState]);

  // --- Selectie-opmaak ------------------------------------------------------
  useEffect(() => {
    for (const e of entries.current) {
      const mat = e.mesh.material as THREE.MeshStandardMaterial;
      const inRoom = !!selectedRoomId && e.surface.roomIds.includes(selectedRoomId);
      const isSel = e.surface.id === selectedSurfaceId;
      mat.color.copy(e.base);
      mat.opacity = 1;
      if (isSel) {
        mat.color.set(SELECT_COLOR);
      } else if (inRoom) {
        mat.color.lerp(new THREE.Color(0xffffff), 0.35);
      } else if (selectedRoomId) {
        mat.opacity = DIMMED_OPACITY;
      }
      mat.depthWrite = mat.opacity === 1;
      if (e.outline) e.outline.visible = !selectedRoomId || inRoom || isSel;
    }
  }, [selectedRoomId, selectedSurfaceId, model]);

  // --- Camera op geselecteerde ruimte ---------------------------------------
  useEffect(() => {
    if (!selectedRoomId) return;
    const b = roomBbox(model, selectedRoomId);
    if (b) fitBox(worldBox(b));
  }, [selectedRoomId, model, fitBox]);

  // --- Origineel model (lazy) ------------------------------------------------
  const modelRequested = useRef(false);
  useEffect(() => {
    if (!showModel || !file || modelRequested.current) return;
    modelRequested.current = true;
    const components = componentsRef.current;
    const world = worldRef.current;
    if (!components || !world) return;
    setModelState({ phase: "loading" });
    const t0 = performance.now();
    void (async () => {
      try {
        const fragments = components.get(OBC.FragmentsManager);
        if (!fragments.initialized) fragments.init(fragmentsWorkerUrl);
        const loader = components.get(OBC.IfcLoader);
        await loader.setup({
          autoSetWasm: false,
          wasm: { path: WASM_PATH, absolute: true },
          // Niet naar de oorsprong verschuiven: de vlakken staan in projectcoordinaten.
          webIfc: { COORDINATE_TO_ORIGIN: false },
        });
        const bytes = new Uint8Array(await file.arrayBuffer());
        const frag = await loader.load(bytes, false, "origineel");
        const cam = world.camera as OBC.SimpleCamera;
        frag.useCamera(cam.three);
        world.scene.three.add(frag.object);
        cam.controls.addEventListener("update", () => void fragments.core.update());
        await fragments.core.update(true);
        frag.onViewUpdated.add(() => void frag.setOpacity(undefined, MODEL_OPACITY));
        await frag.setOpacity(undefined, MODEL_OPACITY);
        ifcModelRef.current = frag;

        // Uitlijningscontrole: middelpunt model t.o.v. middelpunt vlakken.
        let note: string | null = null;
        let offset: THREE.Vector3 | null = null;
        try {
          const boxes = await frag.getBoxes();
          const mb = new THREE.Box3();
          boxes.forEach((b) => mb.union(b));
          mb.applyMatrix4(frag.object.matrixWorld);
          const sb = surfacesBox.current;
          if (!mb.isEmpty() && sb && !sb.isEmpty()) {
            offset = sb.getCenter(new THREE.Vector3()).sub(mb.getCenter(new THREE.Vector3()));
            if (offset.length() > ALIGN_TOLERANCE_M) {
              note = `Het model wijkt ${offset.length().toFixed(1)} m af van de berekende vlakken (middelpunten). Uitlijning niet betrouwbaar.`;
            }
          } else {
            note = "Uitlijning niet gecontroleerd (geen afmetingen beschikbaar).";
          }
        } catch {
          note = "Uitlijning niet gecontroleerd.";
        }
        const ms = performance.now() - t0;
        console.info(`[IfcImportViewer3D] origineel model geladen in ${ms.toFixed(0)} ms`);
        setModelState({ phase: "ready", ms, note, offset });
      } catch (err) {
        modelRequested.current = false;
        setModelState({
          phase: "error",
          message: err instanceof Error ? err.message : "Model laden mislukt",
        });
        setShowModel(false);
      }
    })();
  }, [showModel, file]);

  const applyOffset = () => {
    if (modelState.phase !== "ready" || !modelState.offset || !ifcModelRef.current) return;
    ifcModelRef.current.object.position.add(modelState.offset);
    setModelState({ ...modelState, note: "Model handmatig verschoven naar het middelpunt van de vlakken.", offset: null });
  };

  // --- Picking --------------------------------------------------------------
  const pick = (clientX: number, clientY: number): ViewSurface | null => {
    const container = containerRef.current;
    const cam = worldRef.current?.camera.three;
    if (!container || !cam || !showSurfaces) return null;
    const rect = container.getBoundingClientRect();
    const mouse = new THREE.Vector2(
      ((clientX - rect.left) / rect.width) * 2 - 1,
      -((clientY - rect.top) / rect.height) * 2 + 1,
    );
    raycaster.current.setFromCamera(mouse, cam);
    const meshes = entries.current
      .filter((e) => (e.mesh.material as THREE.Material).opacity > DIMMED_OPACITY || !selectedRoomId)
      .map((e) => e.mesh);
    const hit = raycaster.current.intersectObjects(meshes, false)[0];
    if (!hit) return null;
    return entries.current.find((e) => e.mesh === hit.object)?.surface ?? null;
  };

  const modelAvailable = !!file;
  const redCount = model.surfaces.filter((s) => s.red).length;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-on-surface-secondary">
        <label className="flex items-center gap-1">
          <input type="checkbox" checked={showSurfaces} onChange={(e) => setShowSurfaces(e.target.checked)} />
          Berekende vlakken
        </label>
        <label className="flex items-center gap-1">
          <input type="checkbox" checked={showOpenings} onChange={(e) => setShowOpenings(e.target.checked)} />
          Openingen
        </label>
        <label
          className={`flex items-center gap-1 ${modelAvailable ? "" : "opacity-50"}`}
          title={modelAvailable ? undefined : "Het originele model is niet beschikbaar na herladen: upload het IFC opnieuw."}
        >
          <input
            type="checkbox"
            disabled={!modelAvailable}
            checked={showModel}
            onChange={(e) => setShowModel(e.target.checked)}
          />
          Origineel model (transparant)
        </label>
        {!modelAvailable && (
          <span className="text-on-surface-muted">
            Origineel model niet beschikbaar (herladen): upload het IFC opnieuw om het te tonen.
          </span>
        )}
        {modelState.phase === "loading" && <span>Model laden…</span>}
        {modelState.phase === "error" && (
          <span className="text-red-400">Model laden mislukt: {modelState.message}</span>
        )}
      </div>
      {modelState.phase === "ready" && modelState.note && (
        <p role="status" className="text-xs text-amber-500">
          {modelState.note}{" "}
          {modelState.offset && (
            <button type="button" className="underline" onClick={applyOffset}>
              Verschuif model naar vlakken
            </button>
          )}
        </p>
      )}
      <div
        ref={containerRef}
        data-testid="ifc-viewer-3d"
        className="relative h-[480px] w-full overflow-hidden rounded border border-[var(--oaec-border-subtle)]"
        onPointerDown={(e) => {
          downPos.current = { x: e.clientX, y: e.clientY };
        }}
        onPointerUp={(e) => {
          const d = downPos.current;
          downPos.current = null;
          if (!d || e.button !== 0) return;
          if (Math.hypot(e.clientX - d.x, e.clientY - d.y) > CLICK_MOVE_PX) return;
          onSelectSurface(pick(e.clientX, e.clientY));
        }}
      >
        <div className="pointer-events-none absolute right-2 top-2 z-10 rounded bg-surface-alt/95 p-2 text-[10px] shadow">
          <div className="mb-1 font-semibold">Grenst aan</div>
          {(Object.keys(CATEGORY_LABEL) as SurfaceCategory[]).map((k) => (
            <div key={k} className="flex items-center gap-1.5">
              <span className="h-2.5 w-4 rounded-sm" style={{ backgroundColor: legend[k] }} />
              {CATEGORY_LABEL[k]}
            </div>
          ))}
          <div className="flex items-center gap-1.5">
            <span
              className="h-2.5 w-4 rounded-sm"
              style={{ backgroundColor: "var(--domain-chart-glazing, #2563eb)", opacity: OPENING_OPACITY }}
            />
            Opening
          </div>
          <div className="mt-1 flex items-center gap-1.5">
            <span
              className="h-2.5 w-4 rounded-sm border-2"
              style={{ borderColor: "var(--theme-danger-color, #dc2626)" }}
            />
            Rood: QC-bevinding of geen U ({redCount})
          </div>
        </div>
        <div className="pointer-events-none absolute bottom-1 left-2 z-10 text-[10px] text-on-surface-muted">
          {buildMs != null && `Vlakken opgebouwd in ${buildMs.toFixed(0)} ms`}
          {modelState.phase === "ready" && ` · origineel model ${(modelState.ms / 1000).toFixed(1)} s`}
        </div>
      </div>
    </div>
  );
}
