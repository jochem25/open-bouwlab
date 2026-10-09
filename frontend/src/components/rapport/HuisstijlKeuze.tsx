import { useEffect } from "react";
import { useTranslation } from "react-i18next";

import { useOrganisatieStore } from "../../store/organisatieStore";

const SELECT_CLASS =
  "rounded-md border border-[var(--oaec-border)] bg-[var(--oaec-bg-input)] px-3 py-1.5 text-sm text-on-surface focus:outline-none focus:ring-1 focus:border-primary focus:ring-primary";

/**
 * Huisstijlkeuze voor rapporten.
 *
 * - Een organisatie met meer dan een huisstijl: keuzelijst (standaard voorgeselecteerd).
 * - Een huisstijl: niets te kiezen, niets getoond.
 * - Ingelogd zonder (geldige) organisatie: melding dat rapporten niet beschikbaar zijn.
 * - Niet ingelogd (lokale dev, desktop): niets getoond.
 *
 * De keuze geldt voor alle rapporten in deze sessie; de backend controleert hem.
 */
export function HuisstijlKeuze() {
  const { t } = useTranslation();
  const { status, ingelogd, organisatie, fout, gekozenHuisstijl, laad, kiesHuisstijl } =
    useOrganisatieStore();

  useEffect(() => {
    void laad();
  }, [laad]);

  if (status !== "klaar" || !ingelogd) return null;

  if (!organisatie) {
    return (
      <p role="alert" className="text-sm text-red-400">
        {fout?.detail ?? t("huisstijl.geenOrganisatie")}
      </p>
    );
  }

  if (organisatie.huisstijlen.length < 2) return null;

  return (
    <label className="flex flex-col gap-1 text-sm">
      <span className="font-medium text-on-surface">{t("huisstijl.label")}</span>
      <select
        value={gekozenHuisstijl ?? organisatie.standaard_huisstijl}
        onChange={(e) => kiesHuisstijl(e.target.value)}
        className={SELECT_CLASS}
      >
        {organisatie.huisstijlen.map((h) => (
          <option key={h} value={h}>
            {h === organisatie.standaard_huisstijl ? t("huisstijl.standaard", { naam: h }) : h}
          </option>
        ))}
      </select>
    </label>
  );
}
