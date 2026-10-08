/**
 * Render-tests (SSR, omgeving is "node") voor de IFC-import-resultaatweergave
 * en de bevestigingsmodal, met de mock-response.
 */
import { renderToString as ssr } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { MemoryRouter } from "react-router-dom";

import { MissingUBannerView } from "../components/ifcImport/MissingUBanner";
import { ChecklistView } from "../components/ifcImport/IfcImportChecklistPanel";
import { ConfirmModal } from "../components/ifcImport/ConfirmModal";
import { ImportResultView } from "../components/ifcImport/ImportResultView";
import { progressLabel, validateIfcFile } from "../components/ifcImport/UploadPanel";
import { ifcAnalyseMock } from "../lib/__fixtures__/ifcAnalyseMock";
import { ifcAnalyseBlocking, finding, withFindings } from "../lib/__fixtures__/ifcAnalyseVariants";
import { makeImportResult } from "../lib/__fixtures__/ifcImportTestData";
import { groupFindings } from "../components/ifcImport/ImportResultView";

/** SSR voegt `<!-- -->` tussen tekstdelen in; haal die weg voor tekstvergelijking. */
const renderToString = (el: Parameters<typeof ssr>[0]) => ssr(el).replace(/<!-- -->/g, "");

function render(existing = 0, response = ifcAnalyseBlocking) {
  return renderToString(
    <ImportResultView
      response={response}
      importResult={makeImportResult(ifcAnalyseMock.thermal)}
      existingRoomCount={existing}
      onImport={() => {}}
    />,
  );
}

describe("ImportResultView", () => {
  it("toont samenvatting met aantal echte ruimten, engine en oordeel", () => {
    const html = render();
    expect(html).toContain("voorbeeld.ifc");
    expect(html).toContain("ifc-ruimtebalans 0.1.0");
    expect(html).toMatch(/data-testid="stat-rooms"[^>]*>2</);
    expect(html).toContain("Bevindingen: blokkerend");
    expect(html).toContain("Modelcheck: geschikt met meldingen (niet gehaald: M-05, M-07, M-09)");
    expect(html).toContain("Blokkerend 1 · Waarschuwingen 2 · Info 17");
    expect(html).toContain("Blokkerend (1)");
    expect(html).toContain("Waarschuwingen (2)");
  });

  it("groepeert per code met aantal, modelleur-badge en gewone taal; info staat dicht", () => {
    const html = render();
    expect(html).toContain("L-GEEN-OPBOUW");
    expect(html).toContain("×3");
    expect(html).toContain("modelleur");
    expect(html).toContain("Bouwdeel zonder laagopbouw in het model; U-waarde ontbreekt");
    // onbekende code -> technische message
    expect(html).toContain("Onbekende melding.");
    // ruimtekoppeling via room_id
    expect(html).toContain(">laag</button>");
    // info: alleen kop + "Toon alle", geen rijen
    expect(html).toContain("Info (17)");
    expect(html).toContain("Toon alle 17");
    expect(html).not.toContain("C-VIA-OPENING");
  });

  it("Nr-kolom uit rooms_extra[].number, anders '-'", () => {
    const withNumber = {
      ...ifcAnalyseBlocking,
      rooms_extra: ifcAnalyseBlocking.rooms_extra.map((r, i) =>
        i === 0 ? { ...r, number: "1.07" } : r,
      ),
    };
    const html = render(0, withNumber);
    expect(html).toContain('<td class="pr-3 text-on-surface-muted">1.07</td>');
    expect(html).toContain('<td class="pr-3 text-on-surface-muted">-</td>');
  });

  it("zonder blokkerende bevindingen (contract-fixture) staan beide ruimten aan", () => {
    const html = render(0, ifcAnalyseMock);
    expect(html).toContain("2 van 2 ruimten goedgekeurd");
    expect(html).toContain("Bevindingen: in orde");
  });

  it("vinkt standaard alles aan behalve de ruimte met blokkerende bevinding", () => {
    const html = render();
    const boxes = [...html.matchAll(/<input[^>]*type="checkbox"[^>]*>/g)].map((m) => m[0]);
    const byLabel = (label: string) => boxes.find((b) => b.includes(label))!;
    expect(byLabel("Goedkeuren hoog")).toContain('checked=""');
    expect(byLabel("Goedkeuren laag")).not.toContain('checked=""');
    expect(byLabel("Alles goedkeuren")).not.toContain('checked=""');
    expect(html).toContain("1 van 2 ruimten goedgekeurd");
  });

  it("toont de constructielijst uit de catalogus", () => {
    const html = render();
    expect(html).toContain("Wand test");
    expect(html).toContain("baksteen 100 mm");
  });
});

describe("ConfirmModal", () => {
  const base = { blocking: [], orphanedSurfaces: 0, replacesExisting: false, approvedCount: 1 };
  it("toont blokkerende bevindingen en wees-vlakken", () => {
    const html = renderToString(
      <ConfirmModal
        open
        notices={{ ...base, blocking: ["Schil niet gesloten"], orphanedSurfaces: 3 }}
        onConfirm={() => {}}
        onCancel={() => {}}
      />,
    );
    expect(html).toContain("Schil niet gesloten");
    expect(html).toContain("3 vlakken grenzen aan niet-overgenomen ruimten en tellen als 0 W/K");
    expect(html).toContain("Toch overnemen");
  });
  it("biedt Vervangen als het project al ruimten heeft", () => {
    const html = renderToString(
      <ConfirmModal
        open
        notices={{ ...base, replacesExisting: true }}
        onConfirm={() => {}}
        onCancel={() => {}}
      />,
    );
    expect(html).toContain("Vervangen");
    expect(html).toContain("Annuleren");
  });
  it("rendert niets als gesloten", () => {
    expect(
      renderToString(
        <ConfirmModal open={false} notices={base} onConfirm={() => {}} onCancel={() => {}} />,
      ),
    ).toBe("");
  });
});

describe("upload-validatie", () => {
  it("accepteert .ifc/.ifczip, weigert andere en te grote bestanden", () => {
    expect(validateIfcFile({ name: "a.IFC", size: 10 })).toBeNull();
    expect(validateIfcFile({ name: "a.ifczip", size: 10 })).toBeNull();
    expect(validateIfcFile({ name: "a.pdf", size: 10 })).toMatch(/\.ifc/);
    expect(validateIfcFile({ name: "a.ifc", size: 101 * 1024 * 1024 })).toMatch(/te groot/);
  });
  it("voortgangslabels", () => {
    expect(progressLabel({ phase: "upload", fraction: 0.42 })).toBe("Uploaden… 42%");
    expect(progressLabel({ phase: "analyse" })).toBe("Analyseren… een groot model kan ongeveer een minuut duren.");
  });
});

describe("QC-waarschuwingen inklapbaar", () => {
  it("standaard dicht: aantal + eerste 3 groepen + 'Toon alle N'; blokkerend blijft open", () => {
    const many = withFindings(ifcAnalyseMock, [
      ...Array.from({ length: 10 }, (_, i) =>
        finding({ severity: "warning", code: `W-${i}`, message: `Melding ${i}`, count: 5 }),
      ),
      finding({ severity: "blocking", code: "B-1", message: "Blokkade 1" }),
      finding({ severity: "blocking", code: "B-2", message: "Blokkade 2" }),
      finding({ severity: "blocking", code: "B-3", message: "Blokkade 3" }),
      finding({ severity: "blocking", code: "B-4", message: "Blokkade 4" }),
    ]);
    const html = render(0, many);
    expect(html).toContain("Waarschuwingen (10)");
    expect(html).toContain("Toon alle 10");
    expect(html).toContain("Melding 2");
    expect(html).not.toContain("Melding 3<");
    expect(html).toContain("Blokkade 4");
  });

  it("groupFindings telt per code op (count of 1) en onthoudt modelleur-actie en ruimten", () => {
    const groups = groupFindings([
      finding({ severity: "warning", code: "A", message: "m", count: 3, room_id: "r1" }),
      finding({ severity: "warning", code: "A", message: "m", modeller_action: true, room_id: "r2" }),
      finding({ severity: "warning", code: "B", message: "n" }),
    ]);
    expect(groups).toEqual([
      { code: "A", count: 4, modellerAction: true, message: "m", roomIds: ["r1", "r2"] },
      { code: "B", count: 1, modellerAction: false, message: "n", roomIds: [] },
    ]);
  });
});

describe("ChecklistView", () => {
  it("toont ontbreekt-items in foutkleur met link en niets bij lege lijst", () => {
    const html = renderToString(
      <MemoryRouter><ChecklistView
        items={[
          { id: "u-constructies", severity: "ontbreekt", aantal: 3, link: "/constructies", tekst: "3 vlakken tellen nu als 0 W/K" },
          { id: "klimaat", severity: "default", aantal: 1, link: "/project", tekst: "Klimaat default" },
        ]}
      /></MemoryRouter>,
    );
    expect(html).toContain("Nog in te vullen na IFC-import (2)");
    expect(html).toContain("tellen nu als 0 W/K");
    expect(html).toContain("text-red-400");
    expect(html).toContain("--theme-warning-border");
    expect(renderToString(<ChecklistView items={[]} />)).toBe("");
  });
});

describe("MissingUBannerView (Resultaten)", () => {
  it("toont X m2, N vlakken, link naar /constructies en geen sluitknop", () => {
    const html = renderToString(
      <MemoryRouter>
        <MissingUBannerView
          area={242.5}
          count={23}
          geenOpbouwText="Bouwdeel zonder laagopbouw in het model; U-waarde ontbreekt"
        />
      </MemoryRouter>,
    );
    expect(html).toContain("Berekening onvolledig: 242,5 m² zonder U-waarde (23 vlakken)");
    expect(html).toContain("tellen als 0 W/K");
    expect(html).toContain('href="/constructies"');
    expect(html).toContain("Bouwdeel zonder laagopbouw in het model");
    expect(html).not.toContain("Verberg");
    expect(html).not.toContain("&times;");
  });
  it("niets als er geen vlakken zonder U zijn", () => {
    expect(
      renderToString(
        <MemoryRouter>
          <MissingUBannerView area={0} count={0} />
        </MemoryRouter>,
      ),
    ).toBe("");
  });
});
