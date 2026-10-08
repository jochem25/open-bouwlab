import { describe, expect, it } from "vitest";

import { ifcAnalyseMock } from "./__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "./__fixtures__/ifcImportTestData";
import { applyThermalImport } from "./applyThermalImport";
import {
  applyRoomFunctionsFromNames,
  matchRoomFunction,
  roomFunctionFromName,
} from "./roomFunctionFromName";

describe("roomFunctionFromName", () => {
  it.each([
    ["13. Toilet", "toilet", true],
    ["9. Gang", "hallway", true],
    ["2. Gang/keuken", "kitchen", true], // eerste match wint: keuken voor gang
    ["6. Berging", "storage", true],
    ["17. Leefruimte", "living_room", true],
    ["14. Keuken", "kitchen", true],
    ["3. Slaapkamer 1", "bedroom", true],
    ["5. Badkamer", "bathroom", true],
    ["Overloop", "landing", true],
    ["Zolder", "attic", true],
    ["Bijkeuken", "kitchen", true],
    ["Garage", "storage", true],
    ["Kantoor", "living_room", true],
  ] as const)("%s -> %s", (name, fn, recognized) => {
    expect(roomFunctionFromName(name)).toEqual({ function: fn, recognized });
  });

  it("geen match -> living_room, niet herkend", () => {
    expect(roomFunctionFromName("12. Ruimte X")).toEqual({
      function: "living_room",
      recognized: false,
    });
  });

  it("modeller-mapping is ongewijzigd (Nederlandse sleutels, custom bij geen match)", () => {
    expect(matchRoomFunction("Woonkamer")).toBe("woonkamer");
    expect(matchRoomFunction("Gang")).toBe("hal");
    expect(matchRoomFunction("Overloop")).toBe("hal");
    expect(matchRoomFunction("Bijkeuken")).toBe("keuken");
    expect(matchRoomFunction("Iets anders")).toBe("custom");
  });
});

describe("applyRoomFunctionsFromNames", () => {
  const file = {
    ...ifcAnalyseMock.thermal,
    rooms: ifcAnalyseMock.thermal.rooms.map((r, i) =>
      i === 0 ? { ...r, name: "13. Toilet" } : i === 1 ? { ...r, name: "6. Berging", type: "unheated" as const } : r,
    ),
  };

  it("zet functie uit naam voor verwarmde ruimten, laat onverwarmde met rust", () => {
    const result = makeImportResult(file);
    const project = applyRoomFunctionsFromNames(result.project, file.rooms);
    expect(project.rooms.find((r) => r.name === "13. Toilet")?.function).toBe("toilet");
    expect(project.rooms.find((r) => r.name === "6. Berging")?.function).toBe("living_room");
  });

  it("de wizard-overname (applyThermalImport) past namen NIET toe", () => {
    const result = makeImportResult(file);
    const deps = {
      ensureProjectConstruction: () => "proj-x",
      getProjectConstructions: () => [],
    };
    const applied = applyThermalImport(file, result, file.rooms, [], new Map(), deps);
    expect(applied.project.rooms.find((r) => r.name === "13. Toilet")?.function).toBe("living_room");
  });
});
