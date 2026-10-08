/**
 * Filter een thermal-import op goedgekeurde ruimten.
 *
 * Id-mapping: de Rust-mapper behoudt de thermal-room-id als `project.rooms[].id`
 * (zie `map_thermal_import`: `Room { id: thermal_room.id.clone() }`), dus een
 * goedgekeurde thermal-room-id selecteert direct de project-room. De
 * project-elementen (`{room}-c{n}`) hebben GEEN relatie met thermal-
 * construction-id's; die link loopt alleen via de ruimte.
 *
 * Constructies naar een niet-goedgekeurde buurruimte blijven staan. Gevolg in de
 * rekenkern hangt af van het type: `adjacent_room` valt terug op theta_i (dT = 0,
 * dus 0 W/K); `unheated_space` rekent A*U*f_k met f_k default 0,5 zonder
 * room-lookup. Beide worden apart geteld zodat de UI het juiste kan melden.
 */
import type { ThermalImportFile, ThermalImportResult } from "./thermalImport";

export interface FilteredImport {
  importFile: ThermalImportFile;
  importResult: ThermalImportResult;
  /** Vlakken naar een niet-overgenomen verwarmde ruimte (`adjacent_room`): 0 W/K. */
  orphanedHeatedCount: number;
  /** Vlakken naar een niet-overgenomen onverwarmde ruimte: rekenen met f_k (default 0,5). */
  orphanedUnheatedCount: number;
}

/** Soort wees-vlak: grenst aan een ruimte die niet in het project zit. */
export function orphanKind(
  ce: { catalog_ref?: string | null; adjacent_room_id?: string | null; boundary_type?: string },
  projectRoomIds: ReadonlySet<string>,
): "heated" | "unheated" | null {
  if (!ce.catalog_ref || !ce.adjacent_room_id || projectRoomIds.has(ce.adjacent_room_id)) {
    return null;
  }
  if (ce.boundary_type === "unheated_space") return "unheated";
  if (ce.boundary_type === "adjacent_room") return "heated";
  return null;
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
  let orphanedHeatedCount = 0;
  let orphanedUnheatedCount = 0;
  for (const room of rooms) {
    for (const ce of room.constructions) {
      if (ce.catalog_ref) {
        usedCatalogRefs.add(ce.catalog_ref);
        const kind = orphanKind(ce, approvedRoomIds);
        if (kind === "heated") orphanedHeatedCount += 1;
        else if (kind === "unheated") orphanedUnheatedCount += 1;
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

  return {
    importFile: filteredFile,
    importResult: filteredResult,
    orphanedHeatedCount,
    orphanedUnheatedCount,
  };
}
