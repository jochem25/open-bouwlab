import { describe, expect, it } from "vitest";

import { matchIfcMaterial } from "./ifcMaterialMatcher";

/** Materiaalnamen uit de 2786-engine-uitvoer (geen enkele laag levert lambda mee). */
const CASES: ReadonlyArray<readonly [string, string | null]> = [
  ["hout_vuren_generiek", "hout-naaldhout"],
  ["Holz", "hout-naaldhout"],
  ["n7_isolatie_PIR", "isolatie-kunststof-pir"],
  ["luchtspouw", "spouw-spouw-niet-gevent-rd-0-17"],
  ["f2_cempanel", "plaatmateriaal-cementgebonden-plaat"],
  ["n7_isolatie_resol", "isolatie-kunststof-resolschuim-phenol"],
  ["o1_glas_helder", "glas-glas-float"],
  ["i4_beplating_multiplex", "hout-hardhout-multiplex"],
  ["NLRS_h3_staal_verzinkt", "metaal-staal"],
  // Bewust geen match: kozijnprofiel / raam hebben geen databasemateriaal.
  ["31_CWM_67x139", null],
  ["2. Uitzetraam", null],
];

describe("matchIfcMaterial — 2786-materiaalnamen", () => {
  it.each(CASES)("%s -> %s", (name, id) => {
    expect(matchIfcMaterial(name).material?.id ?? null).toBe(id);
  });

  it("glas is beglazing, nooit Foamglas-isolatie", () => {
    for (const n of ["glas", "Glas helder", "o1_glas_helder", "floatglas", "vensterglas"]) {
      const m = matchIfcMaterial(n).material;
      expect(m?.id, n).toBe("glas-glas-float");
      expect(m?.lambda).toBe(1);
    }
  });

  it("Foamglas / cellulair glas blijven wel naar Foamglas gaan", () => {
    for (const n of ["foamglas", "cellulair glas", "Foamglas T4"]) {
      expect(matchIfcMaterial(n).material?.id, n).toBe("plaatmateriaal-cellulair-glas-foamglas");
    }
  });

  it("luchtspouw krijgt een vaste Rd (geen lambda nodig)", () => {
    const m = matchIfcMaterial("luchtspouw").material;
    expect(m?.rdFixed).toBe(0.17);
  });
});
