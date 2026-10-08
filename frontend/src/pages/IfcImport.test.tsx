/**
 * Render-tests (SSR, omgeving is "node") voor de IFC-import-resultaatweergave
 * en de bevestigingsmodal, met de mock-response.
 */
import { renderToString as ssr } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { MemoryRouter } from "react-router-dom";

import { ChecklistView } from "../components/ifcImport/IfcImportChecklistPanel";
import { ConfirmModal } from "../components/ifcImport/ConfirmModal";
import { ImportResultView } from "../components/ifcImport/ImportResultView";
import { progressLabel, validateIfcFile } from "../components/ifcImport/UploadPanel";
import { ifcAnalyseMock } from "../lib/__fixtures__/ifcAnalyseMock";
import { makeImportResult } from "../lib/__fixtures__/ifcImportTestData";

/** SSR voegt `<!-- -->` tussen tekstdelen in; haal die weg voor tekstvergelijking. */
const renderToString = (el: Parameters<typeof ssr>[0]) => ssr(el).replace(/<!-- -->/g, "");

function render(existing = 0) {
  return renderToString(
    <ImportResultView
      response={ifcAnalyseMock}
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
    expect(html).toContain("ifc-ruimtebalans 0.0.0-mock");
    expect(html).toMatch(/data-testid="stat-rooms"[^>]*>2</);
    expect(html).toContain("Modelcheck: blokkerend");
    expect(html).toContain("Blokkerend (1)");
    expect(html).toContain("Waarschuwingen (2)");
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
    expect(progressLabel({ phase: "analyse" })).toBe("Analyseren…");
  });
});

describe("QC-waarschuwingen inklapbaar", () => {
  it("standaard dicht: aantal + eerste 3 + 'Toon alle N'; blokkerend blijft open", () => {
    const many = {
      ...ifcAnalyseMock,
      qc: {
        verdict: "waarschuwing" as const,
        findings: Array.from({ length: 159 }, (_, i) => ({
          severity: "warning" as const,
          code: i % 2 ? "code_a" : "code_b",
          message: `Melding nummer ${i}`,
        })).concat([
          { severity: "blocking" as never, code: "x", message: "Blokkade 1" },
          { severity: "blocking" as never, code: "x", message: "Blokkade 2" },
          { severity: "blocking" as never, code: "x", message: "Blokkade 3" },
          { severity: "blocking" as never, code: "x", message: "Blokkade 4" },
        ]),
      },
    };
    const html = renderToString(
      <ImportResultView
        response={many}
        importResult={makeImportResult(many.thermal)}
        existingRoomCount={0}
        onImport={() => {}}
      />,
    );
    expect(html).toContain("Waarschuwingen (159)");
    expect(html).toContain("Toon alle 159");
    expect(html).toContain("Melding nummer 2");
    expect(html).not.toContain("Melding nummer 3<");
    expect(html).toContain("Blokkade 4");
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
