import { afterEach, describe, expect, it, vi } from "vitest";

import { analyseIfc, mapIfcAnalyseError, stripAnalyse, type AnalyseProgress } from "./ifcAnalyse";
import { SessionExpiredError } from "./backend";
import { ifcAnalyseMock } from "./__fixtures__/ifcAnalyseMock";

describe("stripAnalyse", () => {
  it("laat report en surfaces_ifc weg, houdt de rest", () => {
    const full = {
      ...ifcAnalyseMock,
      report: { groot: true },
      surfaces_ifc: { filename: "v.ifc", bytes: 1, base64: "AAAA" },
    };
    const stored = stripAnalyse(full);
    expect(stored).not.toHaveProperty("report");
    expect(stored).not.toHaveProperty("surfaces_ifc");
    expect(stored.thermal).toBe(full.thermal);
    expect(stored.qc).toBe(full.qc);
    expect(stored.rooms_extra).toBe(full.rooms_extra);
    expect(stored.thermal_sha256).toBe(full.thermal_sha256);
    expect(stored.engine.commit).toBe(full.engine.commit);
  });
});

describe("mapIfcAnalyseError", () => {
  it("503 BEZET / analyse_busy: wacht een halve minuut", () => {
    for (const error of ["BEZET", "analyse_busy"]) {
      expect(mapIfcAnalyseError(503, { error }).message).toBe(
        "Er loopt al een IFC-analyse. Probeer het over een halve minuut opnieuw.",
      );
    }
  });
  it("413 toont de detail; 422 IFC_ONLEESBAAR en 500 ENGINE_FOUT", () => {
    const e = mapIfcAnalyseError(413, { error: "TE_GROOT", detail: "250 MB > 200 MB" });
    expect(e.message).toContain("100 MB");
    expect(e.message).toContain("250 MB > 200 MB");
    expect(mapIfcAnalyseError(422, { error: "IFC_ONLEESBAAR", detail: "SPF header" }).message).toContain(
      "SPF header",
    );
    expect(mapIfcAnalyseError(500, { error: "ENGINE_FOUT", detail: "exit 2" }).message).toContain(
      "exit 2",
    );
  });
  it("503 analyse_unavailable", () => {
    const e = mapIfcAnalyseError(503, { error: "analyse_unavailable", detail: "engine ontbreekt" });
    expect(e.message).toContain("IFC-analyse niet beschikbaar");
    expect(e.message).toContain("engine ontbreekt");
  });
  it("413, 422, 502, 504", () => {
    expect(mapIfcAnalyseError(413, { error: "file_too_large" }).message).toContain("100 MB");
    expect(mapIfcAnalyseError(422, { error: "analyse_failed", detail: "kapot" }).message).toContain(
      "kapot",
    );
    expect(mapIfcAnalyseError(502, null).message).toContain("ongeldig antwoord");
    expect(mapIfcAnalyseError(504, { error: "analyse_timeout" }).message).toContain("te lang");
  });
  it("onbekende status", () => {
    expect(mapIfcAnalyseError(500, null).message).toContain("HTTP 500");
  });
});

class FakeXhr {
  static last: FakeXhr;
  status = 0;
  responseText = "";
  responseType = "";
  withCredentials = false;
  upload: { onprogress?: (e: unknown) => void; onload?: () => void } = {};
  onload?: () => void;
  onerror?: () => void;
  ontimeout?: () => void;
  contentType = "application/json";
  constructor() {
    FakeXhr.last = this;
  }
  open = vi.fn();
  send = vi.fn();
  getResponseHeader() {
    return this.contentType;
  }
  respond(status: number, body: unknown) {
    this.status = status;
    this.responseText = JSON.stringify(body);
    this.onload?.();
  }
}

describe("analyseIfc", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
  });

  const file = new File(["x"], "a.ifc");

  it("mock-modus levert de mock zonder netwerk", async () => {
    vi.stubEnv("VITE_IFC_ANALYSE_MOCK", "1");
    const phases: string[] = [];
    const r = await analyseIfc(file, (p) => phases.push(p.phase));
    // Byte-kopie van analyse_200_v12_fixture.json (+ source_filename).
    expect(r.engine).toMatchObject({ name: "ifc-ruimtebalans", version: "0.1.0", contract: "1.2" });
    expect(r.source_filename).toBe("voorbeeld.ifc");
    expect(r.qc.counts).toEqual({ blocking: 0, warning: 0, info: 17 });
    expect(r.qc.modelcheck.failed_rules).toEqual(["M-05", "M-07", "M-09"]);
    expect(r.rooms_extra).toHaveLength(3);
    expect(phases).toEqual(["upload", "analyse"]);
  });

  it("meldt upload-voortgang, dan analyseren, en resolvet bij 200", async () => {
    vi.stubGlobal("XMLHttpRequest", FakeXhr);
    const seen: AnalyseProgress[] = [];
    const promise = analyseIfc(file, (p) => seen.push(p));
    const x = FakeXhr.last;
    x.upload.onprogress?.({ lengthComputable: true, loaded: 50, total: 100 });
    x.upload.onload?.();
    x.respond(200, { engine: { name: "e", version: "1" }, thermal: {}, qc: { verdict: "ok", findings: [] } });
    await expect(promise).resolves.toMatchObject({ engine: { name: "e" } });
    expect(seen).toEqual([{ phase: "upload", fraction: 0.5 }, { phase: "analyse" }]);
    expect(x.open).toHaveBeenCalledWith("POST", "/api/v1/ifc/analyse");
    expect(x.withCredentials).toBe(true);
  });

  it("mapt een 503 naar de nette melding", async () => {
    vi.stubGlobal("XMLHttpRequest", FakeXhr);
    const promise = analyseIfc(file);
    FakeXhr.last.respond(503, { error: "analyse_unavailable", detail: "uit" });
    await expect(promise).rejects.toThrow("IFC-analyse niet beschikbaar");
  });

  it("verlopen sessie en netwerkfout", async () => {
    vi.stubGlobal("XMLHttpRequest", FakeXhr);
    const p1 = analyseIfc(file);
    FakeXhr.last.respond(401, {});
    await expect(p1).rejects.toBeInstanceOf(SessionExpiredError);
    const p2 = analyseIfc(file);
    FakeXhr.last.onerror?.();
    await expect(p2).rejects.toThrow("Geen verbinding");
  });
});
