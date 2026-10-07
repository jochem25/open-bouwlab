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
    expect(rapportBestandsnaam("hout", "71 x 196")).toBe("constructie-hout-71-x-196.pdf");
    expect(rapportBestandsnaam("staal", null)).toBe("constructie-staal-geen-advies.pdf");
  });

  it("laat dak-velden weg bij een vloer en neemt ze mee bij een dak", () => {
    const vloer = bouwInvoer(STANDAARD_FORMULIER, "staal")!;
    expect(vloer.algemeen.dak_hellingshoek_graden).toBeUndefined();
    const dak = bouwInvoer({ ...STANDAARD_FORMULIER, toepassing: "dak" }, "staal")!;
    expect(dak.algemeen.dak_afschot_procent).toBe(1.6);
    expect(bouwInvoer({ ...STANDAARD_FORMULIER, overspanning_m: null }, "staal")).toBeNull();
  });

  it("bouwt hout-invoer: balklaag met getagd element en vloerplaat", () => {
    const f = {
      ...STANDAARD_FORMULIER,
      vloerplaat_dikte_mm: 22,
      vloerplaat_e_n_mm2: 3000,
      vloerbreedte_m: 4,
    };
    const invoer = bouwInvoer(f, "hout")!;
    expect(invoer.element).toEqual({ type: "balklaag", hoh_mm: 400 });
    expect(invoer.sterkteklasse).toBe("C24");
    expect(invoer.breedte_mm).toBe(71);
    expect(invoer.klimaatklasse).toBe(1);
    expect(invoer.drukrand_gesteund).toBe(true);
    expect(invoer.vloerplaat).toEqual({ dikte_mm: 22, e_mean_n_mm2: 3000 });
    expect(invoer.vloerbreedte_m).toBe(4);
    expect(invoer.hoogte_mm).toBeUndefined();
  });

  it("laat vloerplaat weg bij een enkele balk en gebruikt de belastingbreedte", () => {
    const f = {
      ...STANDAARD_FORMULIER,
      hout_element: "balk" as const,
      belastingbreedte_m: 2.5,
      vloerplaat_dikte_mm: 22,
      vloerplaat_e_n_mm2: 3000,
      vloerbreedte_m: 4,
    };
    const invoer = bouwInvoer(f, "hout")!;
    expect(invoer.element).toEqual({ type: "balk", belastingbreedte_m: 2.5 });
    expect(invoer.vloerplaat).toBeUndefined();
    expect(invoer.vloerbreedte_m).toBeUndefined();
  });

  it("hout: vaste hoogte meegeven; halve vloerplaat of lege breedte is onvolledig", () => {
    const vast = bouwInvoer({ ...STANDAARD_FORMULIER, hoogte_automatisch: false, hoogte_mm: 220 }, "hout")!;
    expect(vast.hoogte_mm).toBe(220);
    expect(bouwInvoer({ ...STANDAARD_FORMULIER, hoogte_automatisch: false, hoogte_mm: null }, "hout")).toBeNull();
    expect(bouwInvoer({ ...STANDAARD_FORMULIER, vloerplaat_dikte_mm: 22 }, "hout")).toBeNull();
    expect(bouwInvoer({ ...STANDAARD_FORMULIER, hout_breedte_mm: null }, "hout")).toBeNull();
    expect(bouwInvoer({ ...STANDAARD_FORMULIER, hout_element: "balk", belastingbreedte_m: null }, "hout")).toBeNull();
  });
});
