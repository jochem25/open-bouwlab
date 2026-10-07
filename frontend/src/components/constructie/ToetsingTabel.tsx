import { useTranslation } from "react-i18next";

import { formatGetal, formatUc } from "../../lib/constructieFormat";
import type { Bron, Toets } from "../../types/constructie";

/** Weergave van waarde/grens: ruime waarden met 1, kleine met 2 decimalen. */
function formatWaarde(x: number | null): string {
  if (x === null) return "-";
  return formatGetal(x, Math.abs(x) >= 100 ? 1 : 2);
}

function bronTekst(bron: Bron, nbLabel: string): string {
  return bron.nb ? `${bron.norm} ${nbLabel} ${bron.artikel}` : `${bron.norm} ${bron.artikel}`;
}

function StatusCel({ toets }: { toets: Toets }) {
  const { t } = useTranslation();
  const s = toets.status;
  // Een aanname (of voldoet) met UC > 1 blokkeert het advies: nooit als aanname of voldoet tonen.
  const overschreden = toets.uc !== null && toets.uc > 1;
  if (overschreden && (s.status === "voldoet" || s.status === "aanname")) {
    return (
      <span className="font-medium text-[var(--theme-danger-color)]">
        {t("constructie.status.voldoetNiet")}
      </span>
    );
  }
  switch (s.status) {
    case "voldoet":
      return <span className="font-medium text-green-600">{t("constructie.status.voldoet")}</span>;
    case "voldoet_niet":
      return (
        <span className="font-medium text-[var(--theme-danger-color)]">
          {t("constructie.status.voldoetNiet")}
        </span>
      );
    case "niet_getoetst":
      // Nooit een vinkje of "voldoet": een niet uitgevoerde toets is geen goedkeuring.
      return (
        <span className="text-[var(--theme-warning-text)]">
          <span className="font-medium">{t("constructie.status.nietGetoetst")}</span>
          <span className="block text-xs">{s.reden}</span>
          {s.blokkeert_advies && (
            <span className="block text-xs font-medium">{t("constructie.status.blokkeertAdvies")}</span>
          )}
        </span>
      );
    case "aanname":
      return (
        <span className="text-on-surface-secondary">
          <span className="font-medium">{t("constructie.status.aanname")}</span>
          <span className="block text-xs">{s.tekst}</span>
        </span>
      );
  }
}

interface Props {
  toetsen: Toets[];
  /** Sleutel van de maatgevende toets; die rij wordt gemarkeerd. */
  maatgevendId?: string | null;
}

export function ToetsingTabel({ toetsen, maatgevendId }: Props) {
  const { t } = useTranslation();
  return (
    <div className="overflow-x-auto">
      <table className="w-full min-w-[640px] border-collapse text-sm">
        <caption className="sr-only">{t("constructie.toetsing.titel")}</caption>
        <thead>
          <tr className="border-b border-[var(--oaec-border)] text-left text-xs text-on-surface-secondary">
            <th scope="col" className="px-2 py-2 font-medium">{t("constructie.toetsing.toets")}</th>
            <th scope="col" className="px-2 py-2 font-medium">{t("constructie.toetsing.formule")}</th>
            <th scope="col" className="px-2 py-2 text-right font-medium">{t("constructie.toetsing.waarde")}</th>
            <th scope="col" className="px-2 py-2 text-right font-medium">{t("constructie.toetsing.grens")}</th>
            <th scope="col" className="px-2 py-2 text-right font-medium">{t("constructie.toetsing.uc")}</th>
            <th scope="col" className="px-2 py-2 font-medium">{t("constructie.toetsing.bron")}</th>
            <th scope="col" className="px-2 py-2 font-medium">{t("constructie.toetsing.status")}</th>
          </tr>
        </thead>
        <tbody>
          {toetsen.map((toets) => {
            const maatgevend = maatgevendId === toets.id;
            return (
              <tr
                key={toets.id}
                data-maatgevend={maatgevend ? "true" : undefined}
                className={`border-b border-[var(--oaec-border-subtle)] align-top ${
                  maatgevend ? "bg-[var(--theme-accent-soft)]" : ""
                }`}
              >
                <th scope="row" className="px-2 py-2 text-left font-medium text-on-surface">
                  {toets.naam}
                  {maatgevend && (
                    <span className="ml-2 rounded bg-primary px-1.5 py-0.5 text-2xs text-on-accent">
                      {t("constructie.toetsing.maatgevend")}
                    </span>
                  )}
                </th>
                <td className="px-2 py-2 font-mono text-xs text-on-surface-secondary">{toets.formule}</td>
                <td className="px-2 py-2 text-right font-mono">{formatWaarde(toets.waarde)}</td>
                <td className="px-2 py-2 text-right font-mono">
                  {toets.grens === null ? "-" : `${formatWaarde(toets.grens)} ${toets.eenheid}`}
                </td>
                <td className="px-2 py-2 text-right font-mono">{formatUc(toets.uc)}</td>
                <td className="px-2 py-2 text-xs text-on-surface-secondary">
                  {bronTekst(toets.bron, t("constructie.toetsing.nb"))}
                  {toets.bron.nb && (
                    <span className="ml-1 rounded border border-[var(--oaec-border)] px-1 text-2xs">
                      {t("constructie.toetsing.nbBadge")}
                    </span>
                  )}
                </td>
                <td className="px-2 py-2">
                  <StatusCel toets={toets} />
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
