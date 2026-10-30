/**
 * Werkscherm van de trapontwerper (O-2026-10-30-14), ingedeeld als de mockup:
 * invoer links; rechts plattegrond of 3D, tredenstaat, toetsing en uitvoer.
 * Rekent na elke invoerwijziging (debounce) via trapClient (Tauri of API).
 */
import { useCallback, useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import { useTranslation } from "react-i18next";

import { bewaarTekst, ontwerpTrap } from "../../lib/trapClient";
import type { TrapUitvoer } from "../../types/trap";
import { PageHeader } from "../layout/PageHeader";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { bouwInvoer, type Formulier, invoerNaarFormulier, STANDAARD_FORMULIER } from "./formulier";
import { Plattegrond } from "./Plattegrond";
import { TrapInvoer } from "./TrapInvoer";
import { TrapToetsing } from "./TrapToetsing";
import { TrapViewer3D } from "./TrapViewer3D";

/** Wachttijd na de laatste invoerwijziging voordat er gerekend wordt. */
export const REKEN_DEBOUNCE_MS = 400;

/** Letterlijke disclaimer (shared/uitvoer/trapontwerper/disclaimer.txt); ook in het IFC. */
export const TRAP_DISCLAIMER =
  "Coordinatiemodel, indicatief ontwerp. Definitieve uitwerking, maatvoering, inmeten ter plaatse " +
  "en constructieve verantwoording door de trappenmaker of uitvoerder. Niet zonder controle " +
  "gebruiken voor productie. Opgesteld door Kolthof Bouwadvies.";

const IFC_FILTER = { name: "IFC", extensions: ["ifc"] };
const JSON_FILTER = { name: "Trapinvoer", extensions: ["json"] };

type Weergave = "plattegrond" | "3d";

function getal(x: number, decimalen = 1): string {
  return x.toLocaleString("nl-NL", { minimumFractionDigits: decimalen, maximumFractionDigits: decimalen });
}

export function TrapPagina() {
  const { t } = useTranslation();
  const [formulier, setFormulier] = useState<Formulier>(STANDAARD_FORMULIER);
  const [uitvoer, setUitvoer] = useState<TrapUitvoer | null>(null);
  const [bezig, setBezig] = useState(false);
  const [fout, setFout] = useState<string | null>(null);
  const [laadFout, setLaadFout] = useState<string | null>(null);
  const [weergave, setWeergave] = useState<Weergave>("plattegrond");
  const [toonClearance, setToonClearance] = useState(true);
  const bestandKiezer = useRef<HTMLInputElement | null>(null);

  const gebouwd = useMemo(() => bouwInvoer(formulier), [formulier]);
  const invoer = gebouwd.invoer ?? null;
  const invoerSleutel = invoer ? JSON.stringify(invoer) : null;

  useEffect(() => {
    if (!invoer) {
      setBezig(false);
      return;
    }
    const controller = new AbortController();
    setBezig(true);
    const timer = setTimeout(() => {
      ontwerpTrap(invoer, controller.signal)
        .then((u) => {
          setUitvoer(u);
          setFout(null);
          setBezig(false);
        })
        .catch((e: unknown) => {
          if (controller.signal.aborted) return;
          setFout(e instanceof Error ? e.message : String(e));
          setBezig(false);
        });
    }, REKEN_DEBOUNCE_MS);
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
    // invoerSleutel is de inhoud van `invoer`; de verwijzing wisselt per render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [invoerSleutel]);

  const wijzig = useCallback((patch: Partial<Formulier>) => {
    setFormulier((f) => ({ ...f, ...patch }));
  }, []);

  const laad = async (e: ChangeEvent<HTMLInputElement>) => {
    const bestand = e.target.files?.[0];
    e.target.value = "";
    if (!bestand) return;
    try {
      const r = invoerNaarFormulier(JSON.parse(await bestand.text()) as unknown);
      if (r.fout !== undefined) {
        setLaadFout(r.fout);
        return;
      }
      setLaadFout(null);
      setFormulier(r.formulier);
    } catch {
      setLaadFout(t("trap.uitvoer.geenJson"));
    }
  };

  const naam = uitvoer?.bestandsnaam ?? "trap";
  const r = uitvoer?.rapport;
  const afgekeurd = r?.regels.filter((x) => x.oordeel === "voldoet_niet").length ?? 0;
  const aandacht = r?.regels.filter((x) => x.oordeel === "melding" || x.oordeel === "advies_afwijking").length ?? 0;

  return (
    <div>
      <PageHeader title={t("trap.titel")} subtitle={t("trap.subtitel")} />
      <div className="flex flex-col gap-5 p-6">
        <div
          role="note"
          data-testid="trap-disclaimer"
          className="rounded-md border border-[var(--theme-warning-border)] bg-[var(--theme-warning-bg)] px-4 py-3 text-sm font-medium text-[var(--theme-warning-text)]"
        >
          {r?.disclaimer ?? TRAP_DISCLAIMER}
        </div>
        <div className="grid gap-6 xl:grid-cols-[minmax(300px,380px)_1fr]">
          <Card title={t("trap.invoer.titel")} className="h-fit">
            <TrapInvoer waarde={formulier} onChange={wijzig} />
          </Card>
          <section aria-busy={bezig} className="flex min-w-0 flex-col gap-5">
            {gebouwd.fout !== undefined && (
              <p className="text-sm text-[var(--theme-warning-text)]">{gebouwd.fout}</p>
            )}
            {fout && (
              <p role="alert" data-testid="trap-fout" className="rounded-md border border-red-400 px-3 py-2 text-sm text-red-500">
                {fout}
              </p>
            )}
            {bezig && (
              <p role="status" className="text-sm text-on-surface-secondary">
                {t("trap.rekenen")}
              </p>
            )}
            {uitvoer && r && (
              <>
                <Card>
                  <div className="mb-3 flex flex-wrap items-center gap-2">
                    {(["plattegrond", "3d"] as const).map((w) => (
                      <Button
                        key={w}
                        size="sm"
                        variant={weergave === w ? "primary" : "ghost"}
                        onClick={() => setWeergave(w)}
                        data-testid={`trap-tab-${w}`}
                      >
                        {t(`trap.tab.${w}`)}
                      </Button>
                    ))}
                    {weergave === "3d" && (
                      <label className="ml-auto flex items-center gap-2 text-sm">
                        <input
                          type="checkbox"
                          checked={toonClearance}
                          onChange={(e) => setToonClearance(e.target.checked)}
                          className="h-4 w-4 accent-[var(--theme-accent)]"
                          data-testid="trap-clearance"
                        />
                        {t("trap.drieD.clearance")}
                      </label>
                    )}
                  </div>
                  <p className="mb-2 text-sm text-on-surface-secondary" data-testid="trap-samenvatting">
                    {t("trap.samenvatting", {
                      optreden: r.resultaat.optreden,
                      optrede: getal(r.resultaat.optrede, 2),
                      aantreden: r.resultaat.aantreden,
                      aantrede: getal(r.resultaat.aantrede_looplijn, 2),
                    })}
                  </p>
                  {weergave === "plattegrond" ? (
                    <Plattegrond tekening={uitvoer.tekening} rapport={r} />
                  ) : (
                    <TrapViewer3D ifc={uitvoer.ifc} toonClearance={toonClearance} />
                  )}
                </Card>

                <Card title={t("trap.tredenstaat.titel")}>
                  <div className="overflow-x-auto">
                    <table className="border-collapse text-xs" data-testid="trap-tredenstaat">
                      <tbody>
                        <tr>
                          <th scope="row" className="px-2 py-1 text-left">{t("trap.tredenstaat.trede")}</th>
                          {r.resultaat.aantreden_binnenzijde.map((_, i) => (
                            <td key={i} className="px-2 py-1 text-right font-semibold">{i + 1}</td>
                          ))}
                        </tr>
                        <tr>
                          <th scope="row" className="px-2 py-1 text-left">{t("trap.tredenstaat.binnen")}</th>
                          {r.resultaat.aantreden_binnenzijde.map((a, i) => (
                            <td key={i} className="px-2 py-1 text-right font-mono">{Math.round(a)}</td>
                          ))}
                        </tr>
                        <tr>
                          <th scope="row" className="px-2 py-1 text-left">{t("trap.tredenstaat.looplijn")}</th>
                          <td colSpan={r.resultaat.aantreden_binnenzijde.length} className="px-2 py-1 text-center">
                            {t("trap.tredenstaat.gelijk", { aantrede: getal(r.resultaat.aantrede_looplijn, 2) })}
                          </td>
                        </tr>
                        <tr>
                          <th scope="row" className="px-2 py-1 text-left">{t("trap.tredenstaat.buiten")}</th>
                          {r.resultaat.aantreden_buitenzijde.map((a, i) => (
                            <td key={i} className="px-2 py-1 text-right font-mono">{Math.round(a)}</td>
                          ))}
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </Card>

                <Card title={t("trap.toetsing.titel")}>
                  <p className="mb-2 text-sm" data-testid="trap-toetsing-kop">
                    {afgekeurd > 0
                      ? t("trap.toetsing.afgekeurd", { n: afgekeurd })
                      : t("trap.toetsing.voldoetAlles")}
                    {aandacht > 0 ? ` ${t("trap.toetsing.aandacht", { n: aandacht })}` : ""}
                  </p>
                  {r.waarschuwingen.length > 0 && (
                    <ul className="mb-3 list-disc pl-5 text-sm text-[var(--theme-warning-text)]" data-testid="trap-waarschuwingen">
                      {r.waarschuwingen.map((w) => (
                        <li key={w}>{w}</li>
                      ))}
                    </ul>
                  )}
                  <TrapToetsing regels={r.regels} />
                </Card>
              </>
            )}

            <Card title={t("trap.uitvoer.titel")}>
              <div className="flex flex-wrap gap-2">
                <Button
                  disabled={!uitvoer}
                  data-testid="trap-ifc"
                  onClick={() =>
                    uitvoer && void bewaarTekst(`${naam}.ifc`, uitvoer.ifc, IFC_FILTER, "application/x-step")
                  }
                >
                  {t("trap.uitvoer.ifc")}
                </Button>
                <Button
                  variant="secondary"
                  disabled={!uitvoer}
                  data-testid="trap-ifc-leverancier"
                  onClick={() =>
                    uitvoer &&
                    void bewaarTekst(`${naam}_leverancier.ifc`, uitvoer.ifc_leverancier, IFC_FILTER, "application/x-step")
                  }
                >
                  {t("trap.uitvoer.ifcLeverancier")}
                </Button>
                <Button
                  variant="secondary"
                  disabled={!invoer}
                  data-testid="trap-opslaan"
                  onClick={() =>
                    invoer &&
                    void bewaarTekst(
                      `${naam}.trap.json`,
                      `${JSON.stringify(invoer, null, 2)}\n`,
                      JSON_FILTER,
                      "application/json",
                    )
                  }
                >
                  {t("trap.uitvoer.opslaan")}
                </Button>
                <Button variant="ghost" onClick={() => bestandKiezer.current?.click()} data-testid="trap-laden">
                  {t("trap.uitvoer.laden")}
                </Button>
                <input
                  ref={bestandKiezer}
                  type="file"
                  accept=".json,application/json"
                  className="hidden"
                  onChange={(e) => void laad(e)}
                  data-testid="trap-laden-bestand"
                />
              </div>
              <p className="mt-2 text-xs text-on-surface-secondary">{t("trap.uitvoer.toelichting")}</p>
              {laadFout && (
                <p role="alert" className="mt-2 text-sm text-red-500">
                  {laadFout}
                </p>
              )}
            </Card>
          </section>
        </div>
      </div>
    </div>
  );
}
