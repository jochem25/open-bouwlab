import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import {
  berekenConstructie,
  ConstructieFout,
  genereerConstructieRapport,
  rapportBestandsnaam,
} from "../../lib/constructieClient";
import type { Materiaal, Resultaat } from "../../types/constructie";
import { PageHeader } from "../layout/PageHeader";
import { Card } from "../ui/Card";
import { DisclaimerBanner } from "./DisclaimerBanner";
import { TrillingUitBanner } from "./TrillingUitBanner";
import {
  bouwInvoer,
  type Formulier,
  materiaalOmschrijving,
  STANDAARD_FORMULIER,
  toonTrillingUitBanner,
} from "./formulier";
import { InvoerKolom } from "./InvoerKolom";
import { Uitkomst } from "./Uitkomst";

/** Wachttijd na de laatste invoerwijziging voordat er gerekend wordt. */
export const REKEN_DEBOUNCE_MS = 400;

function foutBericht(e: unknown, fallback: string): string {
  if (e instanceof ConstructieFout || e instanceof Error) return e.message;
  return fallback;
}

/** Beginwaarden per materiaal (hout: vaste hoogte in de buurt van het assortiment). */
function beginFormulier(materiaal: Materiaal): Formulier {
  return materiaal === "hout" ? { ...STANDAARD_FORMULIER, hoogte_mm: 195 } : STANDAARD_FORMULIER;
}

/** Gedeelde pagina voor Staal en Beton: invoerkolom links, uitkomst rechts. */
export function ConstructiePagina({ materiaal }: { materiaal: Materiaal }) {
  const { t } = useTranslation();
  const [formulier, setFormulier] = useState<Formulier>(() => beginFormulier(materiaal));
  const [resultaat, setResultaat] = useState<Resultaat | null>(null);
  const [bezig, setBezig] = useState(false);
  const [fout, setFout] = useState<string | null>(null);
  const [rapportBezig, setRapportBezig] = useState(false);
  const [rapportFout, setRapportFout] = useState<string | null>(null);

  const invoer = useMemo(() => bouwInvoer(formulier, materiaal), [formulier, materiaal]);
  const invoerSleutel = invoer ? JSON.stringify(invoer) : null;
  const laatsteInvoer = useRef(invoer);
  laatsteInvoer.current = invoer;

  useEffect(() => {
    if (invoerSleutel === null) {
      setResultaat(null);
      setBezig(false);
      return;
    }
    const controller = new AbortController();
    setBezig(true);
    const timer = setTimeout(() => {
      const huidig = laatsteInvoer.current;
      if (!huidig) return;
      berekenConstructie(materiaal, huidig, controller.signal)
        .then((r) => {
          setResultaat(r);
          setFout(null);
          setBezig(false);
        })
        .catch((e: unknown) => {
          if (controller.signal.aborted) return;
          setResultaat(null);
          setFout(foutBericht(e, t("constructie.fout.algemeen")));
          setBezig(false);
        });
    }, REKEN_DEBOUNCE_MS);
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
    // `t` bewust niet in deps: taalwissel hoeft geen herberekening.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [invoerSleutel, materiaal]);

  const wijzig = useCallback((patch: Partial<Formulier>) => {
    setFormulier((f) => ({ ...f, ...patch }));
  }, []);

  const downloadRapport = async () => {
    if (!invoer) return;
    setRapportBezig(true);
    setRapportFout(null);
    try {
      const blob = await genereerConstructieRapport(materiaal, invoer);
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = rapportBestandsnaam(materiaal, resultaat?.advies?.naam ?? null);
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
    } catch (e) {
      setRapportFout(foutBericht(e, t("constructie.fout.rapport")));
    } finally {
      setRapportBezig(false);
    }
  };

  const materiaalTekst = materiaalOmschrijving(materiaal, formulier, t("constructie.invoer.hoh"));

  return (
    <div>
      <PageHeader
        title={t(`constructie.${materiaal}.titel`)}
        subtitle={t("constructie.subtitel")}
      />
      <div className="flex flex-col gap-5 p-6">
        <DisclaimerBanner tekst={resultaat?.disclaimer} />
        {toonTrillingUitBanner(formulier, materiaal) && <TrillingUitBanner />}
        <div className="grid gap-6 lg:grid-cols-[minmax(280px,360px)_1fr]">
          <Card title={t("constructie.invoer.titel")} className="h-fit">
            <InvoerKolom materiaal={materiaal} waarde={formulier} onChange={wijzig} />
          </Card>
          <section aria-label={t("constructie.uitkomst.titel")} aria-busy={bezig} className="min-w-0">
            {bezig && (
              <p role="status" className="mb-3 text-sm text-on-surface-secondary">
                {t("constructie.uitkomst.rekenen")}
              </p>
            )}
            {fout && (
              <p role="alert" className="mb-3 rounded-md border border-red-400 px-3 py-2 text-sm text-red-400">
                {fout}
              </p>
            )}
            {invoer === null && (
              <p className="mb-3 text-sm text-on-surface-secondary">{t("constructie.uitkomst.onvolledig")}</p>
            )}
            {resultaat ? (
              <Uitkomst
                materiaal={materiaal}
                resultaat={resultaat}
                overspanningM={formulier.overspanning_m ?? 0}
                materiaalTekst={materiaalTekst}
                rapportBezig={rapportBezig}
                rapportFout={rapportFout}
                onRapport={downloadRapport}
              />
            ) : (
              !bezig &&
              !fout &&
              invoer !== null && (
                <p className="text-sm text-on-surface-secondary">{t("constructie.uitkomst.leeg")}</p>
              )
            )}
          </section>
        </div>
      </div>
    </div>
  );
}
