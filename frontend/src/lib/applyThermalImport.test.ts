import { describe, expect, it } from "vitest";

import type { ProjectConstruction } from "../components/modeller/types";
import { ifcAnalyseMock } from "./__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "./__fixtures__/ifcImportTestData";
import { applyThermalImport } from "./applyThermalImport";
import { calculateRc } from "./rcCalculation";
import {
  applyEditsToProject,
  importCatalogToProjectConstructions,
  toImportedBoundaries,
  type ThermalImportFile,
  type ThermalImportResult,
} from "./thermalImport";

const file = ifcAnalyseMock.thermal;

function makeDeps() {
  const created: ProjectConstruction[] = [];
  return {
    created,
    deps: {
      ensureProjectConstruction: (data: Omit<ProjectConstruction, "id">) => {
        const id = `proj-${created.length + 1}`;
        created.push({ ...data, id } as unknown as ProjectConstruction);
        return id;
      },
      getProjectConstructions: () => created,
    },
  };
}

/** Letterlijke kopie van de oorspronkelijke ThermalImportWizard.handleFinalImport (referentie). */
function legacyHandleFinalImport(
  importFile: ThermalImportFile,
  importResult: ThermalImportResult,
  catalogUValues: Map<string, number>,
) {
  const { deps, created } = makeDeps();
  const refMap = importCatalogToProjectConstructions(
    importResult.construction_catalog,
    deps.ensureProjectConstruction,
  );
  const autoUValues = new Map<string, number>();
  for (const entry of importResult.construction_catalog) {
    const pcId = refMap.get(entry.id);
    if (!pcId) continue;
    const pc = created.find((c) => c.id === pcId);
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
    if (rc.uValue > 0) autoUValues.set(entry.id, Math.round(rc.uValue * 1000) / 1000);
  }
  const merged = new Map(autoUValues);
  catalogUValues.forEach((v, k) => merged.set(k, v));
  let project = applyEditsToProject(
    importResult.project,
    importFile.rooms,
    importFile.openings ?? [],
    merged,
  );
  project = {
    ...project,
    rooms: project.rooms.map((room) => ({
      ...room,
      constructions: room.constructions.map((ce) => {
        if (!ce.catalog_ref) return ce;
        const id = refMap.get(ce.catalog_ref);
        return id ? { ...ce, project_construction_id: id } : ce;
      }),
    })),
  };
  return { project, boundaries: toImportedBoundaries(importFile.constructions, importFile.rooms) };
}

describe("applyThermalImport", () => {
  const result = makeImportResult(file);
  const userU = new Map([["cat-1", 0.25]]);

  it("geeft hetzelfde project en dezelfde boundaries als de oorspronkelijke wizard-logica", () => {
    const { deps } = makeDeps();
    const applied = applyThermalImport(file, result, file.rooms, file.openings ?? [], userU, deps);
    const legacy = legacyHandleFinalImport(file, result, userU);
    expect(applied.project).toEqual(legacy.project);
    expect(applied.boundaries).toEqual(legacy.boundaries);
  });

  it("zet gebruikers-U-waarde, project_construction_id en geometrie", () => {
    const { deps } = makeDeps();
    const applied = applyThermalImport(file, result, file.rooms, file.openings ?? [], userU, deps);
    const els = applied.project.rooms.flatMap((r) => r.constructions);
    const wall = els.find((e) => e.catalog_ref === "cat-1")!;
    expect(wall.u_value).toBe(0.25);
    expect(wall.project_construction_id).toMatch(/^proj-/);
    const g = applied.importGeometry;
    expect(g.trueNorthDeg).toBe(12);
    expect(g.roomPolygons).toHaveLength(2);
    expect(g.roomPolygons.every((p) => p.floorZ !== undefined)).toBe(true);
    expect(g.constructionGeometries).toHaveLength(file.constructions.length);
    expect(g.openingGeometries).toHaveLength(2);
  });
});
