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

  it("toont bij een afgekeurd rapport de detail-tekst, niet de foutcode", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify({
            error: "report_rejected",
            detail: "Rapport afgekeurd door de rapportengine: verplichte afbeelding ontbreekt",
          }),
          { status: 422 },
        ),
      ),
    );
    const invoer = bouwInvoer(STANDAARD_FORMULIER, "staal")!;
    await expect(genereerConstructieRapport("staal", invoer)).rejects.toMatchObject({
      message: "Rapport afgekeurd door de rapportengine: verplichte afbeelding ontbreekt",
      status: 422,
    });
  });

  it("stuurt een gekozen coverfoto mee en laat hem anders weg", async () => {
    const fetchMock = vi.fn().mockImplementation(() => Promise.resolve(new Response("%PDF", { status: 200 })));
    vi.stubGlobal("fetch", fetchMock);
    const invoer = bouwInvoer(STANDAARD_FORMULIER, "staal")!;
    const coverfoto = { data: "QUJD", media_type: "image/png" as const, filename: "foto.png" };
    await genereerConstructieRapport("staal", invoer, { coverfoto });
    const met = JSON.parse((fetchMock.mock.calls[0] as [string, RequestInit])[1].body as string);
    expect(met.project.coverfoto).toEqual(coverfoto);
    await genereerConstructieRapport("staal", invoer);
    const zonder = JSON.parse((fetchMock.mock.calls[1] as [string, RequestInit])[1].body as string);
    expect(zonder.project).toBeUndefined();
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

  it("bouwt hout-invoer: balklaag met getagd element en lagen; lege E telt niet mee", () => {
    const f = {
      ...STANDAARD_FORMULIER,
      vloerlagen: [
        { sleutel: "a", soort: "vloerplaat" as const, dikte_mm: 18, e_n_mm2: 4000 },
        { sleutel: "b", soort: "plafond" as const, dikte_mm: 12.5, e_n_mm2: 2000 },
        { sleutel: "c", soort: "dekvloer" as const, dikte_mm: 40, e_n_mm2: null },
      ],
      vloerbreedte_m: 4,
      dwars_ei_knm2: 1429.8,
    };
    const invoer = bouwInvoer(f, "hout")!;
    expect(invoer.element).toEqual({ type: "balklaag", hoh_mm: 400 });
    expect(invoer.sterkteklasse).toBe("C24");
    expect(invoer.breedte_mm).toBe(71);
    expect(invoer.klimaatklasse).toBe(1);
    expect(invoer.drukrand_gesteund).toBe(true);
    expect(invoer.vloerlagen).toEqual([
      { soort: "vloerplaat", dikte_mm: 18, e_mean_n_mm2: 4000 },
      { soort: "plafond", dikte_mm: 12.5, e_mean_n_mm2: 2000 },
    ]);
    expect(invoer.vloerbreedte_m).toBe(4);
    expect(invoer.dwarsverbinding?.ei_nm2).toBeCloseTo(1429800, 3);
    expect(invoer.trillingstoets).toBe(true);
    expect(invoer.hoogte_mm).toBeUndefined();
  });

  it("hout: schakelaar uit geeft trillingstoets false; zonder lagen geen vloerlagen-veld", () => {
    const invoer = bouwInvoer({ ...STANDAARD_FORMULIER, trillingstoets: false }, "hout")!;
    expect(invoer.trillingstoets).toBe(false);
    expect(invoer.vloerlagen).toBeUndefined();
    expect(invoer.dwarsverbinding).toBeUndefined();
  });

  it("laat lagen weg bij een enkele balk en gebruikt de belastingbreedte", () => {
    const f = {
      ...STANDAARD_FORMULIER,
      hout_element: "balk" as const,
      belastingbreedte_m: 2.5,
      vloerlagen: [{ sleutel: "a", soort: "vloerplaat" as const, dikte_mm: 22, e_n_mm2: 3000 }],
      vloerbreedte_m: 4,
    };
    const invoer = bouwInvoer(f, "hout")!;
    expect(invoer.element).toEqual({ type: "balk", belastingbreedte_m: 2.5 });
    expect(invoer.vloerlagen).toBeUndefined();
    expect(invoer.vloerbreedte_m).toBeUndefined();
    expect(invoer.trillingstoets).toBeUndefined();
  });

  it("hout: vaste hoogte meegeven; E zonder dikte of lege breedte is onvolledig", () => {
    const vast = bouwInvoer({ ...STANDAARD_FORMULIER, hoogte_automatisch: false, hoogte_mm: 220 }, "hout")!;
    expect(vast.hoogte_mm).toBe(220);
    expect(bouwInvoer({ ...STANDAARD_FORMULIER, hoogte_automatisch: false, hoogte_mm: null }, "hout")).toBeNull();
    expect(bouwInvoer(
        { ...STANDAARD_FORMULIER, vloerlagen: [{ sleutel: "a", soort: "vloerplaat", dikte_mm: null, e_n_mm2: 4000 }] },
        "hout",
      )).toBeNull();
    expect(bouwInvoer({ ...STANDAARD_FORMULIER, hout_breedte_mm: null }, "hout")).toBeNull();
    expect(bouwInvoer({ ...STANDAARD_FORMULIER, hout_element: "balk", belastingbreedte_m: null }, "hout")).toBeNull();
  });
});
