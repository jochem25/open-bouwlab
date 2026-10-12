import { afterEach, describe, expect, it, vi } from "vitest";

import {
  BengImportError,
  bengImportSummary,
  importBengDocument,
  type BengImportResult,
} from "./bengImport";
import type { BengGeometry } from "../types/bengGeometry";

function mockFetch(status: number, body: unknown, statusText = ""): ReturnType<typeof vi.fn> {
  const fn = vi.fn().mockResolvedValue({
    ok: status >= 200 && status < 300,
    status,
    statusText,
    json: async () => {
      if (body === undefined) throw new Error("geen JSON");
      return body;
    },
  });
  vi.stubGlobal("fetch", fn);
  return fn;
}

const GEOMETRIE = {
  zones: [{ gevels: [{}, {}] }, { gevels: [{}] }],
} as unknown as BengGeometry;

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("importBengDocument", () => {
  it("200 compleet: geeft het resultaat terug en post de JSON-tekst", async () => {
    const body: BengImportResult = {
      status: "compleet",
      beng_geometry: GEOMETRIE,
      ontbrekend: [],
      meldingen: ["Te controleren: x"],
      genegeerd: [],
      te_controleren: true,
      source: "test",
    };
    const fn = mockFetch(200, body);
    const r = await importBengDocument('{"a":1}');
    expect(r.status).toBe("compleet");
    expect(r.beng_geometry).toBe(GEOMETRIE);
    const [url, init] = fn.mock.calls[0] as [string, RequestInit];
    expect(url).toMatch(/\/import\/beng$/);
    expect(init.method).toBe("POST");
    expect(init.body).toBe('{"a":1}');
  });

  it("serialiseert een object naar JSON", async () => {
    const fn = mockFetch(200, { status: "onvolledig", ontbrekend: [], meldingen: [] });
    await importBengDocument({ a: 1 });
    expect((fn.mock.calls[0][1] as RequestInit).body).toBe('{"a":1}');
  });

  it("200 onvolledig: geeft ontbrekende paden terug", async () => {
    mockFetch(200, {
      status: "onvolledig",
      ontbrekend: ["/beng_geometry/opaque_defs/2/thermal"],
      meldingen: [],
      genegeerd: [],
      te_controleren: true,
      source: "test",
    });
    const r = await importBengDocument({});
    expect(r.status).toBe("onvolledig");
    expect(r.beng_geometry).toBeUndefined();
    expect(r.ontbrekend).toEqual(["/beng_geometry/opaque_defs/2/thermal"]);
  });

  it("422: detail komt in de melding", async () => {
    mockFetch(422, { error: "beng_import_error", detail: "/beng_geometry/zones/0: onbekend veld" });
    const err = await importBengDocument({}).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(BengImportError);
    expect((err as BengImportError).status).toBe(422);
    expect((err as BengImportError).message).toContain("/beng_geometry/zones/0: onbekend veld");
  });

  it("400: geen geldig JSON-bestand", async () => {
    mockFetch(400, { error: "json_parse_error", detail: "EOF" });
    const err = (await importBengDocument("{").catch((e: unknown) => e)) as BengImportError;
    expect(err).toBeInstanceOf(BengImportError);
    expect(err.status).toBe(400);
    expect(err.message).toContain("Geen geldig JSON-bestand");
    expect(err.message).toContain("EOF");
  });

  it("onverwacht antwoord (200 zonder status): BengImportError 502", async () => {
    mockFetch(200, { iets: "anders" });
    const err = (await importBengDocument({}).catch((e: unknown) => e)) as BengImportError;
    expect(err).toBeInstanceOf(BengImportError);
    expect(err.status).toBe(502);
    expect(err.message).toContain("Onverwacht antwoord");
  });

  it("500 zonder JSON-body: generieke fout met HTTP-status", async () => {
    mockFetch(500, undefined, "Internal Server Error");
    const err = (await importBengDocument({}).catch((e: unknown) => e)) as BengImportError;
    expect(err.status).toBe(500);
    expect(err.message).toContain("HTTP 500");
  });
});

describe("bengImportSummary", () => {
  const basis = { ontbrekend: [], meldingen: [], genegeerd: [], source: "t" };

  it("compleet: zones en vlakken, met te controleren", () => {
    const s = bengImportSummary({
      ...basis,
      status: "compleet",
      beng_geometry: GEOMETRIE,
      te_controleren: true,
    });
    expect(s).toBe("2 rekenzone(s), 3 vlakken - te controleren.");
  });

  it("compleet zonder te_controleren", () => {
    const s = bengImportSummary({
      ...basis,
      status: "compleet",
      beng_geometry: GEOMETRIE,
      te_controleren: false,
    });
    expect(s).toBe("2 rekenzone(s), 3 vlakken.");
  });

  it("onvolledig: aantal ontbrekende waarden", () => {
    const s = bengImportSummary({
      ...basis,
      status: "onvolledig",
      ontbrekend: ["/a", "/b"],
      te_controleren: true,
    });
    expect(s).toBe("Onvolledig: 2 waarde(n) ontbreken (Rc/U of U_w).");
  });
});
