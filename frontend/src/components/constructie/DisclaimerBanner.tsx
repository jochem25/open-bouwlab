import { useTranslation } from "react-i18next";

import { CONSTRUCTIE_DISCLAIMER } from "../../lib/constructieFormat";

interface Props {
  /** Disclaimer uit het resultaat; zonder resultaat geldt de vaste tekst. */
  tekst?: string;
}

/** Vaste, niet-sluitbare banner. Altijd zichtbaar, ook zonder resultaat. */
export function DisclaimerBanner({ tekst }: Props) {
  const { t } = useTranslation();
  return (
    <div
      role="note"
      aria-label={t("constructie.banner.label")}
      data-testid="constructie-banner"
      className="rounded-md border border-[var(--theme-warning-border)] bg-[var(--theme-warning-bg)] px-4 py-3 text-sm font-medium text-[var(--theme-warning-text)]"
    >
      {tekst && tekst.length > 0 ? tekst : CONSTRUCTIE_DISCLAIMER}
    </div>
  );
}
