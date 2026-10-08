import { describe, expect, it } from "vitest";

import { ifcAnalyseMock, MOCK_ROOM_HOOG } from "./__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "./__fixtures__/ifcImportTestData";
import {
  applyRoomFunctions,
  resolveRoomFunction,
  roomFunctionFromModel,
} from "./roomFunctionFromModel";

describe("roomFunctionFromModel", () => {
  it.each([
    ["toiletruimte", "toilet"],
    ["Badruimte", "bathroom"],
    ["verkeersruimte", "hallway"],
    ["Bergruimte", "storage"],
    ["afval", "storage"],
    ["meterruimte", "storage"],
    ["Technische ruimte", "storage"],
    ["keuken", "kitchen"],
    ["kookruimte", "kitchen"],
    ["Slaapvertrek", "bedroom"],
    ["slaapruimte", "bedroom"],
    ["verblijfsruimte", "living_room"],
    ["Woonkamer", "living_room"],
    ["zolder", "attic"],
    ["  TOILETRUIMTE ", "toilet"],
  ] as const)("%s -> %s", (term, fn) => {
    expect(roomFunctionFromModel(term)).toBe(fn);
  });

  it("gebruiksfuncties en onbekende termen geven geen ruimtefunctie", () => {
    for (const t of ["overige gebruiksfunctie", "bijeenkomstfunctie", "bijeenkomst", "Kantoorfunctie", "iets onbekends", "", null, undefined]) {
      expect(roomFunctionFromModel(t)).toBeNull();
    }
  });
});

describe("resolveRoomFunction (model -> naam -> default)", () => {
  it("model wint van de naam", () => {
    expect(resolveRoomFunction("Gang", "toiletruimte")).toEqual({ function: "toilet", source: "model" });
  });
  it("gebruiksfunctie valt door naar de naam", () => {
    expect(resolveRoomFunction("13. Toilet", "overige gebruiksfunctie")).toEqual({
      function: "toilet",
      source: "naam",
    });
  });
  it("geen model, geen naam-match -> woonkamer als default", () => {
    expect(resolveRoomFunction("12. Ruimte X", null)).toEqual({
      function: "living_room",
      source: "default",
    });
  });
});

describe("applyRoomFunctions", () => {
  it("gebruikt thermal function, rooms_extra als terugval, daarna de naam", () => {
    const file = {
      ...ifcAnalyseMock.thermal,
      rooms: ifcAnalyseMock.thermal.rooms.map((r) =>
        r.id === MOCK_ROOM_HOOG
          ? { ...r, name: "9. Gang", function: "badruimte", function_source: "pset" }
          : r,
      ),
    };
    const result = makeImportResult(file);
    const project = applyRoomFunctions(result.project, file.rooms);
    expect(project.rooms.find((r) => r.id === MOCK_ROOM_HOOG)?.function).toBe("bathroom");

    // terugval op rooms_extra
    const plain = makeImportResult(ifcAnalyseMock.thermal);
    const extra = ifcAnalyseMock.rooms_extra.map((e) =>
      e.room_id === MOCK_ROOM_HOOG ? { ...e, function: "keuken" } : e,
    );
    const viaExtra = applyRoomFunctions(plain.project, ifcAnalyseMock.thermal.rooms, extra);
    expect(viaExtra.rooms.find((r) => r.id === MOCK_ROOM_HOOG)?.function).toBe("kitchen");
  });
});
