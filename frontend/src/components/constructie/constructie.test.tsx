import { renderToStaticMarkup } from "react-dom/server";
import { createInstance } from "i18next";
import { I18nextProvider, initReactI18next } from "react-i18next";
import { MemoryRouter } from "react-router-dom";
import type { ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";

import nlCommon from "../../i18n/locales/nl/common.json";
import { CONSTRUCTIE_DISCLAIMER } from "../../lib/constructieFormat";
import type { Resultaat, Toets } from "../../types/constructie";
import { Sidebar } from "../layout/Sidebar";
import { ConstructiePagina } from "./ConstructiePagina";
import { CoverfotoKiezer } from "./CoverfotoKiezer";
import { STANDAARD_FORMULIER, TRILLING_UIT_ZIN, toonTrillingUitBanner } from "./formulier";
import { TrillingUitBanner } from "./TrillingUitBanner";
import { ToetsingTabel } from "./ToetsingTabel";
import { Uitkomst } from "./Uitkomst";

const entitlement = vi.hoisted(() => ({ waarde: "uit" as "laden" | "aan" | "uit" }));
vi.mock("../../hooks/useConstructieEntitlement", () => ({
  useConstructieEntitlement: () => entitlement.waarde,
}));

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

const bron = { norm: "EC3", artikel: "6.2.5", nb: false };

function toets(patch: Partial<Toets>): Toets {
  return {
    id: "t",
    naam: "Toets",
    formule: "a <= b",
    waarde: 1,
    grens: 2,
    eenheid: "kNm",
    uc: 0.5,
    bron,
    status: { status: "voldoet" },
    ...patch,
  };
}

describe("ToetsingTabel", () => {
  it("toont 'niet getoetst' met reden en nooit 'voldoet' voor die rij", () => {
    const html = render(
      <ToetsingTabel
        toetsen={[
          toets({
            id: "trilling",
            naam: "Trilling",
            uc: null,
            waarde: null,
            grens: null,
            status: { status: "niet_getoetst", reden: "Invoer ontbreekt voor trillingstoets", blokkeert_advies: false },
          }),
        ]}
      />,
    );
    expect(html).toContain("Niet getoetst");
    expect(html).toContain("Invoer ontbreekt voor trillingstoets");
    expect(html).not.toContain("Voldoet");
    expect(html).not.toContain("✓");
    expect(html).not.toContain("✔");
  });

  it("toont de aanname-tekst, UC met komma en markeert de maatgevende rij", () => {
    const html = render(
      <ToetsingTabel
        maatgevendId="buiging"
        toetsen={[
          toets({ id: "buiging", naam: "Buiging", uc: 0.845 }),
          toets({ id: "a", naam: "Aangenomen", status: { status: "aanname", tekst: "Opleglengte 100 mm" } }),
        ]}
      />,
    );
    expect(html).toContain("0,85");
    expect(html).toContain("Opleglengte 100 mm");
    expect(html).toContain('data-maatgevend="true"');
    expect(html.match(/data-maatgevend/g)?.length).toBe(1);
  });

  it("toont NB bij een bron uit de nationale bijlage", () => {
    const html = render(<ToetsingTabel toetsen={[toets({ bron: { norm: "EC2", artikel: "6.1", nb: true } })]} />);
    expect(html).toContain("EC2 NB 6.1");
  });
});

describe("DisclaimerBanner", () => {
  it("staat ook zonder resultaat op de pagina", () => {
    const html = render(<ConstructiePagina materiaal="staal" />);
    expect(html).toContain('data-testid="constructie-banner"');
    expect(html).toContain(CONSTRUCTIE_DISCLAIMER);
  });

  it("staat bij beton ook zonder resultaat op de pagina", () => {
    expect(render(<ConstructiePagina materiaal="beton" />)).toContain(CONSTRUCTIE_DISCLAIMER);
  });
});

describe("Profielreeksen staal", () => {
  it("biedt UNP aan, standaard niet aangevinkt", () => {
    const html = render(<ConstructiePagina materiaal="staal" />);
    expect(html).toContain(">UNP<");
    expect(STANDAARD_FORMULIER.reeksen).not.toContain("UNP");
  });
});

describe("Hout-pagina", () => {
  it("toont de vloerplaat-velden bij balklaag (default) met helptekst", () => {
    const html = render(<ConstructiePagina materiaal="hout" />);
    expect(html).toContain(CONSTRUCTIE_DISCLAIMER);
    expect(html).toContain('data-testid="vloerplaat-blok"');
    expect(html).toContain("Vloerlagen (voor trillingstoets)");
    expect(html).toContain("leverancier");
    expect(html).toContain('data-testid="dwarsverbinding-blok"');
    expect(html).toContain("Trillingstoets (EC5 7.3) meenemen");
    expect(html).not.toContain('data-testid="trilling-uit-banner"');
  });

  it("toont de banner alleen als de trillingstoets uit staat (balklaag + vloer)", () => {
    const uit = { ...STANDAARD_FORMULIER, trillingstoets: false };
    expect(toonTrillingUitBanner(uit, "hout")).toBe(true);
    expect(toonTrillingUitBanner(STANDAARD_FORMULIER, "hout")).toBe(false);
    expect(toonTrillingUitBanner(uit, "staal")).toBe(false);
    expect(toonTrillingUitBanner({ ...uit, toepassing: "dak" }, "hout")).toBe(false);
    const html = render(<TrillingUitBanner />);
    expect(html).toContain('data-testid="trilling-uit-banner"');
    expect(html).toContain(TRILLING_UIT_ZIN);
  });
});

describe("Uitkomst zonder advies", () => {
  it("toont een duidelijke melding en geen advieskaart", () => {
    const resultaat: Resultaat = {
      disclaimer: CONSTRUCTIE_DISCLAIMER,
      kern_versie: "x",
      normedities: [],
      gevolgklasse: "CC2",
      kandidaten: [],
      advies: null,
      laagste_bouwhoogte: null,
      alternatieven: [],
      meldingen: [],
    };
    const html = render(
      <Uitkomst
        materiaal="staal"
        resultaat={resultaat}
        overspanningM={5}
        materiaalTekst="S235"
        rapportBezig={false}
        rapportFout={null}
        onRapport={() => {}}
        coverfoto={null}
        onCoverfoto={() => {}}
      />,
    );
    expect(html).toContain("Geen advies binnen bereik");
    expect(html).not.toContain("advies-naam");
    // Coverfoto-keuze staat bij de rapportknop; zonder upload de standaardfoto.
    expect(html).toContain("Coverfoto");
    expect(html).toContain("Geen eigen foto: standaardfoto van je organisatie");
  });

  it("toont de gekozen coverfoto met verwijderknop", () => {
    const html = render(
      <CoverfotoKiezer
        waarde={{ data: "QUJD", media_type: "image/jpeg", filename: "bouwplaats.jpg" }}
        onChange={() => {}}
      />,
    );
    expect(html).toContain("bouwplaats.jpg");
    expect(html).toContain("Verwijderen");
    expect(html).not.toContain("standaardfoto van je organisatie");
  });
});

describe("Sidebar", () => {
  it("toont de groep Constructie alleen met entitlement", () => {
    entitlement.waarde = "uit";
    const zonder = render(<Sidebar />);
    expect(zonder).not.toContain("/constructie/staal");

    entitlement.waarde = "aan";
    const met = render(<Sidebar />);
    expect(met).toContain("/constructie/staal");
    expect(met).toContain("/constructie/beton");
    expect(met).toContain('href="/constructie/hout"');
    expect(met).not.toContain("Hout (volgt)");
  });
});
