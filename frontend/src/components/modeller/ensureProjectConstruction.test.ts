import { beforeEach, describe, expect, it } from "vitest";

import { sameLayers, useModellerStore } from "./modellerStore";

const base = {
  name: "23_Holz_Holz",
  category: "vloeren_plafonds" as const,
  materialType: "masonry" as const,
  verticalPosition: "floor" as const,
};

describe("ensureProjectConstruction (naam-pad)", () => {
  beforeEach(() => {
    useModellerStore.setState({ projectConstructions: [] });
  });

  it("hergebruikt een constructie met dezelfde naam en dezelfde lagen", () => {
    const { ensureProjectConstruction } = useModellerStore.getState();
    const layers = [{ materialId: "hout-naaldhout", thickness: 120 }];
    const a = ensureProjectConstruction({ ...base, layers });
    const b = ensureProjectConstruction({ ...base, layers: layers.map((l) => ({ ...l })) });
    expect(b).toBe(a);
    expect(useModellerStore.getState().projectConstructions).toHaveLength(1);
  });

  it("maakt een nieuwe constructie als de lagen verschillen (herimport na matcher-fix)", () => {
    const { ensureProjectConstruction } = useModellerStore.getState();
    const oud = ensureProjectConstruction({ ...base, layers: [{ materialId: "holz", thickness: 120 }] });
    const nieuw = ensureProjectConstruction({
      ...base,
      layers: [{ materialId: "hout-naaldhout", thickness: 120 }],
    });
    expect(nieuw).not.toBe(oud);
    const pc = useModellerStore.getState().projectConstructions.find((c) => c.id === nieuw);
    expect(pc?.layers[0].materialId).toBe("hout-naaldhout");
  });
});

describe("sameLayers", () => {
  it("telt volgorde, dikte en lambda-override mee", () => {
    const a = [{ materialId: "x", thickness: 10 }, { materialId: "y", thickness: 20 }];
    expect(sameLayers(a, [...a].reverse())).toBe(false);
    expect(sameLayers(a, [{ materialId: "x", thickness: 10 }, { materialId: "y", thickness: 21 }])).toBe(false);
    expect(sameLayers([{ materialId: "x", thickness: 10, lambdaOverride: 0.1 }], [{ materialId: "x", thickness: 10 }])).toBe(false);
    expect(sameLayers(a, a.map((l) => ({ ...l })))).toBe(true);
  });
});
