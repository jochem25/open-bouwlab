import { useTranslation } from "react-i18next";

import { formatGetal, formatUc } from "../../lib/constructieFormat";
import type { Advies, Materiaal } from "../../types/constructie";
import { Card } from "../ui/Card";

function maxUc(advies: Advies): number | null {
  const ucs = advies.toetsen.map((t) => t.uc).filter((u): u is number => u !== null);
  return ucs.length > 0 ? Math.max(...ucs) : null;
}

interface Props {
  titel: string;
  materiaal: Materiaal;
  advies: Advies;
  /** Staalsoort of sterkteklasse. */
  materiaalTekst: string;
  overspanningM: number;
}

export function AdviesKaart({ titel, materiaal, advies, materiaalTekst, overspanningM }: Props) {
  const { t } = useTranslation();
  const uc = maxUc(advies);
  const groot = materiaal !== "staal" && !advies.naam.includes("mm") ? `${advies.naam} mm` : advies.naam;
  const b = advies.beton;
  return (
    <Card title={titel}>
      <p className="font-heading text-3xl font-bold text-on-surface" data-testid="advies-naam">
        {groot}
      </p>
      {materiaal === "beton" && b && (
        <p className="mt-1 text-sm text-on-surface">
          {t("constructie.advies.wapening")}: <span className="font-mono">{`${b.aantal} D${formatGetal(b.diameter_mm, 0)}`}</span>
          {" · "}
          {t("constructie.advies.beugels")}:{" "}
          <span className="font-mono">{`D${formatGetal(b.beugel_diameter_mm, 0)}-${formatGetal(b.beugel_s_mm, 0)}`}</span>
        </p>
      )}
      <p className="mt-1 text-sm text-on-surface-secondary">
        {materiaalTekst} · l = {formatGetal(overspanningM, 2)} m
      </p>
      <p className="mt-3">
        <span className="inline-block rounded-full bg-green-600 px-3 py-1 text-sm font-medium text-white">
          {t("constructie.advies.voldoet")} - {t("constructie.advies.ucMax")} {formatUc(uc)}
        </span>
      </p>
    </Card>
  );
}
