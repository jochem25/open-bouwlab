import { beforeEach, describe, expect, it } from "vitest";

import { ifcAnalyseMock, MOCK_ROOM_HOOG, MOCK_ROOM_LAAG } from "./__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "./__fixtures__/ifcImportTestData";
import { stripAnalyse } from "./ifcAnalyse";
import { buildIfcImportChecklist, computeMaterialMatches } from "./ifcImportChecklist";
import { computeFunctionSources } from "./roomFunctionFromModel";
import {
  mergePersistedProjectStore,
  partializeProjectStore,
  useProjectStore,
} from "../store/projectStore";
import type { Project } from "../types";
import type { IfcImportOrigin } from "../types/ifcImport";

const APPROVED = [MOCK_ROOM_HOOG, MOCK_ROOM_LAAG];

function setup() {
  const project = structuredClone(makeImportResult(ifcAnalyseMock.thermal).project) as Project;
  const [hoog, laag] = project.rooms as [Project["rooms"][number], Project["rooms"][number]];
  hoog.name = "13. Toilet";
  hoog.function = "toilet";
  laag.name = "12. Ruimte X";
  laag.function = "living_room";
  const thermal = {
    ...ifcAnalyseMock.thermal,
    rooms: ifcAnalyseMock.thermal.rooms.map((r) =>
      r.id === MOCK_ROOM_HOOG ? { ...r, function: "toiletruimte" } : r,
    ),
  };
  const analyse = { ...stripAnalyse(ifcAnalyseMock), thermal };
  const origin: IfcImportOrigin = {
    ifc_filename: "v.ifc",
    engine_name: "e",
    engine_version: "1",
    imported_at: "2026-10-08T00:00:00Z",
    rooms_count: 2,
    approved_room_ids: APPROVED,
    room_function_sources: computeFunctionSources(project, thermal.rooms, undefined, APPROVED),
    material_matches: computeMaterialMatches(thermal, APPROVED),
    analyse,
  };
  return { project, origin };
}

const ids = (items: { id: string }[]) => items.map((i) => i.id);

describe("ifcImport: bewaarde bronnen en materiaalkoppelingen", () => {
  beforeEach(() => {
    useProjectStore.getState().setProject(setup().project);
  });

  it("computeFunctionSources / computeMaterialMatches leveren kleine records", () => {
    const { origin } = setup();
    expect(origin.room_function_sources).toEqual({
      [MOCK_ROOM_HOOG]: "model",
      [MOCK_ROOM_LAAG]: "default",
    });
    expect(origin.material_matches?.some((m) => m.ifc_name === "n7_isolatie_PIR")).toBe(true);
  });

  it("persist-roundtrip: analyse valt weg, checklist blijft hetzelfde", () => {
    const { project, origin } = setup();
    useProjectStore.getState().setProject(project);
    useProjectStore.getState().setIfcImport(origin);
    const before = buildIfcImportChecklist(project, origin);

    const slim = JSON.parse(JSON.stringify(partializeProjectStore(useProjectStore.getState())));
    expect(slim.ifcImport).not.toHaveProperty("analyse");
    expect(slim.ifcImport.room_function_sources).toEqual(origin.room_function_sources);
    expect(slim.ifcImport.material_matches).toEqual(origin.material_matches);

    const rehydrated = mergePersistedProjectStore(slim, useProjectStore.getState());
    const after = buildIfcImportChecklist(rehydrated.project, rehydrated.ifcImport);
    expect(rehydrated.ifcImport?.analyse).toBeUndefined();

    for (const id of ["functie-model", "functie-onbekend", "materiaal-gekoppeld"]) {
      expect(ids(before)).toContain(id);
      expect(ids(after)).toContain(id);
      expect(after.find((i) => i.id === id)?.aantal).toBe(before.find((i) => i.id === id)?.aantal);
    }
  });

  it("zonder bewaarde velden (oud project) valt de checklist terug op analyse; zonder analyse geen model-item", () => {
    const { project, origin } = setup();
    const old: IfcImportOrigin = {
      ...origin,
      room_function_sources: undefined,
      material_matches: undefined,
    };
    const items = buildIfcImportChecklist(project, old);
    expect(ids(items)).toContain("functie-model");
    expect(ids(items)).toContain("materiaal-gekoppeld");

    const noAnalyse = buildIfcImportChecklist(project, { ...old, analyse: undefined });
    expect(ids(noAnalyse)).not.toContain("functie-model");
    expect(ids(noAnalyse)).not.toContain("materiaal-gekoppeld");
  });

  it("oplossen: functie kiezen laat 'niet herkend' verdwijnen, naam-bron volgt de naam-schatting", () => {
    const { project, origin } = setup();
    const laag = project.rooms.find((r) => r.id === MOCK_ROOM_LAAG)!;
    laag.function = "bedroom";
    expect(ids(buildIfcImportChecklist(project, origin))).not.toContain("functie-onbekend");

    const naamOrigin = { ...origin, room_function_sources: { [MOCK_ROOM_LAAG]: "naam" as const } };
    laag.name = "6. Berging";
    laag.function = "storage";
    expect(ids(buildIfcImportChecklist(project, naamOrigin))).toContain("functie-geschat");
    laag.function = "bedroom";
    expect(ids(buildIfcImportChecklist(project, naamOrigin))).not.toContain("functie-geschat");
  });
});
