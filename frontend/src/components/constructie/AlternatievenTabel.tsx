import { useTranslation } from "react-i18next";

import { formatGetal, formatUc } from "../../lib/constructieFormat";
import type { Alternatief, Kandidaat } from "../../types/constructie";

interface Props {
  alternatieven: Alternatief[];
  kandidaten: Kandidaat[];
}

export function AlternatievenTabel({ alternatieven, kandidaten }: Props) {
  const { t } = useTranslation();
  if (alternatieven.length === 0) return null;
  return (
    <div className="overflow-x-auto">
      <table className="w-full min-w-[520px] border-collapse text-sm">
        <caption className="sr-only">{t("constructie.alternatieven.titel")}</caption>
        <thead>
          <tr className="border-b border-[var(--oaec-border)] text-left text-xs text-on-surface-secondary">
            <th scope="col" className="px-2 py-2 font-medium">{t("constructie.alternatieven.naam")}</th>
            <th scope="col" className="px-2 py-2 text-right font-medium">{t("constructie.alternatieven.hoogte")}</th>
            <th scope="col" className="px-2 py-2 text-right font-medium">{t("constructie.alternatieven.gewichtUc")}</th>
            <th scope="col" className="px-2 py-2 font-medium">{t("constructie.alternatieven.maatgevend")}</th>
            <th scope="col" className="px-2 py-2 font-medium">{t("constructie.alternatieven.status")}</th>
          </tr>
        </thead>
        <tbody>
          {alternatieven.map((alt) => {
            const kand = kandidaten.find((k) => k.naam === alt.naam);
            return (
              <tr key={`${alt.positie}-${alt.naam}`} className="border-b border-[var(--oaec-border-subtle)] align-top">
                <th scope="row" className="px-2 py-2 text-left font-medium text-on-surface">{alt.naam}</th>
                <td className="px-2 py-2 text-right font-mono">
                  {kand ? formatGetal(kand.hoogte_mm, 0) : "-"}
                </td>
                <td className="px-2 py-2 text-right font-mono">
                  {kand?.gewicht_kg_m != null ? `${formatGetal(kand.gewicht_kg_m, 1)} kg/m / ` : ""}
                  {t("constructie.alternatieven.uc")} {formatUc(alt.max_uc)}
                </td>
                <td className="px-2 py-2">{alt.maatgevend ?? "-"}</td>
                <td className="px-2 py-2">
                  <span
                    className={
                      alt.voldoet ? "font-medium text-green-600" : "font-medium text-[var(--theme-danger-color)]"
                    }
                  >
                    {alt.voldoet ? t("constructie.status.voldoet") : t("constructie.status.voldoetNiet")}
                  </span>
                  {alt.toelichting && <span className="block text-xs text-on-surface-secondary">{alt.toelichting}</span>}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
