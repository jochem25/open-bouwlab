import { Link } from "react-router-dom";

import { findMissingU } from "../../lib/ifcImportChecklist";
import { qcCodeText } from "../../lib/qcCodeTexts";
import { useProjectStore } from "../../store/projectStore";

const L_GEEN_OPBOUW = "L-GEEN-OPBOUW";

const fmtM2 = (n: number): string =>
  n.toLocaleString("nl-NL", { minimumFractionDigits: 1, maximumFractionDigits: 1 });

/** Presentatie, los van de store (testbaar via SSR). Niet wegklikbaar. */
export function MissingUBannerView({
  area,
  count,
  geenOpbouwText,
}: {
  area: number;
  count: number;
  /** Gewone-taaltekst bij L-GEEN-OPBOUW, indien de IFC-analyse die meldt. */
  geenOpbouwText?: string | null;
}) {
  if (count === 0) return null;
  return (
    <div
      role="alert"
      data-testid="missing-u-banner"
      className="rounded-md border border-red-400 px-4 py-3 text-sm text-red-400"
    >
      <p className="font-medium">
        Berekening onvolledig: {fmtM2(area)} m² zonder U-waarde ({count} vlakken) — die tellen als
        0 W/K.{" "}
        <Link to="/constructies" className="underline">
          Naar Constructies
        </Link>
      </p>
      {geenOpbouwText && <p className="mt-1 text-on-surface-secondary">{geenOpbouwText}</p>}
    </div>
  );
}

/** Live melding op de Resultaten-pagina: elk vlak met U = 0 maakt de uitkomst onvolledig. */
export function MissingUBanner() {
  const project = useProjectStore((s) => s.project);
  const origin = useProjectStore((s) => s.ifcImport);
  const missing = findMissingU(project.rooms);
  const count = missing.surfaces + missing.openings;
  const geenOpbouw = origin?.analyse?.qc.findings.find((f) => f.code === L_GEEN_OPBOUW);
  return (
    <MissingUBannerView
      area={missing.area}
      count={count}
      geenOpbouwText={geenOpbouw ? qcCodeText(geenOpbouw.code, geenOpbouw.message) : null}
    />
  );
}
