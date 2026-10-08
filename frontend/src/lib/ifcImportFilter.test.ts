import { describe, expect, it } from "vitest";

import { ifcAnalyseMock, MOCK_ROOM_HOOG, MOCK_ROOM_LAAG } from "./__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "./__fixtures__/ifcImportTestData";
import { filterApprovedRooms } from "./ifcImportFilter";

const file = ifcAnalyseMock.thermal;

describe("filterApprovedRooms", () => {
  it("alles goedgekeurd: niets valt weg, geen wezen", () => {
    const result = makeImportResult(file);
    const out = filterApprovedRooms(file, result, new Set([MOCK_ROOM_HOOG, MOCK_ROOM_LAAG]));
    expect(out.importResult.project.rooms).toHaveLength(2);
    expect(out.importFile.constructions).toHaveLength(file.constructions.length);
    expect(out.importResult.construction_catalog.map((e) => e.id)).toEqual(["cat-1", "cat-2"]);
    expect(out.orphanedSurfaceCount).toBe(0);
  });

  it("één ruimte: project-rooms, polygonen en geometrie worden gefilterd", () => {
    const result = makeImportResult(file);
    const out = filterApprovedRooms(file, result, new Set([MOCK_ROOM_HOOG]));
    expect(out.importResult.project.rooms.map((r) => r.id)).toEqual([MOCK_ROOM_HOOG]);
    expect(out.importResult.room_polygons.map((p) => p.room_id)).toEqual([MOCK_ROOM_HOOG]);
    // hoog: 9 eigen constructies + de laag->hoog paarkant (room_b = hoog) blijft ook
    const ids = new Set(out.importFile.constructions.map((c) => c.id));
    expect(ids.has("c-58d027b674de520b")).toBe(true);
    expect(ids.has("c-8ec084da8b41071e")).toBe(false); // vloer van laag
    expect(out.importResult.construction_geometries?.map((g) => g.id).sort()).toEqual(
      [...ids].sort(),
    );
    // openingen: deur o-...E hoort bij laag-wand (c-58d0...), die blijft; beide deuren blijven
    expect(out.importFile.openings?.map((o) => o.construction_id).sort()).toEqual(
      ["c-58d027b674de520b", "c-a310d26865d76134"].sort(),
    );
    expect(out.importResult.opening_geometries).toHaveLength(2);
    // ruimtenlijst blijft volledig voor boundary-typering
    expect(out.importFile.rooms).toHaveLength(file.rooms.length);
  });

  it("telt vlakken naar niet-overgenomen buurruimten als wees", () => {
    const result = makeImportResult(file);
    const hoogRoom = result.project.rooms.find((r) => r.id === MOCK_ROOM_HOOG)!;
    const toLaag = hoogRoom.constructions.filter((c) => c.adjacent_room_id === MOCK_ROOM_LAAG);
    expect(toLaag.length).toBeGreaterThan(0);
    const out = filterApprovedRooms(file, result, new Set([MOCK_ROOM_HOOG]));
    expect(out.orphanedSurfaceCount).toBe(toLaag.length);
    // de elementen blijven staan
    const kept = out.importResult.project.rooms[0]!.constructions;
    expect(kept).toHaveLength(hoogRoom.constructions.length);
  });

  it("catalogus houdt alleen entries die nog gebruikt worden", () => {
    const result = makeImportResult(file);
    // Maak laag de enige gebruiker van cat-2 door hoog's cat-2 te herlabelen.
    const hoog = result.project.rooms.find((r) => r.id === MOCK_ROOM_HOOG)!;
    for (const c of hoog.constructions) if (c.catalog_ref === "cat-2") c.catalog_ref = "cat-1";
    const out = filterApprovedRooms(file, result, new Set([MOCK_ROOM_HOOG]));
    expect(out.importResult.construction_catalog.map((e) => e.id)).toEqual(["cat-1"]);
  });

  it("niets goedgekeurd: lege uitkomst", () => {
    const out = filterApprovedRooms(file, makeImportResult(file), new Set());
    expect(out.importResult.project.rooms).toHaveLength(0);
    expect(out.importFile.constructions).toHaveLength(0);
    expect(out.importResult.construction_catalog).toHaveLength(0);
  });
});
