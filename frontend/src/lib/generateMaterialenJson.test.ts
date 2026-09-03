import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { parseRawMaterials, buildMaterialenDatabaseJson } from "../../scripts/generate-materialen-json.mjs";
import { MATERIALS_DATABASE } from "./materialsDatabase";

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Borgt dat de generator (`scripts/generate-materialen-json.mjs`) die het
 * pyRevit-bestand `materialen_database.json` produceert, in lockstep blijft
 * met `MATERIALS_DATABASE` — de enige bron sinds het besluit van 2026-09-03
 * (zie TODO.md). Geen cross-repo vergelijking hier (CI heeft geen toegang
 * tot de pyrevit-repo); dat blijft een handmatige regeneratiestap.
 */
describe("generate-materialen-json — generator-output", () => {
  const sourcePath = resolve(__dirname, "materialsDatabase.ts");
  const source = readFileSync(sourcePath, "utf8");
  const rawMaterials = parseRawMaterials(source);

  it("levert precies 157 materialen op", () => {
    expect(rawMaterials.length).toBe(157);
  });

  it("heeft voor elk materiaal een niet-leeg id", () => {
    for (const raw of rawMaterials) {
      expect(typeof raw.id).toBe("string");
      expect((raw.id as string).length).toBeGreaterThan(0);
    }
  });

  it("heeft unieke id's", () => {
    const ids = rawMaterials.map((raw) => raw.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("id's zijn exact gelijk aan MATERIALS_DATABASE (geen ontbrekende of extra materialen)", () => {
    const generatedIds = new Set(rawMaterials.map((raw) => raw.id));
    const databaseIds = new Set(MATERIALS_DATABASE.map((m) => m.id));
    expect(generatedIds).toEqual(databaseIds);
  });

  it("produceert geldige, ASCII-only JSON met een _gegenereerd-markering", () => {
    const json = buildMaterialenDatabaseJson(rawMaterials, {
      sourceFile: "open-bouwlab frontend/src/lib/materialsDatabase.ts",
      npmScript: "npm run generate:materialen -- --out <pad>",
    });

    // ASCII-only: geen enkel codepoint > 0x7E ongeescaped in de output.
    // eslint-disable-next-line no-control-regex
    expect(/^[\x00-\x7e\n]*$/.test(json)).toBe(true);

    const parsed = JSON.parse(json);
    expect(parsed.materialen).toHaveLength(157);
    expect(parsed.versie).toBe("3.0");
    expect(typeof parsed._gegenereerd).toBe("string");
    expect(parsed._gegenereerd).toContain("materialsDatabase.ts");

    const parsedIds = parsed.materialen.map((m: { id: string }) => m.id);
    expect(new Set(parsedIds).size).toBe(parsedIds.length);
  });

  it("bevat geen generieke CLT-entry meer (7 specifiekere TS-varianten vervangen 'm)", () => {
    const cltNames = rawMaterials
      .filter((raw) => (raw.naam as string).toLowerCase().includes("clt"))
      .map((raw) => raw.naam as string);
    expect(cltNames).not.toContain("CLT (Cross Laminated Timber)");
    expect(cltNames.length).toBe(7);
  });
});
