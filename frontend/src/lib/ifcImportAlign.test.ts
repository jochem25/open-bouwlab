import { describe, expect, it } from "vitest";

import {
  alignByBboxMin,
  alignmentFromResponse,
  isoCameraFit,
  residualAfterAlignment,
  worldBoxToProject,
} from "./ifcImportAlign";
import type { Vec3 } from "./ifcImportGeometry";

describe("uitlijning en camera", () => {
  const surf = { min: [1, 2, -0.2] as Vec3, max: [12, 9, 2.5] as Vec3 };
  const model = { min: [150000, 450000, 0] as Vec3, max: [150013, 450008, 3] as Vec3 };

  it("alignmentFromResponse levert nu undefined", () => {
    expect(alignmentFromResponse({})).toBeUndefined();
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
