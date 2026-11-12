import { renderToStaticMarkup } from "react-dom/server";
import { createInstance } from "i18next";
import { I18nextProvider, initReactI18next } from "react-i18next";
import { MemoryRouter } from "react-router-dom";
import type { ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";

import nlCommon from "../../i18n/locales/nl/common.json";
import enCommon from "../../i18n/locales/en/common.json";
import type { TrapInvoer, TrapUitvoer } from "../../types/trap";
import { Sidebar } from "../layout/Sidebar";
import invoerKwart from "./__fixtures__/kwart.invoer.json";
import uitvoerKwart from "./__fixtures__/kwart.uitvoer.json";
import {
  bouwInvoer,
  invoerNaarFormulier,
  parseVakken,
  parseVeelhoek,
  STANDAARD_FORMULIER,
} from "./formulier";
import { Plattegrond } from "./Plattegrond";
import { TRAP_DISCLAIMER, TrapPagina } from "./TrapPagina";
import { TrapInvoer as TrapInvoerScherm } from "./TrapInvoer";
import { TrapToegang } from "./TrapToegang";
import { TrapToetsing } from "./TrapToetsing";
import { Trap } from "../../pages/Trap";

vi.mock("../../hooks/useConstructieEntitlement", () => ({
  useConstructieEntitlement: () => "uit",
}));
const trapRecht = vi.hoisted(() => ({ waarde: "aan" as "laden" | "aan" | "uit" }));
vi.mock("../../hooks/useTrapEntitlement", () => ({
  useTrapEntitlement: () => trapRecht.waarde,
}));

const uitvoer = uitvoerKwart as unknown as TrapUitvoer;

function render(el: ReactElement): string {
  const i18n = createInstance();
  void i18n.use(initReactI18next).init({
    lng: "nl",
    resources: { nl: { common: nlCommon } },
    defaultNS: "common",
    interpolation: { escapeValue: false },
    initImmediate: false,
  });
  return renderToStaticMarkup(
    <I18nextProvider i18n={i18n}>
      <MemoryRouter>{el}</MemoryRouter>
    </I18nextProvider>,
  );
}

function sleutels(o: unknown, prefix = ""): string[] {
  if (o === null || typeof o !== "object") return [prefix];
  return Object.entries(o as Record<string, unknown>).flatMap(([k, v]) =>
    sleutels(v, prefix ? `${prefix}.${k}` : k),
  );
}

describe("formulier", () => {
  it("bouwt een kwartdraai met trappaal en automatische vakken", () => {
    const r = bouwInvoer(STANDAARD_FORMULIER);
    expect(r.fout).toBeUndefined();
    const inv = r.invoer!;
    expect(inv.plattegrond.armlengtes).toEqual([2400, 1600]);
    expect(inv.plattegrond.binnenzijde).toBe("links");
    expect(inv.niveaus).toEqual({ onder: 0, boven: 2800, vloerdikte_boven: 300 });
    expect(inv.trappaal).toEqual({ langs_in: 90, langs_uit: 90 });
    expect(inv.optreden).toBeUndefined();
    expect(inv.verdrijving).toBeUndefined();
  });

  it("rechte trap heeft een arm en geen trappaal", () => {
    const inv = bouwInvoer({ ...STANDAARD_FORMULIER, vorm: "recht" }).invoer!;
    expect(inv.plattegrond.armlengtes).toEqual([2400]);
    expect(inv.trappaal).toBeUndefined();
  });

  it("halfdraai rechts heeft drie armen", () => {
    const inv = bouwInvoer({ ...STANDAARD_FORMULIER, vorm: "half_rechts" }).invoer!;
    expect(inv.plattegrond.armlengtes).toEqual([2400, 1600, 1500]);
    expect(inv.plattegrond.binnenzijde).toBe("rechts");
  });

  it("meldt een leeg verplicht veld in plaats van te rekenen", () => {
    const r = bouwInvoer({ ...STANDAARD_FORMULIER, breedte: null });
    expect(r.invoer).toBeUndefined();
    expect(r.fout).toContain("trapbreedte");
  });

  it("vast aantal optreden, handmatige vakken en trapgat", () => {
    const inv = bouwInvoer({
      ...STANDAARD_FORMULIER,
      optreden_auto: false,
      optreden: 16,
      vakken_auto: false,
      vakken: "0-5, 5-15",
      trapgat: "0,0; 1000,0; 1000,3000",
    }).invoer!;
    expect(inv.optreden).toBe(16);
    expect(inv.verdrijving?.vakken).toEqual([
      { van: 0, tot: 5 },
      { van: 5, tot: 15 },
    ]);
    expect(inv.trapgat).toHaveLength(3);
  });

  it("parsers weigeren onzin", () => {
    expect(parseVakken("5-3")).toBeNull();
    expect(parseVakken("a-b")).toBeNull();
    expect(parseVeelhoek("0,0; 1,1")).toBeNull();
  });

  it("opslaan en laden geeft dezelfde invoer terug", () => {
    const f = { ...STANDAARD_FORMULIER, vorm: "half_rechts" as const, looplijn_straal: 200, vrije_hoogte: 2400 };
    const inv = bouwInvoer(f).invoer!;
    const terug = invoerNaarFormulier(JSON.parse(JSON.stringify(inv)) as unknown);
    expect(terug.fout).toBeUndefined();
    expect(bouwInvoer(terug.formulier!).invoer).toEqual(inv);
  });

  it("laadt de fixture-invoer (kale CLI-invoer met standaarden)", () => {
    const r = invoerNaarFormulier(invoerKwart as unknown as TrapInvoer);
    expect(r.formulier?.vorm).toBe("kwart_links");
    expect(r.formulier?.verdiepingshoogte).toBe(2800);
    expect(invoerNaarFormulier({ naam: "x" }).fout).toBeDefined();
  });

  it("laadt een vrije contour met trappaal en geeft hem ongewijzigd terug", () => {
    const contourInvoer = {
      naam: "contour",
      niveaus: { onder: 0, boven: 3180, vloerdikte_boven: 100 },
      plattegrond: {
        soort: "contour",
        binnenzijde: [[1530.7, -1399.8], [1530.7, -851.4], [52.7, -851.4]],
        buitenzijde: [[2359.6, -1399.8], [2359.6, -48.0], [52.7, -48.0]],
        kant_binnen: "links",
        welstuk: 52.7,
      },
      optreden: 16,
      trappaal: { langs_in: 37.9, langs_uit: 115.4, insteek_in: 0, insteek_uit: 29.5 },
      looplijn: { afstand_buitenkant_boom: 300, hoek: "boog", straal: 500 },
    };
    const r = invoerNaarFormulier(contourInvoer as unknown);
    expect(r.fout).toBeUndefined();
    const f = r.formulier!;
    expect(f.vorm).toBe("contour");
    const inv = bouwInvoer(f).invoer!;
    expect(inv.plattegrond).toEqual(contourInvoer.plattegrond);
    expect(inv.trappaal).toEqual(contourInvoer.trappaal);
    expect(inv.looplijn).toEqual(contourInvoer.looplijn);
    // paal aangepast: dan weer de eenvoudige paal uit het formulier
    expect(bouwInvoer({ ...f, paal: 90 }).invoer!.trappaal).toEqual({ langs_in: 90, langs_uit: 90 });
    // het scherm toont de contour in plaats van de armvelden
    const html = render(<TrapInvoerScherm waarde={f} onChange={() => {}} />);
    expect(html).toContain('data-testid="trap-contour"');
    expect(html).toContain("Vrije contour (geladen)");
  });
});

describe("plattegrond", () => {
  it("tekent treden met nummers, looplijn, klimlijn, minimaal trapgat en hoofdmaten", () => {
    const html = render(<Plattegrond tekening={uitvoer.tekening} rapport={uitvoer.rapport} />);
    expect(html).toContain('data-testid="trap-plattegrond"');
    expect(html).toContain('data-testid="trap-klimlijn"');
    expect(html).toContain('data-testid="trap-looplijn"');
    expect(html).toContain('data-testid="trap-trapgat-minimaal"');
    // een nummer per trede (welstuk niet genummerd)
    const n = uitvoer.tekening.treden.length - 1;
    expect(html).toContain(`>${n}</text>`);
    expect(html).not.toContain(`>${n + 1}</text>`);
    expect(html).toContain("klimlijn 300 mm uit de buitenkant van de boom");
    expect(html).toMatch(/hoofdmaten \d/);
  });
});

describe("toetsing", () => {
  it("toont per regel oordeel, eis en bron met bron-id", () => {
    const html = render(<TrapToetsing regels={uitvoer.rapport.regels} />);
    for (const r of uitvoer.rapport.regels) {
      expect(html).toContain(`data-oordeel="${r.oordeel}"`);
      expect(html).toContain(r.bron_id);
    }
    expect(html).toContain("Bbl (tabel 4.26, klimlijn)");
    // standaardtrap (fixture): geen Wattjes-afwijking (besluit Jochem 12-11-2026)
    expect(html).toContain("advies: in orde");
    expect(html).not.toContain("advies: afwijking");
  });
});

describe("pagina", () => {
  it("toont de disclaimer letterlijk en de invoer, ook zonder resultaat", () => {
    const html = render(<TrapPagina />);
    expect(html).toContain(TRAP_DISCLAIMER);
    expect(uitvoer.rapport.disclaimer).toBe(TRAP_DISCLAIMER);
    expect(html).toContain('data-testid="trap-invoer"');
    expect(html).toContain("Kwartdraai links");
    expect(html).toContain('data-testid="trap-ifc"');
  });

  it("sidebar toont de trapontwerper alleen met entitlement trap", () => {
    trapRecht.waarde = "uit";
    const zonder = render(<Sidebar />);
    expect(zonder).not.toContain('href="/trap"');

    trapRecht.waarde = "aan";
    const html = render(<Sidebar />);
    expect(html).toContain('href="/trap"');
    expect(html).toContain("Trapontwerper");
  });

  it("route /trap zonder entitlement toont een melding, geen ontwerper", () => {
    trapRecht.waarde = "uit";
    const zonder = render(<Trap />);
    expect(zonder).toContain('role="alert"');
    expect(zonder).toContain("Trapontwerper niet geactiveerd");
    expect(zonder).not.toContain('data-testid="trap-invoer"');

    trapRecht.waarde = "laden";
    expect(render(<TrapToegang><p>x</p></TrapToegang>)).toContain('role="status"');

    trapRecht.waarde = "aan";
    const met = render(<Trap />);
    expect(met).toContain('data-testid="trap-invoer"');
    expect(met).toContain(TRAP_DISCLAIMER);
  });

  it("NL en EN hebben dezelfde trap-sleutels", () => {
    const nl = sleutels((nlCommon as Record<string, unknown>).trap).sort();
    const en = sleutels((enCommon as Record<string, unknown>).trap).sort();
    expect(en).toEqual(nl);
  });
});
