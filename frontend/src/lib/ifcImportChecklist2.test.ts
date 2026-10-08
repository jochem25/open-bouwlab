import { describe, expect, it } from "vitest";

import { ifcAnalyseMock, MOCK_ROOM_HOOG, MOCK_ROOM_LAAG } from "./__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "./__fixtures__/ifcImportTestData";
import { stripAnalyse } from "./ifcAnalyse";
import { buildIfcImportChecklist, findMissingU } from "./ifcImportChecklist";
import type { Project } from "../types";
import type { IfcImportOrigin } from "../types/ifcImport";

function setup() {
  const project = structuredClone(makeImportResult(ifcAnalyseMock.thermal).project) as Project;
  const origin: IfcImportOrigin = {
    ifc_filename: "v.ifc",
    engine_name: "e",
    engine_version: "1",
    imported_at: "2026-10-08T00:00:00Z",
    rooms_count: 2,
    approved_room_ids: [MOCK_ROOM_HOOG, MOCK_ROOM_LAAG],
    analyse: stripAnalyse(ifcAnalyseMock),
  };
  return { project, origin };
}

const find = (items: { id: string }[], id: string) => items.find((i) => i.id === id);

describe("findMissingU", () => {
  it("telt u_value 0 met area > 0; slaat area 0 en open verbindingen over", () => {
    const { project } = setup();
    const all = project.rooms.flatMap((r) => r.constructions);
    const baseline = findMissingU(project.rooms);
    expect(baseline.surfaces).toBe(all.filter((c) => c.catalog_ref).length);
    const room = project.rooms[0]!;
    const proto = room.constructions[0]!;
    room.constructions.push(
      { ...proto, id: "x-c98", u_value: 0, area: 0 },
      { ...proto, id: "x-oc1", u_value: 0, catalog_ref: null },
      { ...proto, id: "x-c97", u_value: 0, catalog_ref: null, area: 2, boundary_type: "adjacent_room" },
    );
    const after = findMissingU(project.rooms);
    expect(after.surfaces).toBe(baseline.surfaces);
    expect(after.openings).toBe(1); // adjacent_room telt ook: ook daar rekent de kern met U
    expect(after.area).toBeCloseTo(baseline.area + 2);
  });
});

describe("checklist: vlakken naar niet-overgenomen ruimten", () => {
  it("toont het aantal zolang de buurruimte niet in het project staat; verdwijnt als ze er is", () => {
    const { project, origin } = setup();
    const hoog = project.rooms.find((r) => r.id === MOCK_ROOM_HOOG)!;
    const toLaag = hoog.constructions.filter((c) => c.adjacent_room_id === MOCK_ROOM_LAAG).length;
    expect(toLaag).toBeGreaterThan(0);
    const full = buildIfcImportChecklist(project, origin);
    expect(find(full, "wees-vlakken")).toBeUndefined();

    const partial = { ...project, rooms: project.rooms.filter((r) => r.id === MOCK_ROOM_HOOG) };
    const item = find(
      buildIfcImportChecklist(partial, { ...origin, approved_room_ids: [MOCK_ROOM_HOOG] }),
      "wees-vlakken",
    ) as { aantal: number; severity: string; tekst: string };
    expect(item.aantal).toBe(toLaag);
    expect(item.severity).toBe("default");
    expect(item.tekst).toContain("niet-overgenomen verwarmde ruimten en tellen als 0 W/K");
  });
});

describe("checklist: wees-vlakken naar onverwarmde ruimten", () => {
  it("telt apart en meldt f_k = 0,5 i.p.v. 0 W/K", () => {
    const { project, origin } = setup();
    const hoog = project.rooms.find((r) => r.id === MOCK_ROOM_HOOG)!;
    const toLaag = hoog.constructions.filter(
      (c) => c.adjacent_room_id === MOCK_ROOM_LAAG && c.catalog_ref,
    );
    expect(toLaag.length).toBeGreaterThan(0);
    for (const c of toLaag) c.boundary_type = "unheated_space";
    const partial = { ...project, rooms: project.rooms.filter((r) => r.id === MOCK_ROOM_HOOG) };
    const items = buildIfcImportChecklist(partial, origin);
    expect(find(items, "wees-vlakken")).toBeUndefined();
    const item = find(items, "wees-vlakken-onverwarmd") as { aantal: number; tekst: string };
    expect(item.aantal).toBe(toLaag.length);
    expect(item.tekst).toContain("niet-overgenomen onverwarmde ruimten en rekenen met f_k = 0,5");
  });
});

describe("checklist: functie uit model / naam / niet herkend", () => {
  it("splitst in drie items met aantallen", () => {
    const { project, origin } = setup();
    const [hoog, laag] = project.rooms as [Project["rooms"][number], Project["rooms"][number]];
    // hoog: model "toiletruimte"; laag: naam "6. Berging"... voeg een derde ruimte toe zonder match
    hoog.name = "A";
    hoog.function = "toilet";
    laag.name = "6. Berging";
    laag.function = "storage";
    const third = structuredClone(laag);
    third.id = "room-derde";
    third.name = "12. Ruimte X";
    third.function = "living_room";
    project.rooms.push(third);

    const thermal = {
      ...origin.analyse!.thermal,
      rooms: [
        ...origin.analyse!.thermal.rooms.map((r) =>
          r.id === MOCK_ROOM_HOOG ? { ...r, function: "toiletruimte", function_source: "pset" } : r,
        ),
        { id: "room-derde", name: "12. Ruimte X", type: "heated" as const },
      ],
    };
    const withThird = {
      ...origin,
      approved_room_ids: [...origin.approved_room_ids, "room-derde"],
      analyse: { ...origin.analyse!, thermal },
    };
    const items = buildIfcImportChecklist(project, withThird);
    expect(find(items, "functie-model")).toMatchObject({ aantal: 1, severity: "default" });
    expect(find(items, "functie-geschat")).toMatchObject({ aantal: 1 });
    const onbekend = find(items, "functie-onbekend") as { aantal: number; tekst: string };
    expect(onbekend.aantal).toBe(1);
    expect(onbekend.tekst).toContain("default");

    // oplossen: functie kiezen laat het item verdwijnen
    third.function = "bedroom";
    expect(find(buildIfcImportChecklist(project, withThird), "functie-onbekend")).toBeUndefined();
  });
});
