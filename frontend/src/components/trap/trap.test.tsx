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
import { TrapToetsing } from "./TrapToetsing";

vi.mock("../../hooks/useConstructieEntitlement", () => ({
  useConstructieEntitlement: () => "uit",
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
    expect(html).toContain("klimlijn 300 mm uit de buitenzijde");
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
    expect(html).toContain("advies: afwijking");
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

  it("sidebar heeft de trapontwerper", () => {
    const html = render(<Sidebar />);
    expect(html).toContain('href="/trap"');
    expect(html).toContain("Trapontwerper");
  });

  it("NL en EN hebben dezelfde trap-sleutels", () => {
    const nl = sleutels((nlCommon as Record<string, unknown>).trap).sort();
    const en = sleutels((enCommon as Record<string, unknown>).trap).sort();
    expect(en).toEqual(nl);
  });
});
