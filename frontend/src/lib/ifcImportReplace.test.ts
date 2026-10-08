import { beforeEach, describe, expect, it } from "vitest";

import { ifcAnalyseMock } from "./__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "./__fixtures__/ifcImportTestData";
import { useProjectStore } from "../store/projectStore";
import { dismissKey } from "../components/ifcImport/IfcImportChecklistPanel";

const imported = makeImportResult(ifcAnalyseMock.thermal).project;

describe("replaceRoomsFromImport", () => {
  beforeEach(() => {
    useProjectStore.getState().reset();
  });

  it("vervangt alleen de ruimten; info, klimaat en serverkoppeling blijven", () => {
    const base = useProjectStore.getState().project;
    const oldRoom = { ...imported.rooms[0]!, id: "oud-1" };
    useProjectStore.setState({
      project: {
        ...base,
        info: { ...base.info, name: "Mijn project" },
        climate: { ...base.climate, theta_e: -7 },
        rooms: [oldRoom],
      },
      activeProjectId: "srv-1",
      currentLocalPath: "C:/x.ifcenergy",
      ventilation: {
        terminals: [{ id: "t", roomId: "oud-1", type: "supply", source: "manual", flowDm3s: 1 }],
        rooms: {},
        system: "D",
      },
      isDirty: false,
    });
    const before = useProjectStore.getState().project;

    useProjectStore.getState().replaceRoomsFromImport(imported);

    const s = useProjectStore.getState();
    expect(s.project.info).toEqual(before.info);
    expect(s.project.climate).toEqual(before.climate);
    // Instellingen blijven; alleen het gebruiksoppervlak (geometrie) komt uit de import.
    expect(s.project.building).toEqual({
      ...before.building,
      total_floor_area: imported.building.total_floor_area,
    });
    expect(s.project.building.total_floor_area).not.toBe(before.building.total_floor_area);
    expect(s.project.rooms.map((r) => r.id)).toEqual(imported.rooms.map((r) => r.id));
    expect(s.activeProjectId).toBe("srv-1");
    expect(s.currentLocalPath).toBe("C:/x.ifcenergy");
    expect(s.ventilation.system).toBe("D");
    expect(s.ventilation.terminals).toHaveLength(0);
    expect(s.isDirty).toBe(true);
  });
});

describe("checklist-dismiss sleutel", () => {
  it("is per import (imported_at)", () => {
    expect(dismissKey("a")).not.toBe(dismissKey("b"));
    expect(dismissKey("a")).toContain("a");
  });
});
