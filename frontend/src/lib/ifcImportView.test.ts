import { describe, expect, it } from "vitest";

import { ifcAnalyseMock, MOCK_ROOM_HOOG, MOCK_ROOM_LAAG } from "./__fixtures__/ifcAnalyseMock";
import {
  defaultApproved,
  envelopeArea,
  mirrorOrientation,
  realRooms,
  roomSurfaces,
  roomVolume,
} from "./ifcImportView";

const file = ifcAnalyseMock.thermal;

describe("ifcImportView", () => {
  it("telt alleen echte ruimten", () => {
    expect(realRooms(file)).toHaveLength(2);
  });
  it("volume uit gegeven of oppervlak x hoogte", () => {
    expect(roomVolume({ id: "a", name: "a", type: "heated", area_m2: 10, height_m: 2.5 })).toBe(25);
    expect(roomVolume({ id: "a", name: "a", type: "heated" })).toBeUndefined();
  });
  it("schilsom telt alleen vlakken naar buiten/grond/water/onverwarmd", () => {
    // 0.78+12+7.8+10.4+10.4+12 (hoog) + 12+9.6+9.6+7.2+12 (laag)
    expect(envelopeArea(file)).toBeCloseTo(103.78, 2);
  });
  it("spiegelt floor/ceiling/roof zoals thermal.rs", () => {
    expect(mirrorOrientation("floor")).toBe("ceiling");
    expect(mirrorOrientation("roof")).toBe("floor");
    expect(mirrorOrientation("ceiling")).toBe("floor");
    expect(mirrorOrientation("wall")).toBe("wall");
  });
  it("paarkanten (pair_id, v1.2) worden niet vanaf room_b getoond/gespiegeld", () => {
    const surfaces = roomSurfaces(file, MOCK_ROOM_LAAG);
    // laag: 1 paar-wand (eigen kant) + 5 buitenvlakken; de 3 hoog->laag paarkanten niet
    expect(surfaces).toHaveLength(6);
    expect(surfaces.filter((s) => s.construction.pair_id)).toHaveLength(1);
  });
  it("zonder pair_id (v1.1) wordt de room_b-kant gespiegeld getoond", () => {
    const f = {
      ...file,
      version: "1.1",
      rooms: file.rooms,
      constructions: [
        { ...file.constructions[0]!, pair_id: undefined, orientation: "floor" as const },
      ],
    };
    const s = roomSurfaces(f, MOCK_ROOM_LAAG);
    expect(s).toHaveLength(1);
    expect(s[0]!.orientation).toBe("ceiling");
    expect(s[0]!.otherRoomId).toBe(MOCK_ROOM_HOOG);
  });
  it("standaard goedgekeurd: alles behalve ruimten met blokkerende bevinding", () => {
    const set = defaultApproved(file, ifcAnalyseMock.qc.findings);
    expect([...set]).toEqual([MOCK_ROOM_HOOG]);
  });
});
