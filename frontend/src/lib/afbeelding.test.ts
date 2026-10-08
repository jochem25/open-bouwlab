import { describe, expect, it } from "vitest";

import { MAX_AFBEELDING_BYTES, controleerAfbeelding } from "./afbeelding";

describe("controleerAfbeelding", () => {
  it("accepteert PNG en JPEG tot en met 2 MB", () => {
    expect(controleerAfbeelding({ type: "image/png", size: 10 })).toBeNull();
    expect(controleerAfbeelding({ type: "image/jpeg", size: MAX_AFBEELDING_BYTES })).toBeNull();
  });

  it("weigert andere typen en te grote bestanden", () => {
    expect(controleerAfbeelding({ type: "image/gif", size: 10 })).toBe("type");
    expect(controleerAfbeelding({ type: "image/png", size: MAX_AFBEELDING_BYTES + 1 })).toBe("te_groot");
  });
});
