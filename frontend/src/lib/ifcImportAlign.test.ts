import { describe, expect, it } from "vitest";

import { Matrix4, Vector3 } from "three";

import fixture from "./__fixtures__/analyse_200_v12_fixture.json";
import type { GeometryOrigin, IfcAnalyseResponse } from "./ifcAnalyse";
import { toWorld } from "./ifcImportGeometry";
import {
  alignByBboxMin,
  sceneGroupMatrix,
  alignmentFromResponse,
  isoCameraFit,
  residualAfterAlignment,
  worldBoxToProject,
} from "./ifcImportAlign";
import type { Vec3 } from "./ifcImportGeometry";

const origin: GeometryOrigin = {
  frame: "ifc_world_m",
  offset_m: [483686, 116610, 0],
  matrix_world_from_thermal: [
    [1, 0, 0, 483686],
    [0, 1, 0, 116610],
    [0, 0, 1, 0],
    [0, 0, 0, 1],
  ],
};

describe("uitlijning en camera", () => {
  const surf = { min: [1, 2, -0.2] as Vec3, max: [12, 9, 2.5] as Vec3 };
  const model = { min: [150000, 450000, 0] as Vec3, max: [150013, 450008, 3] as Vec3 };

  it("alignmentFromResponse: undefined zonder geldig geometry_origin", () => {
    expect(alignmentFromResponse({})).toBeUndefined();
    expect(alignmentFromResponse({ geometry_origin: null })).toBeUndefined();
    const bad = { ...origin, matrix_world_from_thermal: [[1, 0, 0]] };
    expect(alignmentFromResponse({ geometry_origin: bad })).toBeUndefined();
  });
  it("alignmentFromResponse: fixture bevat geometry_origin", () => {
    const a = alignmentFromResponse(fixture as unknown as IfcAnalyseResponse);
    expect(a?.kind).toBe("exact");
  });
  it("thermal -> wereld -> scene met de gemeten R-1-coordinatiematrix", () => {
    // Gemeten 08-10 in de browser (R-1): @thatopen-coordinatiematrix is Y-up en
    // IFC-wereld -> lokaal, translatie (-483688.2254, 0.23, 116610.06323).
    const a = alignmentFromResponse({ geometry_origin: origin })!;
    const coord = new Matrix4().makeTranslation(-483688.2254, 0.23, 116610.06323);
    const gm = sceneGroupMatrix(a.matrixWorldFromThermal, coord);
    // Thermal (1, 2, 3) -> wereld (483687, 116612, 3) -> Y-up (483687, 3, -116612)
    // -> lokaal (-1.2254, 3.23, -1.93677).
    const out = new Vector3(...toWorld([1, 2, 3])).applyMatrix4(gm);
    expect(out.x).toBeCloseTo(-1.2254, 4);
    expect(out.y).toBeCloseTo(3.23, 4);
    expect(out.z).toBeCloseTo(-1.93677, 4);
  });
  it("zonder coordinatiematrix: alleen M, daarna scene", () => {
    const a = alignmentFromResponse({ geometry_origin: origin })!;
    const out = new Vector3(...toWorld([0, 0, 0])).applyMatrix4(
      sceneGroupMatrix(a.matrixWorldFromThermal, null),
    );
    expect([out.x, out.y, out.z]).toEqual([483686, 0, -116610]);
  });
  it("rotatie in M (90 graden om z) wordt meegenomen", () => {
    const rot = [[0, -1, 0, 0], [1, 0, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]];
    const out = new Vector3(...toWorld([1, 0, 0])).applyMatrix4(sceneGroupMatrix(rot, null));
    // thermal (1,0,0) -> wereld (0,1,0) -> scene (0,0,-1)
    expect(out.x).toBeCloseTo(0);
    expect(out.y).toBeCloseTo(0);
    expect(out.z).toBeCloseTo(-1);
  });
  it("bbox-minimumhoek valt samen na uitlijning", () => {
    const a = alignByBboxMin(model, surf);
    expect(a.kind).toBe("bbox");
    expect(model.min[0] + a.offset[0]).toBeCloseTo(1);
    expect(model.min[1] + a.offset[1]).toBeCloseTo(2);
    expect(model.min[2] + a.offset[2]).toBeCloseTo(-0.2);
    expect(residualAfterAlignment(model, surf, a.offset)).toBeLessThan(1.5);
  });
  it("wereld -> project (x, z, -y)", () => {
    const b = worldBoxToProject([1, -0.2, -9], [12, 2.5, -2]);
    expect(b.min).toEqual([1, 2, -0.2]);
    expect(b.max).toEqual([12, 9, 2.5]);
  });
  it("isometrische camera vanuit (1,1,1) met marge", () => {
    const f = isoCameraFit([0, 0, 0], [10, 3, 10]);
    expect(f.target).toEqual([5, 1.5, 5]);
    const d: Vec3 = [f.position[0] - 5, f.position[1] - 1.5, f.position[2] - 5];
    expect(d[0]).toBeCloseTo(d[1]);
    expect(d[1]).toBeCloseTo(d[2]);
    expect(d[0]).toBeGreaterThan(0);
    expect(Math.hypot(...d)).toBeGreaterThan((Math.hypot(10, 3, 10) / 2) * 1.3);
  });
});
