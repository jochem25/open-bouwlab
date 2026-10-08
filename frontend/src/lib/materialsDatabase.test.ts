import { describe, expect, it } from "vitest";
import { MATERIALS_DATABASE } from "./materialsDatabase";

/**
 * Borgt het externe id-contract voor materialen (zie `ifcMaterialMatcher.ts:96`,
 * dat exact op `Material.id` matcht). Revit-modellen leggen dit id vast per
 * materiaal — een stilzwijgende wijziging (bv. door hernoemen, wat vroeger het
 * afgeleide id veranderde) breekt die koppeling zonder foutmelding.
 */
describe("MATERIALS_DATABASE — id-contract", () => {
  it("heeft voor elk ingebouwd materiaal een expliciet, niet-leeg id", () => {
    for (const material of MATERIALS_DATABASE) {
      expect(material.isBuiltIn).toBe(true);
      expect(typeof material.id).toBe("string");
      expect(material.id.length).toBeGreaterThan(0);
    }
  });

  it("heeft unieke id's over de hele database", () => {
    const ids = MATERIALS_DATABASE.map((m) => m.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("houdt bekende id's stabiel (regressie tegen per-ongeluk hernoemen)", () => {
    const sample: ReadonlyArray<readonly [string, string]> = [
      ["isolatie-kunststof-pir", "PIR"],
      ["isolatie-kunststof-eps", "EPS"],
      ["isolatie-kunststof-xps", "XPS"],
      ["hout-naaldhout", "Naaldhout"],
      ["metselwerk-kalkzandsteen", "Kalkzandsteen"],
      ["beton-beton-gewapend", "Beton gewapend"],
      ["metselwerk-baksteen-700-kg-m", "Baksteen 700 kg/m³"],
    ];
    for (const [id, expectedName] of sample) {
      const material = MATERIALS_DATABASE.find((m) => m.id === id);
      expect(material, `id "${id}" niet gevonden in database`).toBeDefined();
      expect(material?.name).toBe(expectedName);
    }
  });

  it("aantal materialen in de database blijft 158 (wijzig bewust, niet per ongeluk)", () => {
    expect(MATERIALS_DATABASE.length).toBe(158);
  });
});
