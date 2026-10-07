import { useTranslation } from "react-i18next";

import { formatGetal, formatUc } from "../../lib/constructieFormat";
import type { Materiaal, Resultaat } from "../../types/constructie";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { AdviesKaart } from "./AdviesKaart";
import { AlternatievenTabel } from "./AlternatievenTabel";
import { MeldingenLijst } from "./MeldingenLijst";
import { SchemaSvg } from "./SchemaSvg";
import { ToetsingTabel } from "./ToetsingTabel";

interface Props {
  materiaal: Materiaal;
  resultaat: Resultaat;
  overspanningM: number;
  /** Staalsoort of sterkteklasse, voor onder de advieskaart. */
  materiaalTekst: string;
  rapportBezig: boolean;
  rapportFout: string | null;
  onRapport: () => void;
}

export function Uitkomst({
  materiaal,
  resultaat,
  overspanningM,
  materiaalTekst,
  rapportBezig,
  rapportFout,
  onRapport,
}: Props) {
  const { t } = useTranslation();
  const { advies, laagste_bouwhoogte: laagste } = resultaat;
  const kandidaat = advies ? resultaat.kandidaten.find((k) => k.naam === advies.naam) : undefined;
  // Zonder advies tonen we de toetsen van de dichtstbijzijnde kandidaat (hoogste index = zwaarste).
  const toonToetsen = advies?.toetsen ?? resultaat.kandidaten[resultaat.kandidaten.length - 1]?.toetsen ?? [];

  return (
    <div className="flex flex-col gap-5">
      {advies ? (
        <>
          <div className="grid gap-4 md:grid-cols-2">
            <AdviesKaart
              titel={t("constructie.advies.titel")}
              materiaal={materiaal}
              advies={advies}
              materiaalTekst={materiaalTekst}
              overspanningM={overspanningM}
            />
            {materiaal === "staal" && laagste && (
              <AdviesKaart
                titel={t("constructie.advies.laagsteBouwhoogte")}
                materiaal={materiaal}
                advies={laagste}
                materiaalTekst={materiaalTekst}
                overspanningM={overspanningM}
              />
            )}
          </div>

          {advies.maatgevend && (
            <Card title={t("constructie.maatgevend.titel")}>
              <p className="text-sm text-on-surface">
                <span className="font-medium">{advies.maatgevend.naam}</span> - {t("constructie.advies.uc")}{" "}
                <span className="font-mono">{formatUc(advies.maatgevend.uc)}</span>
              </p>
              <p className="mt-1 text-sm text-on-surface-secondary">{advies.maatgevend.uitleg}</p>
            </Card>
          )}

          <Card title={t("constructie.kengetallen.titel")}>
            <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm sm:grid-cols-3">
              <div>
                <dt className="text-xs text-on-surface-secondary">{t("constructie.kengetallen.constructiehoogte")}</dt>
                <dd className="font-mono">{formatGetal(advies.kengetallen.constructiehoogte_mm, 0)} mm</dd>
              </div>
              <div>
                <dt className="text-xs text-on-surface-secondary">{t("constructie.kengetallen.lh")}</dt>
                <dd className="font-mono">{formatGetal(advies.kengetallen.l_h, 1)}</dd>
              </div>
              <div>
                <dt className="text-xs text-on-surface-secondary">{t("constructie.kengetallen.eigenGewicht")}</dt>
                <dd className="font-mono">
                  {formatGetal(advies.kengetallen.eigen_gewicht_kn_m, 2)} kN/m
                  {advies.kengetallen.gewicht_kg_m != null &&
                    ` (${formatGetal(advies.kengetallen.gewicht_kg_m, 1)} kg/m)`}
                </dd>
              </div>
            </dl>
          </Card>

          <Card title={t("constructie.schema.titel")}>
            <SchemaSvg
              overspanningM={overspanningM}
              qD={kandidaat?.tussenwaarden["q_d"]}
              mEd={kandidaat?.tussenwaarden["m_ed"]}
            />
          </Card>
        </>
      ) : (
        <div role="alert" className="rounded-md border border-[var(--theme-warning-border)] bg-[var(--theme-warning-bg)] px-4 py-3 text-sm font-medium text-[var(--theme-warning-text)]">
          {t("constructie.advies.geenAdvies")}
        </div>
      )}

      {toonToetsen.length > 0 && (
        <Card title={t("constructie.toetsing.titel")}>
          <ToetsingTabel toetsen={toonToetsen} maatgevendId={advies?.maatgevend?.toets_id} />
        </Card>
      )}

      {resultaat.alternatieven.length > 0 && (
        <Card title={t("constructie.alternatieven.titel")}>
          <AlternatievenTabel alternatieven={resultaat.alternatieven} kandidaten={resultaat.kandidaten} />
        </Card>
      )}

      {resultaat.meldingen.length > 0 && (
        <Card title={t("constructie.meldingen.titel")}>
          <MeldingenLijst meldingen={resultaat.meldingen} />
        </Card>
      )}

      <div className="flex items-center gap-3">
        <Button type="button" variant="secondary" onClick={onRapport} disabled={rapportBezig}>
          {rapportBezig ? t("constructie.rapport.bezig") : t("constructie.rapport.knop")}
        </Button>
        {rapportFout && (
          <p role="alert" className="text-sm text-red-400">
            {rapportFout}
          </p>
        )}
      </div>
    </div>
  );
}
