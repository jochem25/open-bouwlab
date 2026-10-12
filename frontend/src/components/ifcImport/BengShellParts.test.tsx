/**
 * SSR-tests (omgeving "node") voor de BENG-schil in de IFC-import: schakelaar,
 * legenda, vlakinfo, BENG-status en het keuzevak bij de upload.
 */
import { renderToString as ssr } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { BengImportResult } from "../../lib/bengImport";
import { ifcAnalyseMock } from "../../lib/__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "../../lib/__fixtures__/ifcImportTestData";
import { BengLegend, BengVlakInfoPanel, ViewerModeSwitch } from "./BengShellParts";
import { BengCheckStrip, ImportResultView } from "./ImportResultView";
import { UploadPanel } from "./UploadPanel";

const renderToString = (el: Parameters<typeof ssr>[0]) => ssr(el).replace(/<!-- -->/g, "");

/** Het button-element van een modus uit de schakelaar. */
function modeButton(html: string, mode: string): string {
  const m = html.match(new RegExp(`<button[^>]*data-mode="${mode}"[^>]*>`));
  if (!m) throw new Error(`geen knop ${mode}`);
  return m[0];
}

const compleet: BengImportResult = {
  status: "compleet",
  beng_geometry: { zones: [{ id: "rz-1", a_g_m2: 80, gevels: [] }] },
  ontbrekend: [],
  meldingen: ["Te controleren: IFC-route niet geijkt", "BENG-BELENDING: 2 vlakken"],
  genegeerd: [],
  te_controleren: true,
  source: "ifc-ruimtebalans",
};

describe("ViewerModeSwitch", () => {
  it("markeert de actieve modus", () => {
    const html = renderToString(<ViewerModeSwitch mode="beng" bengAvailable onChange={() => {}} />);
    expect(modeButton(html, "beng")).toContain('aria-checked="true"');
    expect(modeButton(html, "warmteverlies")).toContain('aria-checked="false"');
    expect(html).toContain("Warmteverlies");
    expect(html).toContain("BENG-schil");
  });

  it("zonder schilmodel is BENG-schil uitgeschakeld met uitleg", () => {
    const html = renderToString(
      <ViewerModeSwitch mode="warmteverlies" bengAvailable={false} onChange={() => {}} />,
    );
    const beng = modeButton(html, "beng");
    expect(beng).toContain("disabled");
    expect(beng).toContain("Ook BENG-schil");
    expect(modeButton(html, "warmteverlies")).not.toContain("disabled");
  });
});

describe("BengVlakInfoPanel", () => {
  it("toont gevel_id, zone, grenst_aan, orientatie, helling, A_T, constructie en herkomst", () => {
    const html = renderToString(
      <BengVlakInfoPanel
        name="g-rz-1-03"
        info={{
          gevel_id: "g-rz-1-03",
          zone: "rz-1",
          vlak_type: "gevel",
          grenst_aan: "buitenlucht",
          orientatie: "zuid",
          helling_deg: 90,
          A_T_m2: 28.25,
          constructie_ref: "od-gevel",
          herkomst: "ifc_afgeleid: K.1.2",
          extra: {},
        }}
      />,
    );
    for (const t of ["g-rz-1-03", "rz-1", "Buitenlucht (buitenlucht)", "Z (zuid)", "90 graden", "28,25 m2", "od-gevel", "ifc_afgeleid: K.1.2"]) {
      expect(html).toContain(t);
    }
  });

  it("element zonder Pset: naam", () => {
    expect(renderToString(<BengVlakInfoPanel info={null} name="belending west" />)).toContain("belending west");
  });
});

describe("BengLegend", () => {
  it("toont label, aantal en A_T en meldt afwijkende kleuren", () => {
    const html = renderToString(
      <BengLegend
        entries={[{ key: "buitenlucht:Z", label: "Buitenlucht zuid", color: "#f05433", count: 2, a_t_m2: 30.5 }]}
        mixedColors={["buitenlucht:Z"]}
      />,
    );
    expect(html).toContain("Buitenlucht zuid");
    expect(html).toContain("(2, 30,5 m2)");
    expect(html).toContain("background-color:#f05433");
    expect(html).toContain("Meer kleuren binnen: buitenlucht:Z");
  });
});

describe("BengCheckStrip", () => {
  it("compleet en te controleren: status en meldingen; overnemen loopt via de bestemmingskeuze", () => {
    const html = renderToString(<BengCheckStrip check={{ phase: "ready", result: compleet }} hasShell />);
    expect(html).toContain("compleet (te controleren)");
    expect(html).toContain("1 rekenzone(s)");
    expect(html).not.toContain("BENG-geometrie overnemen");
    expect(html).toContain("BENG-BELENDING: 2 vlakken");
    expect(html).not.toContain("Geen 3D-schilmodel");
  });

  it("onvolledig: geen knop; zonder model een melding", () => {
    const html = renderToString(
      <BengCheckStrip
        check={{ phase: "ready", result: { ...compleet, status: "onvolledig", beng_geometry: undefined, ontbrekend: ["/a", "/b"] } }}
        hasShell={false}
      />,
    );
    expect(html).toContain("Onvolledig: 2 waarde(n)");
    expect(html).not.toContain("BENG-geometrie overnemen");
    expect(html).toContain("Geen 3D-schilmodel");
  });

  it("fout", () => {
    expect(renderToString(<BengCheckStrip check={{ phase: "error", message: "422 pad /x" }} hasShell={false} />)).toContain(
      "BENG-import uit dit IFC mislukt: 422 pad /x",
    );
  });
});

describe("integratie in de importweergave", () => {
  it("ImportResultView toont de BENG-status boven de 3D-controle", () => {
    const html = renderToString(
      <ImportResultView
        response={{ ...ifcAnalyseMock, beng: {}, beng_ifc: { filename: "m_beng.ifc", bytes: 3, base64: "SVNP" } }}
        importResult={makeImportResult(ifcAnalyseMock.thermal)}
        existingRoomCount={0}
        onImport={() => {}}
        bengCheck={{ phase: "ready", result: compleet }}
      />,
    );
    expect(html).toContain('data-testid="beng-check"');
    // Bestemmingskeuze: beide over te nemen, beide standaard aangevinkt.
    expect(html).toMatch(/data-destination="beng" data-state="te_controleren"/);
    expect(html).toMatch(/data-destination="warmteverlies" data-state="(compleet|onvolledig)"/);
    expect(html.indexOf("beng-check")).toBeLessThan(html.indexOf("3D-controle"));
  });

  it("UploadPanel: keuzevak BENG-schil alleen met callback", () => {
    const met = renderToString(
      <UploadPanel busy={false} progress={null} error={null} onFile={() => {}} beng onBengChange={() => {}} />,
    );
    expect(met).toMatch(/data-testid="ifc-beng-toggle"[^>]*checked/);
    const zonder = renderToString(<UploadPanel busy={false} progress={null} error={null} onFile={() => {}} />);
    expect(zonder).not.toContain("ifc-beng-toggle");
  });
});
