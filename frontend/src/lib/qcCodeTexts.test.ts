import { describe, expect, it } from "vitest";

import { isKnownQcCode, qcCodeText } from "./qcCodeTexts";

describe("qcCodeText", () => {
  it("geeft gewone taal voor bekende codes", () => {
    expect(qcCodeText("L-GEEN-OPBOUW", "technisch")).toBe(
      "Bouwdeel zonder laagopbouw in het model; U-waarde ontbreekt",
    );
    expect(qcCodeText("M-HOOGTE-OPGETROKKEN", "technisch")).toBe(
      "Ruimte stopt onder het plafond; hoogte is doorgetrokken tot de constructie",
    );
    expect(isKnownQcCode("L-GEEN-OPBOUW")).toBe(true);
  });

  it("onbekende code -> technische message", () => {
    expect(qcCodeText("X-NIEUW", "De technische tekst")).toBe("De technische tekst");
    expect(isKnownQcCode("X-NIEUW")).toBe(false);
  });
});
