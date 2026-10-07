import { useTranslation } from "react-i18next";

import type { Melding } from "../../types/constructie";

const KLEUR: Record<Melding["soort"], string> = {
  waarschuwing: "border-[var(--theme-warning-border)] bg-[var(--theme-warning-bg)] text-[var(--theme-warning-text)]",
  signalering: "border-[var(--oaec-border)] bg-[var(--theme-accent-soft)] text-on-surface",
  info: "border-[var(--oaec-border-subtle)] text-on-surface-secondary",
};

export function MeldingenLijst({ meldingen }: { meldingen: Melding[] }) {
  const { t } = useTranslation();
  if (meldingen.length === 0) return null;
  return (
    <ul className="flex flex-col gap-2" aria-label={t("constructie.meldingen.titel")}>
      {meldingen.map((m, i) => (
        <li key={`${i}-${m.tekst}`} className={`rounded-md border px-3 py-2 text-sm ${KLEUR[m.soort]}`}>
          <span className="mr-2 font-medium">{t(`constructie.meldingen.${m.soort}`)}:</span>
          {m.tekst}
          {m.bron && (
            <span className="ml-2 text-xs opacity-80">
              ({m.bron.nb ? `${m.bron.norm} ${t("constructie.toetsing.nb")} ${m.bron.artikel}` : `${m.bron.norm} ${m.bron.artikel}`})
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}
