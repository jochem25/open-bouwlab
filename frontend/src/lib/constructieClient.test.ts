import { afterEach, describe, expect, it, vi } from "vitest";

import {
  berekenConstructie,
  ConstructieFout,
  genereerConstructieRapport,
  rapportBestandsnaam,
} from "./constructieClient";
import { bouwInvoer, STANDAARD_FORMULIER } from "../components/constructie/formulier";

afterEach(() => vi.unstubAllGlobals());

describe("constructieClient", () => {
  it("post naar het materiaal-endpoint met credentials", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ advies: null }), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    const invoer = bouwInvoer(STANDAARD_FORMULIER, "staal");
    expect(invoer).not.toBeNull();
    await berekenConstructie("staal", invoer!);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("/api/v1/constructie/staal");
    expect(init.method).toBe("POST");
    expect(init.credentials).toBe("include");
  });

  it("geeft de error-tekst en status door bij 422 en 403", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response(JSON.stringify({ error: "overspanning buiten bereik" }), { status: 422 })),
    );
    const invoer = bouwInvoer(STANDAARD_FORMULIER, "beton")!;
    await expect(berekenConstructie("beton", invoer)).rejects.toMatchObject({
      message: "overspanning buiten bereik",
      status: 422,
    });
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response(JSON.stringify({ error: "geen toegang" }), { status: 403 })),
    );
    await expect(genereerConstructieRapport("beton", invoer)).rejects.toBeInstanceOf(ConstructieFout);
  });

  it("maakt een veilige rapportbestandsnaam", () => {
    expect(rapportBestandsnaam("staal", "IPE 200")).toBe("constructie-staal-ipe-200.pdf");
    expect(rapportBestandsnaam("beton", "300 x 450")).toBe("constructie-beton-300-x-450.pdf");
    expect(rapportBestandsnaam("staal", null)).toBe("constructie-staal-geen-advies.pdf");
  });

  it("laat dak-velden weg bij een vloer en neemt ze mee bij een dak", () => {
    const vloer = bouwInvoer(STANDAARD_FORMULIER, "staal")!;
    expect(vloer.algemeen.dak_hellingshoek_graden).toBeUndefined();
    const dak = bouwInvoer({ ...STANDAARD_FORMULIER, toepassing: "dak" }, "staal")!;
    expect(dak.algemeen.dak_afschot_procent).toBe(1.6);
    expect(bouwInvoer({ ...STANDAARD_FORMULIER, overspanning_m: null }, "staal")).toBeNull();
  });
});
