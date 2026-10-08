import { afterEach, describe, expect, it } from "vitest";

import { useModellerStore } from "../components/modeller/modellerStore";
import type { ProjectConstruction } from "../components/modeller/types";
import {
  mergePersistedProjectStore,
  partializeProjectStore,
  useProjectStore,
} from "../store/projectStore";
import type { ConstructionElement, Project, Room } from "../types";
import type { IfcImportOrigin, IfcMaterialSuggestion } from "../types/ifcImport";
import { buildIfcImportChecklist, computeMaterialSuggestions } from "./ifcImportChecklist";
import { matchIfcMaterial } from "./ifcMaterialMatcher";
import { remapUnknownMaterial } from "./ifcMaterialRemap";
import { getMaterialById } from "./materialsDatabase";

const UNKNOWN = "fibo trespo";
const HPL = "plaatmateriaal-hpl-trespa";

const REPORT = {
  materialen: [
    {
      naam: "Fibo Trespo",
      status: "onbekend",
      verwachte_koppeling: "geen",
      modelleur_moet_hernoemen: true,
      aantal_lagen: 7,
    },
    {
      naam: "NLRS_i4_multiplex_wit_beplating",
      verwacht_materiaal_id: "hout-hardhout-multiplex",
      verwachte_koppeling: "keyword",
      voorstel_v2_5: "hout_vuren_generiek",
      modelleur_moet_hernoemen: false,
    },
    { naam: "Baksteen", status: "exact", modelleur_moet_hernoemen: false },
  ],
};

function pc(id: string, layers: ProjectConstruction["layers"]): ProjectConstruction {
  return {
    id,
    name: `Wand ${id}`,
    category: "wanden",
    materialType: "masonry",
    verticalPosition: "wall",
    layers,
  } as ProjectConstruction;
}

function el(id: string, pcId: string): ConstructionElement {
  return {
    id,
    description: "Wand",
    area: 10,
    u_value: 0,
    boundary_type: "exterior",
    material_type: "masonry",
    vertical_position: "wall",
    project_construction_id: pcId,
    catalog_ref: pcId,
  };
}

function seed(): void {
  const room: Room = {
    id: "r1",
    name: "Ruimte",
    function: "living_room",
    floor_area: 20,
    heating_system: "radiator_ht",
    constructions: [el("e1", "pc-a"), el("e2", "pc-b"), el("e3", "pc-c")],
  };
  const project = { ...useProjectStore.getState().project, rooms: [room] } as Project;
  const origin: IfcImportOrigin = {
    ifc_filename: "x.ifc",
    engine_name: "e",
    engine_version: "1",
    imported_at: "2026-10-08T00:00:00Z",
    rooms_count: 1,
    approved_room_ids: ["r1"],
    material_suggestions: computeMaterialSuggestions(REPORT),
  };
  useProjectStore.setState({ project, ifcImport: origin, _past: [], _future: [] });
  useModellerStore.setState({
    projectConstructions: [
      pc("pc-a", [
        { materialId: UNKNOWN, thickness: 8 },
        { materialId: "hout-osb", thickness: 18 },
      ]),
      pc("pc-b", [{ materialId: UNKNOWN, thickness: 10 }]),
      pc("pc-c", [{ materialId: "hout-osb", thickness: 18 }]),
    ],
  });
}

const items = () =>
  buildIfcImportChecklist(
    useProjectStore.getState().project,
    useProjectStore.getState().ifcImport,
    useModellerStore.getState().projectConstructions,
  );

afterEach(() => {
  useProjectStore.getState().reset();
  useModellerStore.setState({ projectConstructions: [] });
});

describe("HPL-plaat (Trespa Meteon)", () => {
  it("staat in de database met lambda 0,3", () => {
    expect(getMaterialById(HPL)?.lambda).toBe(0.3);
  });
  it.each(["Trespa Meteon 8mm", "HPL", "n7_gevel_trespa"])("matcht %s", (name) => {
    expect(matchIfcMaterial(name).material?.id).toBe(HPL);
  });
  it("matcht 'Fibo Trespo' niet automatisch", () => {
    expect(matchIfcMaterial("Fibo Trespo").material?.id).not.toBe(HPL);
  });
});

describe("computeMaterialSuggestions", () => {
  it("houdt alleen regels met voorstel of hernoemadvies", () => {
    expect(computeMaterialSuggestions(REPORT)).toEqual<IfcMaterialSuggestion[]>([
      { name: "Fibo Trespo", rename_advised: true },
      {
        name: "NLRS_i4_multiplex_wit_beplating",
        expected_material_id: "hout-hardhout-multiplex",
        proposal: "hout_vuren_generiek",
        rename_advised: false,
      },
    ]);
  });
  it("is defensief bij ontbrekend report", () => {
    expect(computeMaterialSuggestions(undefined)).toEqual([]);
    expect(computeMaterialSuggestions({ materialen: "x" })).toEqual([]);
  });
});

describe("remapUnknownMaterial", () => {
  it("toont het onbekende item met engine-voorstel", () => {
    seed();
    const item = items().find((i) => i.id === "materiaal-onbekend")!;
    expect(item.onbekendeMaterialen).toHaveLength(1);
    expect(item.onbekendeMaterialen![0]!.suggestion?.rename_advised).toBe(true);
  });

  it("past alle lagen met die naam aan, herberekent U en laat het item verdwijnen", () => {
    seed();
    expect(items().some((i) => i.id === "materiaal-onbekend")).toBe(true);
    expect(remapUnknownMaterial(UNKNOWN, HPL)).toBe(2);

    const pcs = useModellerStore.getState().projectConstructions;
    expect(pcs.flatMap((p) => p.layers).some((l) => l.materialId === UNKNOWN)).toBe(false);
    expect(pcs[0]!.layers[0]!.materialId).toBe(HPL);
    expect(pcs[1]!.layers[0]!.materialId).toBe(HPL);

    const cs = useProjectStore.getState().project.rooms[0]!.constructions;
    expect(cs.find((c) => c.id === "e1")!.u_value).toBeGreaterThan(0);
    expect(cs.find((c) => c.id === "e2")!.u_value).toBeGreaterThan(0);
    expect(cs.find((c) => c.id === "e3")!.u_value).toBe(0); // niet getroffen
    expect(items().some((i) => i.id === "materiaal-onbekend")).toBe(false);
  });
});

describe("material_suggestions persist", () => {
  it("gaat mee in persist en wordt hersteld", () => {
    seed();
    const persisted = partializeProjectStore(useProjectStore.getState());
    expect(persisted.ifcImport?.material_suggestions).toHaveLength(2);
    const merged = mergePersistedProjectStore(
      JSON.parse(JSON.stringify(persisted)),
      useProjectStore.getState(),
    );
    expect(merged.ifcImport?.material_suggestions?.[0]?.name).toBe("Fibo Trespo");
  });
});
