import { describe, expect, it } from "vitest";

import { saturationPressure } from "./glaserCalculation";
import {
  DELTA_P_MAX,
  HUMIDITY_SAFETY_FACTOR,
  deltaPForClass,
  internalVapourPressure,
} from "./humidityClasses";
import {
  MONTHLY_CLIMATE_NL,
  type YearlyMoistureResult,
  calculateYearlyMoisture,
} from "./yearlyMoistureCalculation";

/**
 * Dakopbouw uit het rapport "Gips 10 | MWd 100 | Spouw 40 | Nhout 25 | EPS 100 |
 * hout-spaanplaat 3 | spouw sterk gevent." (Rc 5,74), binnen naar buiten.
 * Condensatievlak verwacht op 175 mm (Naaldhout | EPS).
 */
const DAK_LAGEN = [
  { materialId: "plaatmateriaal-gipskartonplaat", thickness: 10 },
  { materialId: "isolatie-mineraal-minerale-wol-dekens", thickness: 100 },
  { materialId: "spouw-spouw-niet-gevent-rd-0-17", thickness: 40 },
  { materialId: "hout-naaldhout", thickness: 25 },
  { materialId: "isolatie-kunststof-eps", thickness: 100 },
  { materialId: "hout-spaanplaat", thickness: 3 },
  { materialId: "spouw-spouw-sterk-gevent-rd-0", thickness: 30 },
];

const THETA_I = 20;
const RH_I = 50;
const WINTER = ["Jan", "Feb", "Dec"];

function forClass(humidityClass: 1 | 2 | 3 | 4 | 5): YearlyMoistureResult {
  const r = calculateYearlyMoisture(DAK_LAGEN, "ceiling", THETA_I, RH_I, undefined, {
    mode: "class",
    humidityClass,
  });
  if (!r) throw new Error("geen resultaat");
  return r;
}

function month(r: YearlyMoistureResult, name: string) {
  const m = r.months.find((x) => x.month === name);
  if (!m) throw new Error(`maand ${name} ontbreekt`);
  return m;
}

describe("humidityClasses", () => {
  it("Δp is maximaal bij θe ≤ 0 °C en lineair naar 0 bij θe ≥ 20 °C", () => {
    expect(deltaPForClass(3, -10)).toBe(DELTA_P_MAX[3]);
    expect(deltaPForClass(3, 0)).toBe(DELTA_P_MAX[3]);
    expect(deltaPForClass(3, 10)).toBeCloseTo(DELTA_P_MAX[3] / 2, 9);
    expect(deltaPForClass(3, 20)).toBe(0);
    expect(deltaPForClass(5, 25)).toBe(0);
  });

  it("pi = pe + 1,10·Δp; bij θe ≥ 20 °C is pi = pe", () => {
    expect(internalVapourPressure(4, 0, 500)).toBeCloseTo(500 + 1.1 * 1080, 9);
    expect(HUMIDITY_SAFETY_FACTOR).toBe(1.1);
    expect(internalVapourPressure(5, 20, 1700)).toBe(1700);
    expect(internalVapourPressure(5, 22, 1700)).toBe(1700);
  });
});

describe("calculateYearlyMoisture - vochtklassen (dak, forfaitair klimaat)", () => {
  it("kritiek vlak volgt de pi-methode: klasse 4 condenseert op 110 mm (MW | spouw)", () => {
    // Het vlak wordt gekozen met dezelfde pi (koudste maand) als de jaarbalans.
    // Bij modus c (20 °C / 50 %) is dat 175 mm (Naaldhout | EPS), zie hieronder.
    const r = forClass(4);
    expect(r.planePosition).toBe(110);
    expect(r.planeInnerLayer).toBe("Minerale wol (dekens)");
  });

  it("pi varieert per maand en volgt pe + 1,10·Δp(θe)", () => {
    const r = forClass(3);
    const jan = month(r, "Jan");
    const jul = month(r, "Jul");
    const peJan = (87 / 100) * saturationPressure(3.4);
    expect(jan.pe).toBeCloseTo(peJan, 6);
    expect(jan.pi).toBeCloseTo(peJan + 1.1 * 810 * (20 - 3.4) / 20, 6);
    expect(jul.pi).toBeLessThan(jan.pi + 1000);
    expect(new Set(r.months.map((m) => Math.round(m.pi))).size).toBeGreaterThan(1);
    expect(r.internal).toEqual({
      mode: "class",
      thetaI: THETA_I,
      humidityClass: 3,
      roomTypeLabel: undefined,
    });
  });

  it("klasse 2: het hele jaar droog", () => {
    const r = forClass(2);
    expect(r.maxMa).toBeLessThanOrEqual(0.1);
    expect(r.months.every((m) => m.status === "dry")).toBe(true);
  });

  it("klasse 3: het hele jaar droog (pi in januari onder pSat op het vlak)", () => {
    const r = forClass(3);
    expect(r.maxMa).toBeLessThanOrEqual(0.1);
    expect(r.months.every((m) => m.status === "dry")).toBe(true);
    const jan = month(r, "Jan");
    expect(jan.pC).toBeLessThan(jan.pSatC);
  });

  it("klasse 4: condensatie in de wintermaanden, droogt in de zomer uit", () => {
    const r = forClass(4);
    for (const name of WINTER) {
      const m = month(r, name);
      expect(m.status).toBe("condensation");
      expect(m.gc).toBeGreaterThan(0);
      expect(m.pC).toBeCloseTo(m.pSatC, 9);
    }
    expect(r.maxMa).toBeGreaterThan(0.1);
    expect(month(r, "Jul").status).toBe("dry");
    // Droogt in de zomer volledig uit (ma = 0 in mei t/m okt).
    expect(month(r, "Jun").ma).toBeLessThanOrEqual(0.1);
  });

  it("p_c is in een droge maand het lineaire verloop tussen pi en pe", () => {
    const r = forClass(2);
    const jan = month(r, "Jan");
    expect(jan.pC).toBeGreaterThan(jan.pe);
    expect(jan.pC).toBeLessThan(jan.pi);
  });
});

describe("calculateYearlyMoisture - eigen binnenklimaat (modus c)", () => {
  it("reproduceert het oude resultaat: pi constant = RV_i·psat(θi)", () => {
    const rDefault = calculateYearlyMoisture(DAK_LAGEN, "ceiling", THETA_I, RH_I);
    const rConst = calculateYearlyMoisture(DAK_LAGEN, "ceiling", THETA_I, RH_I, undefined, {
      mode: "constant",
    });
    expect(rConst).toEqual(rDefault);
    const piOld = (RH_I / 100) * saturationPressure(THETA_I);
    expect(rConst?.months.every((m) => Math.abs(m.pi - piOld) < 1e-9)).toBe(true);
    // Oude bevinding uit de aanleiding: elke maand droog bij 20 °C / 50 %.
    expect(rConst?.months.every((m) => m.status === "dry")).toBe(true);
    expect(rConst?.internal).toEqual({ mode: "constant", thetaI: THETA_I, rhI: RH_I });
    expect(rConst?.planePosition).toBe(175);
    expect(rConst?.planeOuterLayer).toBe("EPS");
    expect(MONTHLY_CLIMATE_NL).toHaveLength(12);
  });
});

describe("calculateYearlyMoisture - rand θe ≥ 20 °C", () => {
  it("Δp = 0: pi = pe, geen condensatie, ook in klasse 5", () => {
    const warm = MONTHLY_CLIMATE_NL.map((m) => ({ ...m, thetaE: 22, rhE: 80 }));
    const r = calculateYearlyMoisture(DAK_LAGEN, "ceiling", THETA_I, RH_I, warm, {
      mode: "class",
      humidityClass: 5,
    });
    expect(r).not.toBeNull();
    for (const m of r?.months ?? []) {
      expect(m.pi).toBeCloseTo(m.pe, 9);
      expect(m.status).toBe("dry");
    }
  });
});
