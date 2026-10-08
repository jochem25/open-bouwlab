import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { controleerAfbeelding, leesAfbeelding } from "../../lib/afbeelding";
import type { CoverImage } from "../../types/project";

interface Props {
  waarde: CoverImage | null;
  onChange: (foto: CoverImage | null) => void;
}

/**
 * Optionele coverfoto voor het rapport. Zonder foto gebruikt het rapport de
 * standaardfoto van de organisatie (aangevuld door de backend).
 */
export function CoverfotoKiezer({ waarde, onChange }: Props) {
  const { t } = useTranslation();
  const invoerRef = useRef<HTMLInputElement>(null);
  const [fout, setFout] = useState<string | null>(null);

  const kies = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) return;
    const controle = controleerAfbeelding(file);
    if (controle) {
      setFout(t(`constructie.rapport.coverfoto.fout_${controle}`));
      return;
    }
    try {
      onChange(await leesAfbeelding(file));
      setFout(null);
    } catch {
      setFout(t("constructie.rapport.coverfoto.fout_lezen"));
    }
  };

  return (
    <div className="flex flex-col gap-1 text-sm">
      <span className="font-medium text-on-surface">{t("constructie.rapport.coverfoto.label")}</span>
      <div className="flex flex-wrap items-center gap-2">
        {waarde ? (
          <>
            <span className="truncate text-on-surface-secondary">{waarde.filename ?? "afbeelding"}</span>
            <button
              type="button"
              onClick={() => onChange(null)}
              className="rounded border border-border px-2 py-1 text-xs text-on-surface-secondary hover:bg-surface-alt"
            >
              {t("constructie.rapport.coverfoto.verwijderen")}
            </button>
          </>
        ) : (
          <span className="text-on-surface-secondary">{t("constructie.rapport.coverfoto.standaard")}</span>
        )}
        <button
          type="button"
          onClick={() => invoerRef.current?.click()}
          className="rounded border border-border px-2 py-1 text-xs text-on-surface-secondary hover:bg-surface-alt"
        >
          {t("constructie.rapport.coverfoto.kiezen")}
        </button>
      </div>
      <span className="text-xs text-on-surface-muted">{t("constructie.rapport.coverfoto.hint")}</span>
      {fout && (
        <p role="alert" className="text-xs text-red-400">
          {fout}
        </p>
      )}
      <input
        ref={invoerRef}
        type="file"
        accept="image/png,image/jpeg"
        onChange={kies}
        className="hidden"
        aria-label={t("constructie.rapport.coverfoto.label")}
      />
    </div>
  );
}
