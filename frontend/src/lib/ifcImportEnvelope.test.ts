import { beforeEach, describe, expect, it } from "vitest";

import { ifcAnalyseMock, MOCK_ROOM_HOOG } from "./__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "./__fixtures__/ifcImportTestData";
import { buildIfcEnergyDocument, emptyModellerSnapshot, serializeIfcEnergy } from "./ifcenergy";
import { importProject, openProjectFile, type ImportResult } from "./importExport";
import { partializeProjectStore, useProjectStore } from "../store/projectStore";
import type { IfcImportOrigin } from "../types/ifcImport";

const project = makeImportResult(ifcAnalyseMock.thermal).project;

// JSON kent geen -0 (de fixture bevat er); vergelijk tegen de JSON-gerondreisde vorm.
const viaJson = <T,>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

const origin: IfcImportOrigin = {
  ifc_filename: "voorbeeld.ifc",
  engine_name: "ifc-ruimtebalans",
  engine_version: "0.0.0-mock",
  imported_at: "2026-10-08T10:00:00.000Z",
  rooms_count: 1,
  approved_room_ids: [MOCK_ROOM_HOOG],
  analyse: ifcAnalyseMock,
};

describe("ifcImport in de opslag-envelope", () => {
  beforeEach(() => {
    useProjectStore.getState().setProject(project);
  });

  it(".ifcenergy: round-trip build -> parse behoudt herkomst, goedgekeurde ids en analyse", () => {
    const json = serializeIfcEnergy(
      buildIfcEnergyDocument({
        project,
        result: null,
        modeller: emptyModellerSnapshot(),
        ifcImport: origin,
      }),
    );
    const imported = openProjectFile(json) as ImportResult;
    expect(imported.ifcImport).toEqual(viaJson(origin));

    useProjectStore.getState().setProject(imported.project, { ifcImport: imported.ifcImport });
    expect(useProjectStore.getState().ifcImport?.analyse?.thermal.rooms).toHaveLength(3);
  });

  it(".ifcenergy zonder veld -> undefined", () => {
    const json = serializeIfcEnergy(
      buildIfcEnergyDocument({ project, result: null, modeller: emptyModellerSnapshot() }),
    );
    expect(json).not.toContain("ifcImport");
    expect((openProjectFile(json) as ImportResult).ifcImport).toBeUndefined();
  });

  it(".isso51.json-envelope: met veld -> herkomst; zonder veld of ongeldig -> undefined", () => {
    const base = {
      version: "1.0.0",
      schema: "isso51-project-v1",
      exported_at: "2026-10-08T00:00:00.000Z",
      project,
      result: null,
    };
    expect(importProject(JSON.stringify({ ...base, ifcImport: origin })).ifcImport).toEqual(
      viaJson(origin),
    );
    expect(importProject(JSON.stringify(base)).ifcImport).toBeUndefined();
    expect(importProject(JSON.stringify({ ...base, ifcImport: { x: 1 } })).ifcImport).toBeUndefined();
  });

  it("store: setProject zonder opts wist de herkomst (projectwissel)", () => {
    useProjectStore.getState().setIfcImport(origin);
    useProjectStore.getState().setProject(project);
    expect(useProjectStore.getState().ifcImport).toBeNull();
  });

  it("localStorage-persist bevat de herkomst maar niet de analyse", () => {
    useProjectStore.getState().setIfcImport(origin);
    const slim = partializeProjectStore(useProjectStore.getState());
    expect(slim.ifcImport?.ifc_filename).toBe("voorbeeld.ifc");
    expect(slim.ifcImport?.approved_room_ids).toEqual([MOCK_ROOM_HOOG]);
    expect(slim.ifcImport).not.toHaveProperty("analyse");
  });
});
