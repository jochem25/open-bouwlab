import { describe, expect, it } from "vitest";

import { matchIfcMaterial } from "./ifcMaterialMatcher";

/**
 * Materiaalnamen zoals een IFC-engine ze levert: codeprefix, Duitse of
 * merknamen, geen lambda. Synthetisch, naar het patroon van echte exports.
 */
const CASES: ReadonlyArray<readonly [string, string | null]> = [
  ["a1_hout_vuren", "hout-naaldhout"],
  ["Holz", "hout-naaldhout"],
  ["b2_isolatie_PIR", "isolatie-kunststof-pir"],
  ["luchtspouw", "spouw-spouw-niet-gevent-rd-0-17"],
  ["c3_cempanel", "plaatmateriaal-cementgebonden-plaat"],
  ["b2_isolatie_resol", "isolatie-kunststof-resolschuim-phenol"],
  ["d4_glas_helder", "glas-glas-float"],
  ["e5_beplating_multiplex", "hout-hardhout-multiplex"],
  ["XX_f6_staal_verzinkt", "metaal-staal"],
  // Bewust geen match: kozijnprofiel / raam hebben geen databasemateriaal.
  ["10_kozijnprofiel_67x139", null],
  ["3. Uitzetraam", null],
];

describe("matchIfcMaterial — materiaalnamen uit IFC-exports", () => {
  it.each(CASES)("%s -> %s", (name, id) => {
    expect(matchIfcMaterial(name).material?.id ?? null).toBe(id);
  });

  it("glas is beglazing, nooit Foamglas-isolatie", () => {
    for (const n of ["glas", "Glas helder", "d4_glas_helder", "floatglas", "vensterglas"]) {
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
