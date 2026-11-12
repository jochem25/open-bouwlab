/**
 * Toetsing van de trap: Bbl (tabel 4.26 en klimlijn) en Wattjes-adviezen, per
 * regel eis, waarde, oordeel en bron met bron-id (uit het normcorpus).
 */
import { useTranslation } from "react-i18next";

import type { Oordeel, TrapRegel } from "../../types/trap";

const KLASSE: Record<Oordeel, string> = {
  voldoet: "text-green-700",
  voldoet_niet: "font-semibold text-[var(--theme-danger-color)]",
  melding: "text-[var(--theme-warning-text)]",
  advies_voldoet: "text-green-700",
  advies_afwijking: "text-[var(--theme-warning-text)]",
  handmatig: "text-on-surface-secondary",
  niet_van_toepassing: "text-on-surface-secondary",
};

function getal(x: number | null): string {
  if (x === null) return "-";
  return x.toLocaleString("nl-NL", { maximumFractionDigits: 1 });
}

/** Bbl-regels eerst, dan de adviezen (Wattjes, praktijk). */
function isBbl(r: TrapRegel): boolean {
  return r.id.startsWith("bbl");
}

export function TrapToetsing({ regels }: { regels: TrapRegel[] }) {
  const { t } = useTranslation();
  const groepen: [string, TrapRegel[]][] = [
    [t("trap.toetsing.bbl"), regels.filter(isBbl)],
    [t("trap.toetsing.advies"), regels.filter((r) => !isBbl(r))],
  ];
  return (
    <div className="overflow-x-auto">
      <table data-testid="trap-toetsing" className="w-full min-w-[760px] border-collapse text-sm">
        <caption className="sr-only">{t("trap.toetsing.titel")}</caption>
        <thead>
          <tr className="border-b border-[var(--oaec-border)] text-left text-xs text-on-surface-secondary">
            <th scope="col" className="px-2 py-2 font-medium">{t("trap.toetsing.onderwerp")}</th>
            <th scope="col" className="px-2 py-2 font-medium">{t("trap.toetsing.eis")}</th>
            <th scope="col" className="px-2 py-2 text-right font-medium">{t("trap.toetsing.waarde")}</th>
            <th scope="col" className="px-2 py-2 font-medium">{t("trap.toetsing.oordeel")}</th>
            <th scope="col" className="px-2 py-2 font-medium">{t("trap.toetsing.bron")}</th>
          </tr>
        </thead>
        {groepen.map(([titel, rijen]) => (
          <tbody key={titel}>
            <tr>
              <th colSpan={5} scope="colgroup" className="bg-[var(--oaec-bg-subtle,#f4f4f5)] px-2 py-1 text-left text-xs font-semibold">
                {titel}
              </th>
            </tr>
            {rijen.map((r) => (
              <tr key={r.id} data-oordeel={r.oordeel} className="border-b border-[var(--oaec-border-subtle)] align-top">
                <th scope="row" className="px-2 py-2 text-left font-medium text-on-surface">
                  {r.onderwerp}
                  {r.toelichting && (
                    <span className="block text-xs font-normal text-on-surface-secondary">{r.toelichting}</span>
                  )}
                </th>
                <td className="px-2 py-2 text-xs">{r.eis}</td>
                <td className="px-2 py-2 text-right font-mono">
                  {getal(r.waarde)} {r.waarde !== null ? r.eenheid : ""}
                </td>
                <td className={`px-2 py-2 ${KLASSE[r.oordeel]}`}>{t(`trap.oordeel.${r.oordeel}`)}</td>
                <td className="px-2 py-2 text-xs text-on-surface-secondary">
                  {r.bron}
                  <span className="block font-mono text-2xs">
                    {r.bron_id}
                    {r.regel ? ` ${r.regel}` : ""}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        ))}
      </table>
    </div>
  );
}
