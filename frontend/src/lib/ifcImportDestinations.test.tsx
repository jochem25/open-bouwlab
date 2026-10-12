/**
 * IFC-import op projectniveau: bestemmingskeuze, routes/redirects en migratie
 * van bestaande projecten (alleen synthetische fixtures).
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { renderToString as ssr } from "react-dom/server";
import { beforeEach, describe, expect, it } from "vitest";

import { DestinationPicker } from "../components/ifcImport/ImportResultView";
import { NAV_GROUPS } from "../components/layout/Sidebar";
import { useProjectStore, mergePersistedProjectStore, partializeProjectStore } from "../store/projectStore";
import type { Project } from "../types";
import type { IfcImportOrigin } from "../types/ifcImport";
import { IFC_IMPORT_ROUTE, LEGACY_REDIRECTS } from "./appRoutes";
import type { BengImportResult } from "./bengImport";
import { ifcAnalyseMock, MOCK_ROOM_HOOG, MOCK_ROOM_LAAG } from "./__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "./__fixtures__/ifcImportTestData";
import { stripAnalyse } from "./ifcAnalyse";
import { filterApprovedRooms } from "./ifcImportFilter";
import {
  IMPORT_DESTINATIONS,
  canTakeOver,
  defaultSelection,
  originDestinations,
  routeAfterImport,
  type DestinationContext,
  type ImportDestinationId,
} from "./ifcImportDestinations";
import { buildProjectEnvelope, importProject } from "./importExport";

const renderToString = (el: Parameters<typeof ssr>[0]) => ssr(el).replace(/<!-- -->/g, "");
const SRC = fileURLToPath(new URL("..", import.meta.url));

const compleet: BengImportResult = {
  status: "compleet",
  beng_geometry: { zones: [{ id: "rz-1", a_g_m2: 80, gevels: [] }] },
  ontbrekend: [],
  meldingen: ["Te controleren: IFC-route niet geijkt"],
  genegeerd: [],
  te_controleren: true,
  source: "synthetisch",
};

function ctx(over: Partial<DestinationContext> = {}): DestinationContext {
  const importResult = makeImportResult(ifcAnalyseMock.thermal);
  const approved = new Set([MOCK_ROOM_HOOG, MOCK_ROOM_LAAG]);
  return {
    filtered: filterApprovedRooms(ifcAnalyseMock.thermal, importResult, approved),
    approvedCount: approved.size,
    blockingCount: 0,
    bengCheck: null,
    ...over,
  };
}

function statuses(c: DestinationContext) {
  return Object.fromEntries(IMPORT_DESTINATIONS.map((d) => [d.id, d.status(c)])) as Record<
    ImportDestinationId,
    ReturnType<(typeof IMPORT_DESTINATIONS)[number]["status"]>
  >;
}

describe("bestemmingen", () => {
  it("register: warmteverlies en beng, elk met route; uitbreidbaar via de lijst", () => {
    expect(IMPORT_DESTINATIONS.map((d) => d.id)).toEqual(["warmteverlies", "beng"]);
    expect(routeAfterImport(new Set(["beng"]))).toBe("/beng");
    expect(routeAfterImport(new Set(["beng", "warmteverlies"]))).toBe("/rooms");
    expect(routeAfterImport(new Set())).toBe("/project");
  });

  it("zonder BENG-uitvoer: beng niet beschikbaar, alleen warmteverlies gekozen", () => {
    const s = statuses(ctx());
    expect(s.beng.state).toBe("niet_beschikbaar");
    expect(s.warmteverlies.state === "compleet" || s.warmteverlies.state === "onvolledig").toBe(true);
    expect([...defaultSelection(s)]).toEqual(["warmteverlies"]);
  });

  it("BENG compleet en te controleren: beide standaard gekozen", () => {
    const s = statuses(ctx({ bengCheck: { phase: "ready", result: compleet } }));
    expect(s.beng.state).toBe("te_controleren");
    expect(s.beng.summary).toContain("1 rekenzone(s)");
    expect([...defaultSelection(s)].sort()).toEqual(["beng", "warmteverlies"]);
  });

  it("BENG onvolledig: ontbrekende velden zichtbaar, niet over te nemen", () => {
    const paden = Array.from({ length: 12 }, (_, i) => `/beng_geometry/opaque_defs/${i}/thermal`);
    const s = statuses(
      ctx({
        bengCheck: {
          phase: "ready",
          result: { ...compleet, status: "onvolledig", beng_geometry: undefined, ontbrekend: paden },
        },
      }),
    );
    expect(s.beng.state).toBe("onvolledig");
    expect(s.beng.ontbrekend).toHaveLength(11);
    expect(s.beng.ontbrekend.at(-1)).toBe("en 2 meer");
    expect(canTakeOver("beng", s.beng)).toBe(false);
    expect(defaultSelection(s).has("beng")).toBe(false);
  });

  it("warmteverlies zonder goedgekeurde ruimten: niet beschikbaar; BENG blijft kiesbaar", () => {
    const s = statuses(ctx({ approvedCount: 0, bengCheck: { phase: "ready", result: compleet } }));
    expect(s.warmteverlies.state).toBe("niet_beschikbaar");
    expect([...defaultSelection(s)]).toEqual(["beng"]);
  });

  it("warmteverlies onvolledig toont de vlakken zonder U als ontbrekend", () => {
    const c = ctx();
    const room = c.filtered.importResult.project.rooms[0]!;
    room.constructions[0]!.u_value = 0;
    room.constructions[0]!.description = "Testwand";
    const s = statuses(c);
    expect(s.warmteverlies.state).toBe("onvolledig");
    expect(s.warmteverlies.ontbrekend.some((m) => m.startsWith("U-waarde: Testwand"))).toBe(true);
    expect(canTakeOver("warmteverlies", s.warmteverlies)).toBe(true);
  });

  it("BENG-status bezig en fout", () => {
    expect(statuses(ctx({ bengCheck: { phase: "loading" } })).beng.state).toBe("bezig");
    const f = statuses(ctx({ bengCheck: { phase: "error", message: "422 /x" } })).beng;
    expect(f).toMatchObject({ state: "fout", summary: "422 /x" });
  });

  it("DestinationPicker: status per bestemming, uitgeschakeld als niet over te nemen", () => {
    const s = statuses(ctx());
    const html = renderToString(
      <DestinationPicker statuses={s} chosen={new Set(["warmteverlies"])} onToggle={() => {}} />,
    );
    expect(html).toContain('data-destination="warmteverlies"');
    expect(html).toMatch(/data-destination="beng" data-state="niet_beschikbaar"/);
    expect(html).toContain("Ook BENG-schil");
    const bengLi = html.slice(html.indexOf('data-destination="beng"'));
    expect(bengLi).toMatch(/<input[^>]*disabled/);
  });
});

describe("routes: IFC-import op projectniveau", () => {
  it("oude paden gaan naar de nieuwe route", () => {
    expect(IFC_IMPORT_ROUTE).toBe("/project/ifc-import");
    expect(LEGACY_REDIRECTS).toEqual([
      { from: "/ifc-import", to: IFC_IMPORT_ROUTE },
      { from: "/ifc-reconstructie", to: IFC_IMPORT_ROUTE },
    ]);
  });

  it("navigatie: onder Project, niet meer onder Warmteverlies; Project alleen exact actief", () => {
    const project = NAV_GROUPS.find((g) => g.key === "project")!;
    const wv = NAV_GROUPS.find((g) => g.key === "warmteverlies")!;
    expect(project.items.map((i) => i.to)).toContain(IFC_IMPORT_ROUTE);
    expect(wv.items.map((i) => i.to)).not.toContain(IFC_IMPORT_ROUTE);
    expect(project.items.find((i) => i.to === "/project")?.end).toBe(true);
  });

  it("App registreert de route en de redirects uit appRoutes", () => {
    const app = readFileSync(join(SRC, "App.tsx"), "utf-8");
    expect(app).toContain("path={IFC_IMPORT_ROUTE}");
    expect(app).toMatch(/LEGACY_REDIRECTS\.map\([\s\S]*<Navigate to=\{r\.to\} replace/);
  });

  it("geen dode links: nergens meer een hard pad naar /ifc-import", () => {
    const hits: string[] = [];
    const walk = (dir: string) => {
      for (const name of readdirSync(dir)) {
        const p = join(dir, name);
        if (statSync(p).isDirectory()) {
          if (name !== "__fixtures__") walk(p);
        } else if (/\.(ts|tsx)$/.test(name) && !/\.test\.tsx?$/.test(name) && name !== "appRoutes.ts") {
          if (/["'`]\/ifc-(import|reconstructie)["'`]/.test(readFileSync(p, "utf-8"))) hits.push(p);
        }
      }
    };
    walk(SRC);
    expect(hits).toEqual([]);
  });
});

describe("migratie: bestaand project met een eerdere IFC-import", () => {
  const oldOrigin: IfcImportOrigin = {
    ifc_filename: "oud.ifc",
    engine_name: "ifc-ruimtebalans",
    engine_version: "0.1.0",
    imported_at: "2026-10-08T12:00:00Z",
    rooms_count: 2,
    approved_room_ids: [MOCK_ROOM_HOOG, MOCK_ROOM_LAAG],
    room_function_sources: { [MOCK_ROOM_HOOG]: "model", [MOCK_ROOM_LAAG]: "default" },
    analyse: stripAnalyse(ifcAnalyseMock),
  };

  beforeEach(() => {
    const project = structuredClone(makeImportResult(ifcAnalyseMock.thermal).project) as Project;
    useProjectStore.getState().setProject(project);
  });

  it("oud record zonder destinations geldt als warmteverlies; onbekende waarden vallen weg", () => {
    expect(originDestinations(oldOrigin)).toEqual(["warmteverlies"]);
    expect(originDestinations(null)).toEqual([]);
    expect(originDestinations({ ...oldOrigin, destinations: ["beng", "ventilatie"] })).toEqual(["beng"]);
  });

  it("projectbestand: oud record komt ongewijzigd terug en blijft gekoppeld", () => {
    useProjectStore.getState().setIfcImport(oldOrigin);
    const envelope = buildProjectEnvelope(useProjectStore.getState().project, null);
    const back = importProject(JSON.stringify(envelope));
    expect(back.type).toBe("project");
    const origin = (back as { ifcImport?: IfcImportOrigin }).ifcImport;
    // Via JSON: -0 wordt 0 (geen dataverlies).
    expect(origin).toEqual(JSON.parse(JSON.stringify(oldOrigin)));
    expect(originDestinations(origin)).toEqual(["warmteverlies"]);
  });

  it("nieuw record met BENG overleeft bestand en localStorage", () => {
    const nieuw: IfcImportOrigin = {
      ...oldOrigin,
      destinations: ["warmteverlies", "beng"],
      beng: {
        imported_at: "2026-10-12T15:00:00Z",
        source: "synthetisch",
        status: "compleet",
        te_controleren: true,
        zones: 1,
        vlakken: 6,
      },
    };
    useProjectStore.getState().setIfcImport(nieuw);
    const envelope = buildProjectEnvelope(useProjectStore.getState().project, null);
    const back = (importProject(JSON.stringify(envelope)) as { ifcImport?: IfcImportOrigin }).ifcImport;
    expect(back).toEqual(JSON.parse(JSON.stringify(nieuw)));

    const slim = JSON.parse(JSON.stringify(partializeProjectStore(useProjectStore.getState())));
    const rehydrated = mergePersistedProjectStore(slim, useProjectStore.getState());
    expect(rehydrated.ifcImport?.destinations).toEqual(["warmteverlies", "beng"]);
    expect(rehydrated.ifcImport?.beng?.vlakken).toBe(6);
    expect(rehydrated.ifcImport?.approved_room_ids).toEqual(oldOrigin.approved_room_ids);
  });
});
