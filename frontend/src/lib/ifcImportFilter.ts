/**
 * Filter een thermal-import op goedgekeurde ruimten.
 *
 * Id-mapping: de Rust-mapper behoudt de thermal-room-id als `project.rooms[].id`
 * (zie `map_thermal_import`: `Room { id: thermal_room.id.clone() }`), dus een
 * goedgekeurde thermal-room-id selecteert direct de project-room. De
 * project-elementen (`{room}-c{n}`) hebben GEEN relatie met thermal-
 * construction-id's; die link loopt alleen via de ruimte.
 *
 * Constructies naar een niet-goedgekeurde verwarmde buurruimte blijven staan:
 * de rekenkern geeft een verweesde `adjacent_room_id` 0 W/K. Ze worden geteld
 * (`orphanedSurfaceCount`) zodat de UI dat kan melden.
 */
import type { ThermalImportFile, ThermalImportResult } from "./thermalImport";

export interface FilteredImport {
  importFile: ThermalImportFile;
  importResult: ThermalImportResult;
  /** Aantal vlakken (excl. openingen) dat grenst aan een niet-overgenomen ruimte. */
  orphanedSurfaceCount: number;
}

export function filterApprovedRooms(
  importFile: ThermalImportFile,
  importResult: ThermalImportResult,
  approvedRoomIds: ReadonlySet<string>,
): FilteredImport {
  // Thermal-constructies die bij minstens één goedgekeurde ruimte horen
  // (room_a, of de gespiegelde room_b-kant).
  const constructions = importFile.constructions.filter(
    (c) => approvedRoomIds.has(c.room_a) || approvedRoomIds.has(c.room_b),
  );
  const constructionIds = new Set(constructions.map((c) => c.id));
  const openings = (importFile.openings ?? []).filter((o) =>
    constructionIds.has(o.construction_id),
  );
  const openingIds = new Set(openings.map((o) => o.id));

  // Alle ruimten blijven in de lijst: boundary-typering (buiten/grond/buur) zoekt erin op.
  const filteredFile: ThermalImportFile = {
    ...importFile,
    constructions,
    openings,
    open_connections: (importFile.open_connections ?? []).filter(
      (oc) => approvedRoomIds.has(oc.room_a) || approvedRoomIds.has(oc.room_b),
    ),
  };

  const rooms = importResult.project.rooms.filter((r) => approvedRoomIds.has(r.id));
  const usedCatalogRefs = new Set<string>();
  let orphanedSurfaceCount = 0;
  for (const room of rooms) {
    for (const ce of room.constructions) {
      if (ce.catalog_ref) {
        usedCatalogRefs.add(ce.catalog_ref);
        if (ce.adjacent_room_id && !approvedRoomIds.has(ce.adjacent_room_id)) {
          orphanedSurfaceCount += 1;
        }
      }
    }
  }

  const filteredResult: ThermalImportResult = {
    ...importResult,
    project: { ...importResult.project, rooms },
    construction_catalog: importResult.construction_catalog.filter((e) =>
      usedCatalogRefs.has(e.id),
    ),
    room_polygons: importResult.room_polygons.filter((rp) => approvedRoomIds.has(rp.room_id)),
    construction_geometries: (importResult.construction_geometries ?? []).filter((g) =>
      constructionIds.has(g.id),
    ),
    opening_geometries: (importResult.opening_geometries ?? []).filter((g) =>
      openingIds.has(g.id),
    ),
  };

  return { importFile: filteredFile, importResult: filteredResult, orphanedSurfaceCount };
}
