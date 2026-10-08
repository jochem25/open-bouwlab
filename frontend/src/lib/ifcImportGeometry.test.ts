import { describe, expect, it } from "vitest";

import fixture from "./__fixtures__/analyse_200_v12_fixture.json";
import {
  buildViewModel,
  hasNoU,
  modelBbox,
  newellNormal,
  polygonArea,
  roomBbox,
  surfaceCategory,
  triangulatePolygon,
  type Vec3,
} from "./ifcImportGeometry";
import type { QcFinding } from "./ifcAnalyse";
import type { ThermalImportFile, ThermalRoom } from "./thermalImport";

const thermal = fixture.thermal as unknown as ThermalImportFile;
const findings = fixture.qc.findings as unknown as QcFinding[];

function triArea(positions: number[], indices: number[]): number {
  let sum = 0;
  for (let i = 0; i < indices.length; i += 3) {
    const p = (k: number): Vec3 => [
      positions[indices[i + k]! * 3]!,
      positions[indices[i + k]! * 3 + 1]!,
      positions[indices[i + k]! * 3 + 2]!,
    ];
    const [a, b, c] = [p(0), p(1), p(2)];
    const ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    const ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    sum +=
      Math.hypot(
        ab[1]! * ac[2]! - ab[2]! * ac[1]!,
        ab[2]! * ac[0]! - ab[0]! * ac[2]!,
        ab[0]! * ac[1]! - ab[1]! * ac[0]!,
      ) / 2;
  }
  return sum;
}

describe("triangulatePolygon", () => {
  it("vierkant -> 2 driehoeken", () => {
    const t = triangulatePolygon([[0, 0, 0], [2, 0, 0], [2, 2, 0], [0, 2, 0]]);
    expect(t.indices.length / 3).toBe(2);
    expect(triArea(t.positions, t.indices)).toBeCloseTo(4);
  });
  it("L-vorm -> 4 driehoeken met juist oppervlak", () => {
    const l: Vec3[] = [[0, 0, 0], [2, 0, 0], [2, 1, 0], [1, 1, 0], [1, 2, 0], [0, 2, 0]];
    const t = triangulatePolygon(l);
    expect(t.indices.length / 3).toBe(4);
    expect(triArea(t.positions, t.indices)).toBeCloseTo(3);
  });
  it("verticale wand (x = const)", () => {
    const wall: Vec3[] = [[4, 0, 0], [4, 0, 1.3], [4, 3, 1.3], [4, 3, 0]];
    const t = triangulatePolygon(wall);
    expect(t.indices.length / 3).toBe(2);
    expect(triArea(t.positions, t.indices)).toBeCloseTo(3.9);
    expect(Math.abs(newellNormal(wall)[0]!)).toBeCloseTo(1);
  });
  it("sluitpunt dubbel en gedegenereerd", () => {
    const t = triangulatePolygon([[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0], [0, 0, 0]]);
    expect(t.indices.length / 3).toBe(2);
    expect(triangulatePolygon([[0, 0, 0], [1, 0, 0], [2, 0, 0]]).indices).toEqual([]);
  });
  it("polygonArea", () => {
    expect(polygonArea([[4, 0, 0], [4, 0, 1.3], [4, 3, 1.3], [4, 3, 0]])).toBeCloseTo(3.9);
  });
});

describe("surfaceCategory", () => {
  const mk = (id: string, type: ThermalRoom["type"]): ThermalRoom => ({ id, name: id, type });
  const rooms = new Map(
    [
      mk("h", "heated"),
      mk("h2", "heated"),
      mk("o", "outside"),
      mk("g", "ground"),
      mk("w", "water"),
      mk("u", "unheated"),
    ].map((r) => [r.id, r]),
  );
  it("per buurtype", () => {
    expect(surfaceCategory(rooms, { room_a: "h", room_b: "o" })).toBe("exterior");
    expect(surfaceCategory(rooms, { room_a: "h", room_b: "g" })).toBe("ground");
    expect(surfaceCategory(rooms, { room_a: "h", room_b: "w" })).toBe("water");
    expect(surfaceCategory(rooms, { room_a: "h", room_b: "u" })).toBe("unheated");
    expect(surfaceCategory(rooms, { room_a: "h", room_b: "h2" })).toBe("adjacent");
  });
  it("pseudo-ruimte als room_a", () => {
    expect(surfaceCategory(rooms, { room_a: "g", room_b: "h" })).toBe("ground");
  });
});

describe("rood-criterium", () => {
  it("constructie zonder lagen of catalogus-entry zonder lagen", () => {
    expect(hasNoU({ layers: [] })).toBe(true);
    expect(hasNoU({ layers: [{ material: "x", thickness_mm: 1 }] })).toBe(false);
    expect(
      hasNoU(
        { layers: [{ material: "x", thickness_mm: 1 }], revit_type_name: "T" },
        new Set(["T"]),
      ),
    ).toBe(true);
  });
  it("bevinding met construction_id markeert dat vlak", () => {
    const target = thermal.constructions[4]!.id;
    const f: QcFinding = {
      severity: "warning",
      code: "X",
      message: "m",
      count: 1,
      modeller_action: false,
      construction_id: target,
    };
    const m = buildViewModel(thermal, [f]);
    expect(m.surfaces.filter((s) => s.red).map((s) => s.id)).toEqual([target]);
    expect(m.surfaces.find((s) => s.id === target)?.redFinding).toBe(true);
  });
  it("fixture: geen rood zonder bevinding (alle vlakken hebben lagen)", () => {
    const m = buildViewModel(
      thermal,
      findings.filter((f) => !f.construction_id),
    );
    expect(m.surfaces.some((s) => s.red)).toBe(false);
  });
});

describe("buildViewModel / bbox (fixture)", () => {
  const m = buildViewModel(thermal, findings);
  it("pair_id: een kant getekend", () => {
    // 4 pair-constructies (3 aan kant A, 1 aan kant B): alleen kant A wordt getekend.
    expect(m.surfaces.length).toBe(thermal.constructions.length - 1);
    expect(m.openings.length).toBe(2);
  });
  it("bbox per ruimte", () => {
    const hoog = thermal.rooms[0]!.id;
    const laag = thermal.rooms[1]!.id;
    const a = roomBbox(m, hoog)!;
    expect(a.min).toEqual([0, 0, 0]);
    expect(a.max[0]).toBeCloseTo(4);
    expect(a.max[2]).toBeCloseTo(2.6);
    const b = roomBbox(m, laag)!;
    expect(b.min[0]).toBeCloseTo(4);
    expect(b.max[0]).toBeCloseTo(8.2);
    expect(roomBbox(m, "bestaat-niet")).toBeNull();
    expect(modelBbox(m)!.max[0]).toBeCloseTo(8.2);
  });
});
