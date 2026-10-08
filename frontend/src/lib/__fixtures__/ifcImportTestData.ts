/** Testhulp: bouwt een gemockt `/import/thermal`-resultaat uit een thermal-bestand. */
import type { Project, Room } from "../../types";
import type {
  CatalogEntry,
  ThermalImportFile,
  ThermalImportResult,
} from "../thermalImport";

export function makeImportResult(file: ThermalImportFile): ThermalImportResult {
  const byId = new Map(file.rooms.map((r) => [r.id, r]));
  const real = file.rooms.filter((r) => r.type === "heated" || r.type === "unheated");
  const realIds = new Set(real.map((r) => r.id));

  const rooms = real.map((r) => {
    let n = 0;
    const constructions = file.constructions
      .filter((c) => c.room_a === r.id)
      .map((c) => {
        n += 1;
        const other = byId.get(c.room_b);
        const adjacent = other && realIds.has(other.id) ? other.id : null;
        return {
          id: `${r.id}-c${n}`,
          description: "",
          area: c.gross_area_m2,
          u_value: 0,
          boundary_type: adjacent ? "adjacent_room" : "exterior",
          material_type: "masonry",
          adjacent_room_id: adjacent,
          vertical_position: c.orientation === "wall" ? "wall" : "floor",
          use_forfaitaire_thermal_bridge: false,
          has_embedded_heating: false,
          catalog_ref: c.orientation === "wall" ? "cat-1" : "cat-2",
        };
      });
    return {
      id: r.id,
      name: r.name,
      function: "living_room",
      floor_area: r.area_m2 ?? 0,
      height: r.height_m,
      constructions,
      heating_system: "radiator_lt",
    } as unknown as Room;
  });

  const project = {
    info: { name: "Test" },
    building: { building_type: "terraced", qv10: 100, total_floor_area: 24, security_class: "b" },
    climate: { theta_e: -10 },
    ventilation: { system_type: "system_c" },
    rooms,
  } as unknown as Project;

  const catalog: CatalogEntry[] = [
    {
      id: "cat-1",
      description: "Wand test",
      layers: [{ material: "baksteen", thickness_mm: 100, lambda: 0.9 }],
      used_for: [["exterior", "wall"]],
      total_area_m2: 10,
      surface_count: 5,
    },
    {
      id: "cat-2",
      description: "Vloer test",
      layers: [{ material: "beton", thickness_mm: 200, lambda: 1.7 }],
      used_for: [["exterior", "floor"]],
      total_area_m2: 12,
      surface_count: 2,
    },
    {
      id: "cat-3",
      description: "Ongebruikt",
      layers: [],
      used_for: [],
      total_area_m2: 0,
      surface_count: 0,
    },
  ];

  return {
    project,
    warnings: [],
    construction_catalog: catalog,
    room_polygons: real.map((r) => ({
      room_id: r.id,
      polygon: r.boundary_polygon ?? [],
      name: r.name,
      level: r.level,
      height_m: r.height_m,
    })),
    true_north_deg: 12,
    construction_geometries: file.constructions.map((c, i) => ({
      id: c.id,
      vertices: [
        [0, 0, c.orientation === "floor" ? i : 3],
        [1, 0, c.orientation === "floor" ? i : 3],
        [1, 1, c.orientation === "floor" ? i : 3],
      ],
    })),
    opening_geometries: (file.openings ?? []).map((o) => ({
      id: o.id,
      vertices: [
        [0, 0, 0],
        [1, 0, 0],
        [1, 0, 2],
      ],
    })),
  };
}
