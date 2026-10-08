import { afterEach, describe, expect, it, vi } from "vitest";

import { analyseIfc, mapIfcAnalyseError, type AnalyseProgress } from "./ifcAnalyse";
import { SessionExpiredError } from "./backend";

describe("mapIfcAnalyseError", () => {
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
    expect(r.engine.version).toBe("0.0.0-mock");
    expect(r.qc.findings.filter((f) => f.severity === "blocking")).toHaveLength(1);
    expect(r.qc.findings.filter((f) => f.severity === "warning")).toHaveLength(2);
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
