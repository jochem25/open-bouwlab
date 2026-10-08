import { describe, expect, it } from "vitest";

import { ifcAnalyseMock, MOCK_ROOM_HOOG, MOCK_ROOM_LAAG } from "./__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "./__fixtures__/ifcImportTestData";
import { ifcAnalyseBlocking } from "./__fixtures__/ifcAnalyseVariants";
import { stripAnalyse } from "./ifcAnalyse";
import { buildIfcImportChecklist } from "./ifcImportChecklist";
import type { ProjectConstruction } from "../components/modeller/types";
import type { Project } from "../types";
import type { IfcImportOrigin } from "../types/ifcImport";

function setup() {
  const project = structuredClone(makeImportResult(ifcAnalyseMock.thermal).project) as Project;
  project.rooms[0]!.name = "13. Toilet"; // herkend
  project.rooms[0]!.function = "toilet";
  project.rooms[1]!.name = "12. Ruimte X"; // niet herkend
  project.rooms.forEach((r) => (r.heating_system = "radiator_lt"));
  const origin: IfcImportOrigin = {
    ifc_filename: "voorbeeld.ifc",
    engine_name: "e",
    engine_version: "1",
    imported_at: "2026-10-08T00:00:00Z",
    rooms_count: 2,
    approved_room_ids: [MOCK_ROOM_HOOG, MOCK_ROOM_LAAG],
    defaults: {
      heating_system: "radiator_lt",
      ventilation_system: project.ventilation.system_type,
      theta_e: project.climate.theta_e ?? 0,
      qv10: project.building.qv10,
      building_type: project.building.building_type,
    },
  };
  return { project, origin };
}

const ids = (items: { id: string }[]) => items.map((i) => i.id);

describe("buildIfcImportChecklist", () => {
  it("zonder herkomst: leeg", () => {
    expect(buildIfcImportChecklist(setup().project, null)).toEqual([]);
  });

  it("telt vlakken zonder U (u_value 0), met buiten-deel en beschrijving", () => {
    const { project, origin } = setup();
    const items = buildIfcImportChecklist(project, origin);
    const u = items.find((i) => i.id === "u-constructies")!;
    expect(u.severity).toBe("ontbreekt");
    expect(u.link).toBe("/constructies");
    const total = project.rooms.flatMap((r) => r.constructions).filter((c) => c.catalog_ref).length;
    expect(u.aantal).toBe(total);
    expect(u.tekst).toContain("tellen nu als 0 W/K");
    expect(u.tekst).toMatch(/waarvan \d+ aan buiten\/grond\/water/);
  });

  it("noemt het aantal L-GEEN-OPBOUW-bevindingen bij het U-ontbreekt-item", () => {
    const { project, origin } = setup();
    const withFinding = {
      ...origin,
      analyse: stripAnalyse(ifcAnalyseBlocking),
    };
    const u = buildIfcImportChecklist(project, withFinding).find((i) => i.id === "u-constructies")!;
    expect(u.tekst).toContain("3× bouwdeel zonder laagopbouw (L-GEEN-OPBOUW)");
    const zonder = buildIfcImportChecklist(project, origin).find((i) => i.id === "u-constructies")!;
    expect(zonder.tekst).not.toContain("L-GEEN-OPBOUW");
  });

  it("oplossen laat het item verdwijnen", () => {
    const { project, origin } = setup();
    for (const r of project.rooms) for (const c of r.constructions) c.u_value = 0.3;
    expect(ids(buildIfcImportChecklist(project, origin))).not.toContain("u-constructies");
  });

  it("openingen zonder U apart, open verbindingen overgeslagen", () => {
    const { project, origin } = setup();
    const room = project.rooms[0]!;
    const base = room.constructions[0]!;
    room.constructions.push(
      { ...base, id: "r-c90", catalog_ref: null, u_value: 0 },
      { ...base, id: "r-oc1", catalog_ref: null, u_value: 0 },
    );
    const open = buildIfcImportChecklist(project, origin).find((i) => i.id === "u-openingen")!;
    expect(open.aantal).toBe(1);
  });

  it("functies: geschat vs niet herkend, en oplossen", () => {
    const { project, origin } = setup();
    let items = buildIfcImportChecklist(project, origin);
    expect(items.find((i) => i.id === "functie-geschat")?.aantal).toBe(1);
    const onbekend = items.find((i) => i.id === "functie-onbekend")!;
    expect(onbekend.aantal).toBe(1);
    expect(onbekend.severity).toBe("default");
    expect(onbekend.link).toBe("/rooms");
    project.rooms[1]!.function = "bedroom";
    items = buildIfcImportChecklist(project, origin);
    expect(ids(items)).not.toContain("functie-onbekend");
  });

  it("defaults: verwarming, ventilatie en klimaat tonen zolang ze ongewijzigd zijn", () => {
    const { project, origin } = setup();
    let items = buildIfcImportChecklist(project, origin);
    expect(ids(items)).toEqual(expect.arrayContaining(["verwarming", "ventilatie", "klimaat"]));
    expect(items.find((i) => i.id === "ventilatie")?.link).toBe("/warmteverlies/instellingen");
    expect(items.find((i) => i.id === "klimaat")?.link).toBe("/project");

    project.rooms.forEach((r) => (r.heating_system = "floor_heating" as never));
    project.climate.theta_e = -8;
    project.ventilation.system_type = "system_d" as never;
    items = buildIfcImportChecklist(project, origin);
    expect(ids(items)).not.toContain("verwarming");
    expect(ids(items)).not.toContain("klimaat");
    expect(ids(items)).not.toContain("ventilatie");
  });

  it("alleen goedgekeurde ruimten tellen mee", () => {
    const { project, origin } = setup();
    const full = buildIfcImportChecklist(project, origin).find((i) => i.id === "u-constructies")!;
    const half = buildIfcImportChecklist(project, {
      ...origin,
      approved_room_ids: [MOCK_ROOM_HOOG],
    }).find((i) => i.id === "u-constructies")!;
    expect(half.aantal).toBeLessThan(full.aantal);
  });

  it("lagen met onbekend materiaal en zonder lambda: ontbreekt, per materiaal met m2 en aantal", () => {
    const { project, origin } = setup();
    for (const c of project.rooms[0]!.constructions) c.project_construction_id = "pc1";
    const area = project.rooms[0]!.constructions.reduce((s, c) => s + c.area, 0);
    const pcs = [
      {
        id: "pc1",
        layers: [
          { materialId: "holz", thickness: 100 },
          { materialId: "hout-naaldhout", thickness: 50 },
          { materialId: "onbekend-met-lambda", thickness: 20, lambdaOverride: 0.2 },
        ],
      },
    ] as unknown as ProjectConstruction[];
    const item = buildIfcImportChecklist(project, origin, pcs).find(
      (i) => i.id === "materiaal-onbekend",
    )!;
    expect(item.severity).toBe("ontbreekt");
    expect(item.link).toBe("/constructies");
    expect(item.aantal).toBe(1);
    expect(item.tekst).toContain("holz");
    expect(item.tekst).toContain("1 constructies");
    expect(item.tekst).toContain("R = 0");
    expect(item.tekst).not.toContain("hout-naaldhout");
    expect(item.tekst).not.toContain("onbekend-met-lambda");
    expect(area).toBeGreaterThan(0);

    // oplossen: lambda invullen laat het item verdwijnen
    pcs[0]!.layers[0]!.lambdaOverride = 0.13;
    expect(ids(buildIfcImportChecklist(project, origin, pcs))).not.toContain("materiaal-onbekend");
  });

  it("op naam gekoppelde materialen: inklapbare lijst; zonder analyse weggelaten", () => {
    const { project, origin } = setup();
    const withAnalyse = { ...origin, analyse: ifcAnalyseMock };
    const item = buildIfcImportChecklist(project, withAnalyse).find(
      (i) => i.id === "materiaal-gekoppeld",
    )!;
    expect(item.severity).toBe("default");
    expect(item.details?.some((d) => d.startsWith("n7_isolatie_PIR -> PIR (λ 0,023)"))).toBe(true);
    expect(ids(buildIfcImportChecklist(project, origin))).not.toContain("materiaal-gekoppeld");
  });

  it("toont NL-labels i.p.v. enumwaarden en legt qv10 = 0 uit", () => {
    const { project, origin } = setup();
    origin.defaults = {
      ...origin.defaults!,
      heating_system: "radiator_lt",
      ventilation_system: "system_c",
      building_type: "detached",
      qv10: 0,
      theta_e: project.climate.theta_e ?? 0,
    };
    project.ventilation.system_type = "system_c";
    project.building.building_type = "detached";
    project.building.qv10 = 0;
    const items = buildIfcImportChecklist(project, origin);
    const text = items.map((i) => i.tekst).join(" | ");
    expect(text).toContain("Radiator LT");
    expect(text).toContain("Systeem C");
    expect(text).toContain("Vrijstaand");
    expect(text).toContain("gunstigste luchtdichtheidsklasse");
    expect(text).not.toMatch(/radiator_lt|system_c|detached/);
  });
});
