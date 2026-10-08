/**
 * Gedeelde overname-logica voor een thermal-import (v1.x): gebruikt door de
 * ThermalImportWizard (Revit/JSON) en de IFC-import-tab.
 *
 * De functie is "pure-ish": de enige neveneffecten zijn de aanroepen van
 * `deps.ensureProjectConstruction` (schrijft in de modellerStore). Het
 * resultaat wordt teruggegeven; de aanroeper zet project/boundaries/geometrie
 * in de stores en navigeert.
 */
import type { Project } from "../types";
import type {
  ImportedBoundary,
  ImportGeometry,
  ProjectConstruction,
} from "../components/modeller/types";

import { calculateRc } from "./rcCalculation";
import type {
  ThermalImportFile,
  ThermalImportResult,
  ThermalOpening,
  ThermalRoom,
} from "./thermalImport";
import {
  applyEditsToProject,
  importCatalogToProjectConstructions,
  toImportedBoundaries,
} from "./thermalImport";

type EnsureProjectConstruction = Parameters<
  typeof importCatalogToProjectConstructions
>[1];

export interface ApplyThermalImportDeps {
  ensureProjectConstruction: EnsureProjectConstruction;
  /** Actuele projectconstructies (na `ensureProjectConstruction`-aanroepen). */
  getProjectConstructions: () => ProjectConstruction[];
}

export interface AppliedThermalImport {
  project: Project;
  boundaries: ImportedBoundary[];
  importGeometry: ImportGeometry;
}

export function applyThermalImport(
  importFile: ThermalImportFile,
  importResult: ThermalImportResult,
  editedRooms: ThermalRoom[],
  editedOpenings: ThermalOpening[],
  catalogUValues: Map<string, number>,
  deps: ApplyThermalImportDeps,
): AppliedThermalImport {
  // 1. Convert catalog entries to ProjectConstructions in modellerStore.
  //    Returns a map from CatalogEntry.id → ProjectConstruction.id. We do
  //    this BEFORE applyEditsToProject so we can compute Rc/U per
  //    construction and feed the auto-calculated U-values into the merge.
  const refMap = importCatalogToProjectConstructions(
    importResult.construction_catalog,
    deps.ensureProjectConstruction,
  );

  // 2. Auto-compute U-values via calculateRc for every imported catalog
  //    entry, using the ProjectConstruction just created. User edits from
  //    the LayerEditor (catalogUValues) win over the auto-values, so the
  //    workflow stays: user opens LayerEditor → handmatige waarde; anders
  //    pakt de backend de Rc-berekening uit de geïmporteerde lambdas.
  const autoUValues = new Map<string, number>();
  const projectConstructions = deps.getProjectConstructions();
  for (const entry of importResult.construction_catalog) {
    const pcId = refMap.get(entry.id);
    if (!pcId) continue;
    const pc = projectConstructions.find((c) => c.id === pcId);
    if (!pc || pc.layers.length === 0) continue;
    const rc = calculateRc(
      pc.layers.map((l) => ({
        materialId: l.materialId,
        thickness: l.thickness,
        stud: l.stud,
        lambdaOverride: l.lambdaOverride,
      })),
      pc.verticalPosition,
    );
    if (rc.uValue > 0) {
      autoUValues.set(entry.id, Math.round(rc.uValue * 1000) / 1000);
    }
  }

  // Merge: start with auto values, override with user edits.
  const mergedUValues = new Map(autoUValues);
  catalogUValues.forEach((v, k) => mergedUValues.set(k, v));

  // 3. Merge user edits (room types, U-values, LayerEditor results) into the backend-mapped project
  let mergedProject = applyEditsToProject(
    importResult.project,
    editedRooms,
    editedOpenings,
    mergedUValues,
  );

  // 4. Stamp project_construction_id on every ConstructionElement that has
  //    a catalog_ref. Openings (catalog_ref == null) are left untouched.
  mergedProject = {
    ...mergedProject,
    rooms: mergedProject.rooms.map((room) => ({
      ...room,
      constructions: room.constructions.map((ce) => {
        if (!ce.catalog_ref) return ce;
        const projectConstructionId = refMap.get(ce.catalog_ref);
        if (!projectConstructionId) return ce;
        return { ...ce, project_construction_id: projectConstructionId };
      }),
    })),
  };

  // 5. Boundaries for the 3D viewer. Use editedRooms so boundary conditions
  //    reflect user's type changes.
  const boundaries = toImportedBoundaries(importFile.constructions, editedRooms);

  // 5b. Store the real v1.1 geometry (true room polygons, true-north,
  //     per-surface vertices) so the 3D viewer renders the actual boundary
  //     instead of the derived rectangle. All coordinates stay in meters, as
  //     returned by the backend.
  //
  //     Derive a real floor elevation (floorZ) per room so the viewer stacks
  //     floors in Z at their true height. Chain:
  //       constructions[room_a === room.id && orientation === "floor"].id
  //       → construction_geometries[id].vertices → min-Z (raw Revit Z, m).
  //     A room can have several floor surfaces (e.g. split levels); we take
  //     the lowest. Rooms without a floor construction get no floorZ and the
  //     viewer falls back to level-name grouping.
  const geomById = new Map<string, [number, number, number][]>();
  for (const g of importResult.construction_geometries ?? []) {
    geomById.set(g.id, g.vertices);
  }
  const floorZByRoom = new Map<string, number>();
  for (const c of importFile.constructions) {
    if (c.orientation !== "floor") continue;
    const verts = geomById.get(c.id);
    if (!verts || verts.length === 0) continue;
    let minZ = Infinity;
    for (const v of verts) {
      const z = v[2];
      if (typeof z === "number" && z < minZ) minZ = z;
    }
    if (!Number.isFinite(minZ)) continue;
    const prev = floorZByRoom.get(c.room_a);
    if (prev === undefined || minZ < prev) floorZByRoom.set(c.room_a, minZ);
  }

  const importGeometry: ImportGeometry = {
    roomPolygons: importResult.room_polygons.map((rp) => ({
      roomId: rp.room_id,
      polygon: rp.polygon,
      name: rp.name,
      level: rp.level,
      heightM: rp.height_m,
      floorZ: floorZByRoom.get(rp.room_id),
    })),
    trueNorthDeg: importResult.true_north_deg,
    constructionGeometries: (importResult.construction_geometries ?? []).map((g) => ({
      id: g.id,
      vertices: g.vertices,
    })),
    openingGeometries: (importResult.opening_geometries ?? []).map((g) => ({
      id: g.id,
      vertices: g.vertices,
    })),
  };

  return { project: mergedProject, boundaries, importGeometry };
}
