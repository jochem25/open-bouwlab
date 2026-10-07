import { useTranslation } from "react-i18next";

import { formatGetal } from "../../lib/constructieFormat";

interface Props {
  overspanningM: number;
  /** Ontwerpbelasting q_d in kN/m; optioneel als de server geen tussenwaarde meegeeft. */
  qD?: number | null;
  /** Maatgevend moment in kNm. */
  mEd?: number | null;
}

const X0 = 50;
const X1 = 350;
const Y_LIGGER = 70;
const Y_MOMENT = 110;
const MOMENT_DIEPTE = 50;

/** Ligger op twee steunpunten met gelijkmatig verdeelde belasting en parabolische momentenlijn. */
export function SchemaSvg({ overspanningM, qD, mEd }: Props) {
  const { t } = useTranslation();
  const pijlen = Array.from({ length: 9 }, (_, i) => X0 + ((X1 - X0) / 8) * i);
  const midden = (X0 + X1) / 2;
  const momentPad = `M ${X0} ${Y_MOMENT} Q ${midden} ${Y_MOMENT + 2 * MOMENT_DIEPTE} ${X1} ${Y_MOMENT} Z`;
  const beschrijving = t("constructie.schema.beschrijving", {
    l: formatGetal(overspanningM, 2),
  });

  return (
    <svg
      viewBox="0 0 400 200"
      role="img"
      aria-label={beschrijving}
      className="h-auto w-full max-w-xl text-on-surface"
    >
      <title>{beschrijving}</title>
      {/* belasting */}
      <line x1={X0} y1="20" x2={X1} y2="20" stroke="var(--theme-accent)" strokeWidth="2" />
      {pijlen.map((x) => (
        <g key={x} stroke="var(--theme-accent)" strokeWidth="1.5" fill="var(--theme-accent)">
          <line x1={x} y1="20" x2={x} y2={Y_LIGGER - 8} />
          <polygon points={`${x - 3},${Y_LIGGER - 10} ${x + 3},${Y_LIGGER - 10} ${x},${Y_LIGGER - 2}`} stroke="none" />
        </g>
      ))}
      <text x={midden} y="14" textAnchor="middle" fontSize="11" fill="var(--theme-text)">
        {qD != null ? `q_d = ${formatGetal(qD, 2)} kN/m` : "q_d"}
      </text>
      {/* ligger */}
      <line x1={X0} y1={Y_LIGGER} x2={X1} y2={Y_LIGGER} stroke="currentColor" strokeWidth="4" />
      {/* steunpunten */}
      <polygon points={`${X0},${Y_LIGGER + 2} ${X0 - 8},${Y_LIGGER + 16} ${X0 + 8},${Y_LIGGER + 16}`} fill="currentColor" />
      <polygon points={`${X1},${Y_LIGGER + 2} ${X1 - 8},${Y_LIGGER + 16} ${X1 + 8},${Y_LIGGER + 16}`} fill="currentColor" />
      {/* momentenlijn */}
      <path d={momentPad} fill="var(--theme-accent-soft)" stroke="var(--domain-chart-transmission)" strokeWidth="1.5" />
      <line x1={X0} y1={Y_MOMENT} x2={X1} y2={Y_MOMENT} stroke="var(--theme-text-muted)" strokeWidth="1" />
      <text x={midden} y={Y_MOMENT + MOMENT_DIEPTE + 14} textAnchor="middle" fontSize="11" fill="var(--theme-text)">
        {mEd != null ? `M_Ed = ${formatGetal(mEd, 1)} kNm` : "M_Ed"}
      </text>
      {/* maatlijn */}
      <line x1={X0} y1="190" x2={X1} y2="190" stroke="var(--theme-text-muted)" strokeWidth="1" />
      <text x={midden} y="186" textAnchor="middle" fontSize="11" fill="var(--theme-text)">
        {`l = ${formatGetal(overspanningM, 2)} m`}
      </text>
    </svg>
  );
}
