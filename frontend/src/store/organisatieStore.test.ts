import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { genereerConstructieRapport } from "../lib/constructieClient";
import { generateReportDirect } from "../lib/reportClient";
import { gekozenHuisstijl, organisatieNaam, useOrganisatieStore } from "./organisatieStore";

const KBA = {
  id: "kba",
  naam: "Testbureau",
  huisstijlen: ["kba", "openaec_foundation"],
  standaard_huisstijl: "kba",
};

function meAntwoord(extra: Record<string, unknown>): Response {
  return new Response(JSON.stringify({ id: "u", email: "", name: "", ...extra }), { status: 200 });
}

function pdfAntwoord(): Response {
  return new Response("%PDF", { status: 200, headers: { "content-type": "application/pdf" } });
}

function verzondenBody(fetchMock: ReturnType<typeof vi.fn>): Record<string, unknown> {
  const init = fetchMock.mock.calls.at(-1)?.[1] as RequestInit;
  return JSON.parse(String(init.body)) as Record<string, unknown>;
}

beforeEach(() => {
  useOrganisatieStore.setState({
    status: "idle",
    ingelogd: false,
    organisatie: null,
    fout: null,
    gekozenHuisstijl: null,
  });
});

afterEach(() => vi.unstubAllGlobals());

describe("organisatieStore", () => {
  it("laadt organisatie uit /me en geeft de naam voor rapporten", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(meAntwoord({ organisatie: KBA })));
    await useOrganisatieStore.getState().laad();
    const s = useOrganisatieStore.getState();
    expect(s.status).toBe("klaar");
    expect(s.ingelogd).toBe(true);
    expect(s.organisatie?.id).toBe("kba");
    expect(organisatieNaam()).toBe("Testbureau");
  });

  it("bewaart de fout bij een ongeldige org-groep; naam blijft leeg", async () => {
    const fout = { code: "meerdere_organisaties", detail: "meer dan een organisatie" };
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(meAntwoord({ organisatie: null, organisatie_fout: fout })),
    );
    await useOrganisatieStore.getState().laad();
    expect(useOrganisatieStore.getState().fout).toEqual(fout);
    expect(organisatieNaam()).toBe("");
  });

  it("niet ingelogd (401): geen organisatie, geen fout", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("", { status: 401 })));
    await useOrganisatieStore.getState().laad();
    const s = useOrganisatieStore.getState();
    expect(s.ingelogd).toBe(false);
    expect(s.organisatie).toBeNull();
  });

  it("accepteert alleen toegestane huisstijlen; standaard gaat niet expliciet mee", () => {
    useOrganisatieStore.setState({ status: "klaar", ingelogd: true, organisatie: KBA });
    const { kiesHuisstijl } = useOrganisatieStore.getState();
    expect(gekozenHuisstijl()).toBeUndefined();
    kiesHuisstijl("openaec_foundation");
    expect(gekozenHuisstijl()).toBe("openaec_foundation");
    kiesHuisstijl("kba");
    expect(gekozenHuisstijl()).toBeUndefined();
    kiesHuisstijl("3bm");
    expect(useOrganisatieStore.getState().gekozenHuisstijl).toBeNull();
  });
});

describe("huisstijl en organisatienaam in rapporten", () => {
  it("rapportclient stuurt de gekozen huisstijl als brand mee", async () => {
    useOrganisatieStore.setState({ status: "klaar", ingelogd: true, organisatie: KBA });
    const fetchMock = vi.fn().mockImplementation(() => Promise.resolve(pdfAntwoord()));
    vi.stubGlobal("fetch", fetchMock);

    await generateReportDirect({ project: "p" });
    expect(verzondenBody(fetchMock).brand).toBeUndefined();

    useOrganisatieStore.getState().kiesHuisstijl("openaec_foundation");
    await generateReportDirect({ project: "p" });
    expect(verzondenBody(fetchMock).brand).toBe("openaec_foundation");
  });

  it("toont de leesbare 403-tekst van de backend", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify({ error: "geen_organisatie", detail: "Je account is niet aan een organisatie gekoppeld" }),
          { status: 403 },
        ),
      ),
    );
    await expect(generateReportDirect({ project: "p" })).rejects.toThrow(
      "Je account is niet aan een organisatie gekoppeld",
    );
  });

  it("constructierapport stuurt de gekozen huisstijl mee", async () => {
    useOrganisatieStore.setState({ status: "klaar", ingelogd: true, organisatie: KBA });
    useOrganisatieStore.getState().kiesHuisstijl("openaec_foundation");
    const fetchMock = vi.fn().mockResolvedValue(pdfAntwoord());
    vi.stubGlobal("fetch", fetchMock);
    await genereerConstructieRapport("staal", {} as never);
    expect(verzondenBody(fetchMock).huisstijl).toBe("openaec_foundation");
  });
});
