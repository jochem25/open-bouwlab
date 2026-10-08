/**
 * Pure helpers voor uitlijning van het originele IFC-model op de berekende
 * vlakken, en de camera-fit van de IFC-import-viewer (geen React, geen DOM).
 */
import { Matrix4 } from "three";

import type { GeometryOrigin } from "./ifcAnalyse";
import type { Bbox, Vec3 } from "./ifcImportGeometry";

type Tuple16 = [
  number, number, number, number, number, number, number, number,
  number, number, number, number, number, number, number, number,
];

/** Verschuiving (projectcoordinaten, Z-up, m) die op het model moet worden toegepast. */
export interface Alignment {
  offset: Vec3;
  /** "exact": uit de engine-response; "bbox": automatisch op omhullende. */
  kind: "exact" | "bbox";
}

/** Exacte uitlijning uit `geometry_origin` van de engine. */
export interface ExactAlignment {
  kind: "exact";
  /** IFC-wereld (m) = M @ [x, y, z, 1] met thermal Z-up meters; rij-major 4x4. */
  matrixWorldFromThermal: number[][];
}

/**
 * Expliciete transformatie uit de engine-response (`geometry_origin`), of
 * undefined als die ontbreekt of ongeldig is (dan geldt de bbox-terugval).
 */
export function alignmentFromResponse(response: {
  geometry_origin?: GeometryOrigin | null;
}): ExactAlignment | undefined {
  const m = response?.geometry_origin?.matrix_world_from_thermal;
  if (!m || m.length !== 4 || m.some((r) => !Array.isArray(r) || r.length !== 4)) {
    return undefined;
  }
  if (m.some((r) => r.some((v) => !Number.isFinite(v)))) return undefined;
  return { kind: "exact", matrixWorldFromThermal: m };
}

/** Z-up -> Y-up wereld: (x, y, z) -> (x, z, -y). */
const SCENE_FROM_ZUP = new Matrix4().set(1, 0, 0, 0, 0, 0, 1, 0, 0, -1, 0, 0, 0, 0, 0, 1);

/**
 * Matrix voor de groep met vlakken. De vlakken zijn al per punt omgezet met
 * toWorld (thermal -> scene zonder verschuiving); deze matrix zet ze daarna naar
 * waar het model staat:
 *   scene = C . W . M . W^-1 (toWorld(p))
 * met M = matrix_world_from_thermal (Z-up), W = Z-up -> Y-up en C = de
 * coordinatiematrix van het geladen model (null = identiteit). Gemeten 08-10
 * op R-1: @thatopen levert C al in het Y-up sceneframe en als IFC-wereld ->
 * lokaal (translatie -483688,2 / 0,23 / +116610,1), dus C zelf, niet C^-1.
 */
export function sceneGroupMatrix(
  matrixWorldFromThermal: number[][],
  coordination: Matrix4 | null,
): Matrix4 {
  const m = new Matrix4().set(...(matrixWorldFromThermal.flat() as Tuple16));
  const c = coordination ? coordination.clone() : new Matrix4();
  const wInv = SCENE_FROM_ZUP.clone().invert();
  return c.multiply(SCENE_FROM_ZUP).multiply(m).multiply(wInv);
}

/** Three.js-wereldbox (Y-up: x, z, -y) terug naar projectcoordinaten (Z-up). */
export function worldBoxToProject(min: Vec3, max: Vec3): Bbox {
  return { min: [min[0], -max[2], min[1]], max: [max[0], -min[2], max[1]] };
}

export function bboxCenter(b: Bbox): Vec3 {
  return [
    (b.min[0] + b.max[0]) / 2,
    (b.min[1] + b.max[1]) / 2,
    (b.min[2] + b.max[2]) / 2,
  ];
}

/**
 * Automatische uitlijning: de minimumhoek van het model (x, y in plan; z op de
 * laagste vloer) valt samen met die van de vlakken. Geen rotatie.
 */
export function alignByBboxMin(modelBox: Bbox, surfacesBox: Bbox): Alignment {
  return {
    kind: "bbox",
    offset: [
      surfacesBox.min[0] - modelBox.min[0],
      surfacesBox.min[1] - modelBox.min[1],
      surfacesBox.min[2] - modelBox.min[2],
    ],
  };
}

/** Afstand (m) tussen de bbox-middelpunten na toepassing van de uitlijning. */
export function residualAfterAlignment(
  modelBox: Bbox,
  surfacesBox: Bbox,
  offset: Vec3,
): number {
  const m = bboxCenter(modelBox);
  const s = bboxCenter(surfacesBox);
  return Math.hypot(
    m[0] + offset[0] - s[0],
    m[1] + offset[1] - s[1],
    m[2] + offset[2] - s[2],
  );
}

export const ISO_DIRECTION: Vec3 = [1, 1, 1];
export const FIT_MARGIN = 0.15;
const DEFAULT_FOV_DEG = 60;
const MIN_RADIUS_M = 0.5;

/**
 * Isometrische camerapositie (three.js-wereld, Y-up) voor een wereldbox: kijkt
 * vanuit de (1,1,1)-richting naar het middelpunt, met `margin` rondom.
 */
export function isoCameraFit(
  min: Vec3,
  max: Vec3,
  fovDeg = DEFAULT_FOV_DEG,
  margin = FIT_MARGIN,
): { position: Vec3; target: Vec3 } {
  const target: Vec3 = [
    (min[0] + max[0]) / 2,
    (min[1] + max[1]) / 2,
    (min[2] + max[2]) / 2,
  ];
  const radius = Math.max(
    Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]) / 2,
    MIN_RADIUS_M,
  );
  const dist = (radius * (1 + 2 * margin)) / Math.sin((fovDeg * Math.PI) / 360);
  const len = Math.hypot(...ISO_DIRECTION);
  return {
    target,
    position: [
      target[0] + (ISO_DIRECTION[0] / len) * dist,
      target[1] + (ISO_DIRECTION[1] / len) * dist,
      target[2] + (ISO_DIRECTION[2] / len) * dist,
    ],
  };
}
