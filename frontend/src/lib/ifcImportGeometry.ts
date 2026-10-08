/**
 * Pure helpers voor de 3D-viewer op het tabblad IFC-import (geen React, geen DOM).
 *
 * Invoer zijn de thermal-vlakken uit de engine-response: meters, Z-up,
 * vlakke 3D-polygonen in projectcoordinaten. De viewer zet ze om naar de
 * three.js-wereld met dezelfde conventie als FloorCanvas3D: world = (x, z, -y).
 */
import { ShapeUtils, Vector2 } from "three";

import type { QcFinding } from "./ifcAnalyse";
import type {
  CatalogEntry,
  ThermalConstruction,
  ThermalImportFile,
  ThermalOpening,
  ThermalRoom,
} from "./thermalImport";
import { isPseudoRoom } from "./thermalImport";

export type Vec3 = [number, number, number];

/** Kleurcategorie van een vlak: waar grenst het aan. */
export type SurfaceCategory = "exterior" | "ground" | "unheated" | "adjacent" | "water";

export const CATEGORY_LABEL: Record<SurfaceCategory, string> = {
  exterior: "Buiten",
  ground: "Grond",
  unheated: "Onverwarmd",
  adjacent: "Verwarmde buur",
  water: "Water",
};

export interface TriangulatedPolygon {
  /** Platte xyz-lijst, in de coordinaten van de invoer. */
  positions: number[];
  /** Driehoeksindices in `positions` (per drie). */
  indices: number[];
}

/** Newell-normaal (niet genormeerd lengte 1; nulvector bij een gedegenereerd vlak). */
export function newellNormal(v: Vec3[]): Vec3 {
  let nx = 0;
  let ny = 0;
  let nz = 0;
  for (let i = 0; i < v.length; i++) {
    const a = v[i]!;
    const b = v[(i + 1) % v.length]!;
    nx += (a[1] - b[1]) * (a[2] + b[2]);
    ny += (a[2] - b[2]) * (a[0] + b[0]);
    nz += (a[0] - b[0]) * (a[1] + b[1]);
  }
  const len = Math.hypot(nx, ny, nz);
  return len < 1e-12 ? [0, 0, 0] : [nx / len, ny / len, nz / len];
}

/** Z-up projectcoordinaat (m) -> three.js Y-up wereld: (x, z, -y). */
export function toWorld(p: Vec3): Vec3 {
  return [p[0], p[2], -p[1]];
}

/**
 * Trianguleer een vlakke 3D-polygoon: projecteer op het vlak (Newell-normaal),
 * ShapeUtils.triangulateShape, terug naar 3D (indices verwijzen naar de
 * oorspronkelijke vertices, een dubbel sluitpunt wordt genegeerd).
 */
export function triangulatePolygon(vertices: Vec3[]): TriangulatedPolygon {
  let pts = vertices;
  const first = pts[0];
  const last = pts[pts.length - 1];
  if (
    pts.length > 3 &&
    first &&
    last &&
    Math.hypot(first[0] - last[0], first[1] - last[1], first[2] - last[2]) < 1e-9
  ) {
    pts = pts.slice(0, -1);
  }
  if (pts.length < 3) return { positions: [], indices: [] };
  const n = newellNormal(pts);
  if (n[0] === 0 && n[1] === 0 && n[2] === 0) return { positions: [], indices: [] };

  // Orthonormale basis (u, w) in het vlak.
  const ref: Vec3 = Math.abs(n[0]) < 0.9 ? [1, 0, 0] : [0, 1, 0];
  let u: Vec3 = [
    ref[1] * n[2] - ref[2] * n[1],
    ref[2] * n[0] - ref[0] * n[2],
    ref[0] * n[1] - ref[1] * n[0],
  ];
  const ul = Math.hypot(u[0], u[1], u[2]);
  u = [u[0] / ul, u[1] / ul, u[2] / ul];
  const w: Vec3 = [
    n[1] * u[2] - n[2] * u[1],
    n[2] * u[0] - n[0] * u[2],
    n[0] * u[1] - n[1] * u[0],
  ];
  const contour = pts.map(
    (p) =>
      new Vector2(
        p[0] * u[0] + p[1] * u[1] + p[2] * u[2],
        p[0] * w[0] + p[1] * w[1] + p[2] * w[2],
      ),
  );
  const faces = ShapeUtils.triangulateShape(contour, []);
  const positions: number[] = [];
  for (const p of pts) positions.push(p[0], p[1], p[2]);
  const indices: number[] = [];
  for (const f of faces) indices.push(f[0]!, f[1]!, f[2]!);
  return { positions, indices };
}

/** Oppervlak (m2) van een vlakke polygoon via de Newell-som. */
export function polygonArea(vertices: Vec3[]): number {
  let sx = 0;
  let sy = 0;
  let sz = 0;
  for (let i = 0; i < vertices.length; i++) {
    const a = vertices[i]!;
    const b = vertices[(i + 1) % vertices.length]!;
    sx += a[1] * b[2] - a[2] * b[1];
    sy += a[2] * b[0] - a[0] * b[2];
    sz += a[0] * b[1] - a[1] * b[0];
  }
  return Math.hypot(sx, sy, sz) / 2;
}

/**
 * Categorie van een vlak via de buurruimte: normaal `room_b` vanuit `room_a`;
 * is `room_a` een pseudo-ruimte (buiten/grond/water) en `room_b` echt, dan
 * bepaalt `room_a` de categorie.
 */
export function surfaceCategory(
  rooms: Map<string, ThermalRoom>,
  c: Pick<ThermalConstruction, "room_a" | "room_b">,
): SurfaceCategory {
  const a = rooms.get(c.room_a);
  const b = rooms.get(c.room_b);
  const other = a && isPseudoRoom(a) && b && !isPseudoRoom(b) ? a : b;
  switch (other?.type) {
    case "outside":
      return "exterior";
    case "ground":
      return "ground";
    case "water":
      return "water";
    case "unheated":
      return "unheated";
    case "heated":
      return "adjacent";
    default:
      return "exterior";
  }
}

/**
 * Het vlak kan geen U krijgen: constructie zonder lagen, of de catalogus bevat
 * een entry zonder lagen voor dit constructietype (match op revit_type_name).
 */
export function hasNoU(
  c: Pick<ThermalConstruction, "layers" | "revit_type_name">,
  layerlessTypeNames: ReadonlySet<string> = new Set(),
): boolean {
  if ((c.layers ?? []).length === 0) return true;
  return !!c.revit_type_name && layerlessTypeNames.has(c.revit_type_name);
}

/** Typenamen van catalogus-entries zonder lagen. */
export function layerlessTypeNames(catalog: CatalogEntry[]): Set<string> {
  const out = new Set<string>();
  for (const e of catalog) {
    if (e.layers.length === 0 && e.revit_type_name) out.add(e.revit_type_name);
  }
  return out;
}

/** Rood-criterium: QC-bevinding naar dit vlak, of geen U mogelijk. */
export function redReasons(
  c: ThermalConstruction,
  findings: QcFinding[],
  layerless: ReadonlySet<string> = new Set(),
): { finding: boolean; noU: boolean } {
  return {
    finding: findings.some((f) => f.construction_id === c.id),
    noU: hasNoU(c, layerless),
  };
}

export interface ViewSurface {
  /** construction.id van het getekende vlak. */
  id: string;
  /** Alle construction-ids die dit vlak vertegenwoordigen (samengevoegd). */
  constructionIds: string[];
  /** Echte ruimten waar het vlak bij hoort (room_a en/of room_b). */
  roomIds: string[];
  category: SurfaceCategory;
  vertices: Vec3[];
  areaM2: number;
  red: boolean;
  redFinding: boolean;
  redNoU: boolean;
  openingIds: string[];
}

export interface ViewOpening {
  id: string;
  constructionId: string;
  roomIds: string[];
  type: ThermalOpening["type"];
  vertices: Vec3[];
}

export interface ViewModel {
  surfaces: ViewSurface[];
  openings: ViewOpening[];
}

/**
 * Bouw de teken-modellen. Elk vlak een keer: bij een kamerpaar (pair_id)
 * wordt alleen de kant van de eerst geziene ruimte getekend; de vlakken van de
 * andere kant verwijzen via `roomIds` wel naar beide ruimten.
 */
export function buildViewModel(
  file: ThermalImportFile,
  findings: QcFinding[],
  catalog: CatalogEntry[] = [],
): ViewModel {
  const rooms = new Map(file.rooms.map((r) => [r.id, r]));
  const isReal = (id: string) => {
    const r = rooms.get(id);
    return !!r && !isPseudoRoom(r);
  };
  const layerless = layerlessTypeNames(catalog);
  const pairSide = new Map<string, string>();
  const surfaces: ViewSurface[] = [];
  const byConstruction = new Map<string, ViewSurface>();

  for (const c of file.constructions) {
    if (!c.vertices || c.vertices.length < 3) continue;
    if (c.pair_id) {
      const side = pairSide.get(c.pair_id);
      if (side === undefined) pairSide.set(c.pair_id, c.room_a);
      else if (side !== c.room_a) continue;
    }
    const red = redReasons(c, findings, layerless);
    const s: ViewSurface = {
      id: c.id,
      constructionIds: [c.id],
      roomIds: [c.room_a, c.room_b].filter(isReal),
      category: surfaceCategory(rooms, c),
      vertices: c.vertices,
      areaM2: c.gross_area_m2,
      red: red.finding || red.noU,
      redFinding: red.finding,
      redNoU: red.noU,
      openingIds: [],
    };
    surfaces.push(s);
    byConstruction.set(c.id, s);
  }

  // Vlakken van de niet-getekende pair-kant: bevindingen/geen-U tellen mee op de ruimte-kant.
  for (const c of file.constructions) {
    if (!c.pair_id || byConstruction.has(c.id)) continue;
    const drawn = pairSide.get(c.pair_id);
    if (drawn === c.room_a) continue;
    const red = redReasons(c, findings, layerless);
    if (!(red.finding || red.noU)) continue;
    // Zoek het getekende vlak dat deze ruimte raakt; markeer dat.
    const target = surfaces.find((s) => s.roomIds.includes(c.room_a) && s.roomIds.includes(c.room_b));
    if (target) {
      target.red = true;
      target.redFinding ||= red.finding;
      target.redNoU ||= red.noU;
      target.constructionIds.push(c.id);
    }
  }

  const openings: ViewOpening[] = [];
  for (const o of file.openings ?? []) {
    if (!o.vertices || o.vertices.length < 3) continue;
    const parent = byConstruction.get(o.construction_id);
    if (parent) parent.openingIds.push(o.id);
    const src = file.constructions.find((c) => c.id === o.construction_id);
    openings.push({
      id: o.id,
      constructionId: o.construction_id,
      roomIds: parent?.roomIds ?? [src?.room_a, src?.room_b].filter((x): x is string => !!x && isReal(x)),
      type: o.type,
      vertices: o.vertices,
    });
  }
  return { surfaces, openings };
}

export interface Bbox {
  min: Vec3;
  max: Vec3;
}

/** Bbox (projectcoordinaten, Z-up) van de vlakken die bij een ruimte horen; null zonder vlakken. */
export function roomBbox(model: ViewModel, roomId: string): Bbox | null {
  let min: Vec3 | null = null;
  let max: Vec3 | null = null;
  for (const s of model.surfaces) {
    if (!s.roomIds.includes(roomId)) continue;
    for (const v of s.vertices) {
      if (!min || !max) {
        min = [...v];
        max = [...v];
        continue;
      }
      for (let i = 0; i < 3; i++) {
        if (v[i]! < min[i]!) min[i] = v[i]!;
        if (v[i]! > max[i]!) max[i] = v[i]!;
      }
    }
  }
  return min && max ? { min, max } : null;
}

/** Bbox van alle vlakken. */
export function modelBbox(model: ViewModel): Bbox | null {
  let min: Vec3 | null = null;
  let max: Vec3 | null = null;
  for (const s of model.surfaces) {
    for (const v of s.vertices) {
      if (!min || !max) {
        min = [...v];
        max = [...v];
        continue;
      }
      for (let i = 0; i < 3; i++) {
        if (v[i]! < min[i]!) min[i] = v[i]!;
        if (v[i]! > max[i]!) max[i] = v[i]!;
      }
    }
  }
  return min && max ? { min, max } : null;
}
