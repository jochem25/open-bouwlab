import { describe, expect, it } from "vitest";

import { formatGetal, formatUc, rondHalfOp } from "./constructieFormat";

describe("rondHalfOp / formatUc", () => {
  it("rondt half naar boven op de decimale waarde", () => {
    expect(formatUc(0.845)).toBe("0,85");
    expect(formatUc(0.8449)).toBe("0,84");
    expect(rondHalfOp(0.845, 2)).toBe(0.85);
    expect(rondHalfOp(1.005, 2)).toBe(1.01);
  });

  it("is symmetrisch voor negatieve waarden en gebruikt een komma", () => {
    expect(rondHalfOp(-0.845, 2)).toBe(-0.85);
    expect(formatGetal(12.5, 1)).toBe("12,5");
    expect(formatGetal(3, 2)).toBe("3,00");
  });

  it("geeft een streepje voor ontbrekende of ongeldige waarden", () => {
    expect(formatUc(null)).toBe("-");
    expect(formatUc(undefined)).toBe("-");
    expect(formatUc(Number.NaN)).toBe("-");
  });
});
