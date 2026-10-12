import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  buildLegend,
  crossCheckGevelIds,
  defaultViewerMode,
  elementKind,
  legendKey,
  legendLabel,
  normalizeOrientatie,
  vlakInfoFromPsets,
  vlakInfoRows,
  type BengVlakInfo,
  type RawPropertySet,
} from "./bengShell";
import { loadBengShell } from "./bengShellLoader";

const FIXTURE = fileURLToPath(new URL("./__fixtures__/beng-schil-synthetisch.ifc", import.meta.url));
/** Echte uitvoer van ifc-ruimtebalans (synthetische tussenwoning, B1b 12-10-2026). */
const RB_IFC = fileURLToPath(new URL("./__fixtures__/rb-tussenwoning_beng.ifc", import.meta.url));
const RB_DOC = fileURLToPath(new URL("./__fixtures__/rb-tussenwoning.beng.json", import.meta.url));

function pset(name: string, props: Record<string, unknown>): RawPropertySet {
  return {
    Name: { value: name },
    HasProperties: Object.entries(props).map(([k, v]) => ({ Name: { value: k }, NominalValue: { value: v } })),
  };
}

const gevelZ: BengVlakInfo = {
  gevel_id: "g-1",
  zone: "rz-1",
  vlak_type: "gevel",
  grenst_aan: "buitenlucht",
  orientatie: "Z",
  helling_deg: 90,
  A_T_m2: 28,
  constructie_ref: "od-gevel",
  herkomst: "ifc_afgeleid",
  extra: {},
};

describe("vlakInfoFromPsets", () => {
  it("leest de vaste velden, getallen als number, de rest in extra", () => {
    const info = vlakInfoFromPsets([
      pset("Pset_Overig", { gevel_id: "fout" }),
      pset("Pset_KBA_BENG", {
        gevel_id: "g-rz-1-03",
        zone: "rz-1",
        helling_deg: 90,
        A_T_m2: "28.5",
        bedekking: "deels",
        leeg: null,
      }),
    ]);
    expect(info).toEqual({
      gevel_id: "g-rz-1-03",
      zone: "rz-1",
      helling_deg: 90,
      A_T_m2: 28.5,
      extra: { bedekking: "deels" },
    });
  });

  it("geeft null zonder Pset_KBA_BENG", () => {
    expect(vlakInfoFromPsets([pset("Pset_WallCommon", { IsExternal: true })])).toBeNull();
  });
});

describe("orientatie en legenda", () => {
  it("normaliseert korte codes en snake_case-namen", () => {
    expect(normalizeOrientatie("zuid_west")).toBe("ZW");
    expect(normalizeOrientatie("NO")).toBe("NO");
    expect(normalizeOrientatie("horizontaal")).toBe("H");
    expect(normalizeOrientatie("noordoost")).toBe("NO");
    expect(normalizeOrientatie("xx")).toBeNull();
  });

  it("legendasleutel per begrenzing, buitenlucht per orientatie, deels apart", () => {
    expect(legendKey(gevelZ)).toBe("buitenlucht:Z");
    expect(legendKey({ ...gevelZ, orientatie: "zuid" })).toBe("buitenlucht:Z");
    expect(legendKey({ ...gevelZ, grenst_aan: "vloer_op_maaiveld_boven_grond", orientatie: undefined })).toBe(
      "vloer_op_maaiveld_boven_grond",
    );
    const bel = { ...gevelZ, grenst_aan: "aangrenzende_verwarmde_ruimte" };
    expect(legendKey(bel)).toBe("aangrenzende_verwarmde_ruimte");
    expect(legendKey({ ...bel, extra: { bedekking: "deels" } })).toBe("aangrenzende_verwarmde_ruimte:deels");
    expect(legendKey({ ...bel, herkomst: "melding BENG-BELENDING-DEELS" })).toBe(
      "aangrenzende_verwarmde_ruimte:deels",
    );
    expect(legendKey({ extra: {}, window_def: "wd-A", gevel_id: "g-1" })).toBe("raam");
    expect(legendKey({ extra: { soort: "deur" }, window_def: "wd-B", gevel_id: "g-1" })).toBe("raam");
    expect(legendKey({ extra: {}, vlak_type: "belending" })).toBe("belending");
    expect(legendKey(null)).toBe("overig");
    expect(legendLabel("buitenlucht:ZW")).toBe("Buitenlucht zuidwest");
    expect(legendLabel("aangrenzende_verwarmde_ruimte:deels")).toBe("Aangrenzend gebouw (belending), deels bedekt");
  });

  it("buildLegend telt A_T per groep, sorteert buitenlucht eerst en meldt afwijkende kleuren", () => {
    const { entries, mixedColors } = buildLegend([
      { info: { ...gevelZ, orientatie: "N" }, color: "#2563eb" },
      { info: gevelZ, color: "#f05433" },
      { info: { ...gevelZ, gevel_id: "g-2", A_T_m2: 2 }, color: "#F05433" },
      { info: { ...gevelZ, grenst_aan: "water" }, color: "#1a6b8a" },
      { info: { extra: {}, window_def: "wd-A", A_T_m2: 4.8 }, color: "#99d9ff" },
      { info: { extra: {}, window_def: "wd-B", A_T_m2: 2 }, color: "#2e86c1" },
      { info: { extra: {}, window_def: "wd-A", A_T_m2: 2 }, color: "#99d9ff" },
      { info: { ...gevelZ, gevel_id: "g-3", orientatie: "N" }, color: "#000000" },
    ]);
    expect(entries.map((e) => [e.key, e.count, e.a_t_m2])).toEqual([
      ["buitenlucht:N", 2, 56],
      ["buitenlucht:Z", 2, 30],
      ["water", 1, 28],
      ["raam", 3, 0],
    ]);
    // Ramen: kleur per kozijnmerk is bedoeld, geen afwijking.
    expect(mixedColors).toEqual(["buitenlucht:N"]);
    expect(entries.find((e) => e.key === "raam")?.merken).toBe(2);
  });
});

describe("vlakInfoRows", () => {
  it("toont gevel_id, zone, grenst_aan, orientatie, helling, A_T, constructie en herkomst", () => {
    const rows = vlakInfoRows({ ...gevelZ, extra: { omschrijving: "voorgevel" } });
    expect(rows).toEqual([
      ["Gevel-id", "g-1"],
      ["Zone", "rz-1"],
      ["Vlaktype", "gevel"],
      ["Grenst aan", "Buitenlucht (buitenlucht)"],
      ["Orientatie", "Z (zuid)"],
      ["Helling", "90 graden"],
      ["A_T", "28 m2"],
      ["Constructie", "od-gevel"],
      ["omschrijving", "voorgevel"],
      ["Herkomst", "ifc_afgeleid"],
    ]);
  });

  it("raam: kozijnmerk en gastgevel; zonder Pset de elementnaam", () => {
    const rows = vlakInfoRows({ extra: {}, window_def: "wd-A", gevel_id: "g-1", A_T_m2: 4.8 });
    expect(rows.slice(0, 2)).toEqual([
      ["Kozijnmerk", "wd-A"],
      ["In gevel", "g-1"],
    ]);
    expect(rows).toContainEqual(["A_T", "4,8 m2"]);
    expect(vlakInfoRows(null, "belending west")).toEqual([["Element", "belending west"]]);
  });

  it("deels bedekt komt als eigen regel", () => {
    const rows = vlakInfoRows({ ...gevelZ, grenst_aan: "aangrenzende_verwarmde_ruimte", extra: { bedekking: "deels" } });
    expect(rows).toContainEqual(["Belending", "deels bedekt"]);
    expect(rows.find(([k]) => k === "bedekking")).toBeUndefined();
  });
});

describe("schakelaar: begin-modus", () => {
  it("BENG-schil bij 'te controleren' met model, anders warmteverlies", () => {
    expect(defaultViewerMode({ hasBengShell: true, teControleren: true })).toBe("beng");
    expect(defaultViewerMode({ hasBengShell: true, teControleren: false })).toBe("warmteverlies");
    expect(defaultViewerMode({ hasBengShell: false, teControleren: true })).toBe("warmteverlies");
  });
});

describe("crossCheckGevelIds", () => {
  it("meldt id's die alleen in model of alleen in beng.json staan; ramen tellen niet", () => {
    const r = crossCheckGevelIds(
      [gevelZ, { ...gevelZ, gevel_id: "g-9" }, { extra: {}, window_def: "wd", gevel_id: "g-1" }, null],
      { beng_geometry: { zones: [{ gevels: [{ id: "g-1" }, { id: "g-2" }] }] } },
    );
    expect(r).toEqual({ alleenModel: ["g-9"], alleenDocument: ["g-2"] });
    expect(elementKind(null)).toBe("overig");
  });
});

describe("loadBengShell (web-ifc, synthetische tussenwoning)", () => {
  it("leest vlakken, raam en belending met kleur, Pset en wereldcoordinaten in m", async () => {
    const els = await loadBengShell(new Uint8Array(readFileSync(FIXTURE)), { coordinateToOrigin: false });
    expect(els).toHaveLength(8);
    const byName = new Map(els.map((e) => [e.name, e]));

    const zuid = byName.get("g-rz-1-03")!;
    expect(zuid.ifcType).toBe("IfcBuildingElementProxy");
    expect(zuid.info?.grenst_aan).toBe("buitenlucht");
    expect(zuid.info?.A_T_m2).toBe(28);
    expect(zuid.color).toBe("#f05433");
    expect(zuid.opacity).toBe(1);
    // Gevel met raamgat: meer dan 2 driehoeken.
    expect(zuid.indices.length / 3).toBeGreaterThan(2);

    // Bbox van alle schilvlakken: 5 x 5,6 x 8 m; Y-up, wereld = (x, z, -y).
    const min = [Infinity, Infinity, Infinity];
    const max = [-Infinity, -Infinity, -Infinity];
    for (const e of els.filter((x) => x.info?.gevel_id && !x.info.window_def)) {
      for (let i = 0; i < e.positions.length; i += 3) {
        for (let k = 0; k < 3; k++) {
          min[k] = Math.min(min[k]!, e.positions[i + k]!);
          max[k] = Math.max(max[k]!, e.positions[i + k]!);
        }
      }
    }
    expect(min.map((v) => +v.toFixed(3))).toEqual([0, 0, -8]);
    expect(max.map((v) => +v.toFixed(3))).toEqual([5, 5.6, 0]);

    const raam = byName.get("wd-A")!;
    expect(raam.info?.window_def).toBe("wd-A");
    expect(raam.opacity).toBeCloseTo(0.7, 5);

    const bel = byName.get("belending west")!;
    expect(bel.info?.vlak_type).toBe("belending");
    expect(bel.opacity).toBeCloseTo(0.3, 5);

    const legend = buildLegend(els.map((e) => ({ info: e.info, color: e.color })));
    expect(legend.entries.map((e) => e.key)).toEqual([
      "buitenlucht:N",
      "buitenlucht:Z",
      "buitenlucht:H",
      "aangrenzende_verwarmde_ruimte",
      "aangrenzende_verwarmde_ruimte:deels",
      "vloer_op_maaiveld_boven_grond",
      "raam",
      "belending",
    ]);
    expect(legend.mixedColors).toEqual([]);
  });

  it("standaard naar de oorsprong geschoven (RD-modellen): zelfde maten, kleinere coordinaten", async () => {
    const els = await loadBengShell(new Uint8Array(readFileSync(FIXTURE)));
    const zuid = els.find((e) => e.name === "g-rz-1-03")!;
    const xs = [...zuid.positions].filter((_, i) => i % 3 === 0);
    expect(Math.max(...xs) - Math.min(...xs)).toBeCloseTo(5, 6);
    for (const v of zuid.positions) expect(Math.abs(v)).toBeLessThan(10);
  });
});

describe("contract met ifc-ruimtebalans (hun tussenwoning-uitvoer)", () => {
  it("vlakken, raam en belendingen; gevel_id's en som A_T kloppen met beng.json", async () => {
    const els = await loadBengShell(new Uint8Array(readFileSync(RB_IFC)));
    const doc = JSON.parse(readFileSync(RB_DOC, "utf-8")) as {
      beng_geometry: { zones: Array<{ gevels: Array<{ id: string; bruto_buiten_opp_m2: number }> }> };
    };
    const soort = (k: string) => els.filter((e) => elementKind(e.info) === k).length;
    expect([soort("vlak"), soort("raam"), soort("belending"), soort("overig")]).toEqual([6, 1, 2, 0]);
    expect(crossCheckGevelIds(els.map((e) => e.info), doc)).toEqual({ alleenModel: [], alleenDocument: [] });

    const gevels = doc.beng_geometry.zones.flatMap((z) => z.gevels);
    const somDoc = gevels.reduce((n, g) => n + g.bruto_buiten_opp_m2, 0);
    const somModel = els.filter((e) => elementKind(e.info) === "vlak").reduce((n, e) => n + (e.info?.A_T_m2 ?? 0), 0);
    expect(somModel).toBeCloseTo(somDoc, 6);

    const legend = buildLegend(els.map((e) => ({ info: e.info, color: e.color })));
    expect(legend.mixedColors).toEqual([]);
    expect(legend.entries.map((e) => e.key)).toContain("aangrenzende_verwarmde_ruimte:deels");
    for (const e of els.filter((x) => elementKind(x.info) === "vlak")) {
      expect(e.info?.herkomst).toBeTruthy();
      expect(e.info?.zone).toBe("rz-1");
    }
  });
});
