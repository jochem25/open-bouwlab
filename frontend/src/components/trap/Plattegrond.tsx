/**
 * Plattegrond van de trap als SVG: treden met nummers, bomen en trappalen,
 * looplijn, klimlijn (300 uit de buitenzijde), het minimale trapgat (uit de
 * clearance), een opgegeven trapgat en de hoofdmaten.
 *
 * Projectcoordinaten in mm (y omhoog); de SVG draait y om.
 */
import { useTranslation } from "react-i18next";

import type { P2, TrapRapport, TrapTekening } from "../../types/trap";

/** Ruimte rondom de tekening voor maatlijnen en tekst (mm in tekenschaal). */
const MARGE = 450;
/** Afstand van de maatlijn tot de tekening (mm). */
const MAAT_AFSTAND = 200;
/** Tekstgrootte in mm (in tekenschaal). */
const TEKST = 70;

function pad(punten: P2[], sluit: boolean): string {
  if (punten.length === 0) return "";
  const d = punten.map(([x, y], i) => `${i === 0 ? "M" : "L"}${x.toFixed(1)},${(-y).toFixed(1)}`);
  return d.join(" ") + (sluit ? " Z" : "");
}

function zwaartepunt(p: P2[]): P2 {
  let a = 0;
  let cx = 0;
  let cy = 0;
  for (let i = 0; i < p.length; i++) {
    const [x0, y0] = p[i]!;
    const [x1, y1] = p[(i + 1) % p.length]!;
    const k = x0 * y1 - x1 * y0;
    a += k;
    cx += (x0 + x1) * k;
    cy += (y0 + y1) * k;
  }
  if (Math.abs(a) < 1e-9) {
    const n = p.length || 1;
    return [p.reduce((s, q) => s + q[0], 0) / n, p.reduce((s, q) => s + q[1], 0) / n];
  }
  return [cx / (3 * a), cy / (3 * a)];
}

interface Kader {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

function kader(groepen: P2[][]): Kader {
  const k = { x0: Infinity, y0: Infinity, x1: -Infinity, y1: -Infinity };
  for (const g of groepen) {
    for (const [x, y] of g) {
      k.x0 = Math.min(k.x0, x);
      k.y0 = Math.min(k.y0, y);
      k.x1 = Math.max(k.x1, x);
      k.y1 = Math.max(k.y1, y);
    }
  }
  return k;
}

function mm(x: number): string {
  return Math.round(x).toLocaleString("nl-NL");
}

interface Props {
  tekening: TrapTekening;
  rapport: TrapRapport;
}

export function Plattegrond({ tekening, rapport }: Props) {
  const { t } = useTranslation();
  const trap = kader([...tekening.treden, ...tekening.bomen, ...tekening.palen]);
  const alles = kader([
    ...tekening.treden,
    ...tekening.bomen,
    ...tekening.palen,
    rapport.trapgat_minimaal,
    tekening.trapgat ?? [],
  ]);
  const vb = [alles.x0 - MARGE, -alles.y1 - MARGE, alles.x1 - alles.x0 + 2 * MARGE, alles.y1 - alles.y0 + 2 * MARGE];
  const breedte = trap.x1 - trap.x0;
  const diepte = trap.y1 - trap.y0;
  const yMaat = -(alles.y0 - MAAT_AFSTAND);
  const xMaat = alles.x1 + MAAT_AFSTAND;
  const start = tekening.looplijn[0];
  const r = rapport.resultaat;

  return (
    <figure className="m-0">
      <svg
        data-testid="trap-plattegrond"
        role="img"
        aria-label={t("trap.plattegrond.titel")}
        viewBox={vb.map((v) => v.toFixed(0)).join(" ")}
        className="h-[520px] w-full rounded-md border border-[var(--oaec-border)] bg-[var(--oaec-bg-surface,white)]"
      >
        {rapport.trapgat_minimaal.length > 2 && (
          <path
            d={pad(rapport.trapgat_minimaal, true)}
            fill="none"
            stroke="var(--theme-danger-color, #c0392b)"
            strokeWidth={12}
            strokeDasharray="60 40"
            data-testid="trap-trapgat-minimaal"
          />
        )}
        {tekening.trapgat && (
          <path d={pad(tekening.trapgat, true)} fill="none" stroke="currentColor" strokeWidth={20} />
        )}
        {tekening.bomen.map((b, i) => (
          <path key={`b${i}`} d={pad(b, true)} fill="#d9c4a3" stroke="#6b4f2a" strokeWidth={6} />
        ))}
        {tekening.palen.map((p, i) => (
          <path key={`p${i}`} d={pad(p, true)} fill="#b08d5b" stroke="#6b4f2a" strokeWidth={6} />
        ))}
        {tekening.treden.map((tr, i) => (
          <path
            key={`t${i}`}
            d={pad(tr, true)}
            fill={i === tekening.treden.length - 1 ? "#ece6da" : "#f6f2ea"}
            stroke="#3f3f46"
            strokeWidth={5}
          />
        ))}
        {r.voorkanten.map((v) => (
          <line
            key={`v${v.nummer}`}
            x1={v.p_binnen[0]}
            y1={-v.p_binnen[1]}
            x2={v.p_buiten[0]}
            y2={-v.p_buiten[1]}
            stroke={v.verdreven ? "var(--theme-accent, #0d6862)" : "#3f3f46"}
            strokeWidth={v.verdreven ? 10 : 6}
          />
        ))}
        {tekening.treden.slice(0, -1).map((tr, i) => {
          const [x, y] = zwaartepunt(tr);
          return (
            <text
              key={`n${i}`}
              x={x}
              y={-y}
              fontSize={TEKST}
              textAnchor="middle"
              dominantBaseline="middle"
              fill="#18181b"
            >
              {i + 1}
            </text>
          );
        })}
        <path
          d={pad(tekening.looplijn, false)}
          fill="none"
          stroke="#2563eb"
          strokeWidth={8}
          strokeDasharray="40 30"
          data-testid="trap-looplijn"
        />
        <path
          d={pad(tekening.klimlijn, false)}
          fill="none"
          stroke="var(--theme-accent, #0d6862)"
          strokeWidth={10}
          data-testid="trap-klimlijn"
        />
        {start && <circle cx={start[0]} cy={-start[1]} r={35} fill="#2563eb" />}
        {/* hoofdmaten van de trap (treden, bomen, palen) */}
        <g stroke="#18181b" strokeWidth={4} fill="#18181b" fontSize={TEKST}>
          <line x1={trap.x0} y1={yMaat} x2={trap.x1} y2={yMaat} />
          <line x1={trap.x0} y1={yMaat - 50} x2={trap.x0} y2={yMaat + 50} />
          <line x1={trap.x1} y1={yMaat - 50} x2={trap.x1} y2={yMaat + 50} />
          <text x={(trap.x0 + trap.x1) / 2} y={yMaat + 110} textAnchor="middle" stroke="none">
            {mm(breedte)}
          </text>
          <line x1={xMaat} y1={-trap.y0} x2={xMaat} y2={-trap.y1} />
          <line x1={xMaat - 50} y1={-trap.y0} x2={xMaat + 50} y2={-trap.y0} />
          <line x1={xMaat - 50} y1={-trap.y1} x2={xMaat + 50} y2={-trap.y1} />
          <text
            x={xMaat + 110}
            y={-(trap.y0 + trap.y1) / 2}
            textAnchor="middle"
            stroke="none"
            transform={`rotate(-90 ${xMaat + 110} ${-(trap.y0 + trap.y1) / 2})`}
          >
            {mm(diepte)}
          </text>
        </g>
      </svg>
      <figcaption className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-on-surface-secondary">
        <span>
          <span className="mr-1 inline-block h-0.5 w-5 bg-[var(--theme-accent,#0d6862)] align-middle" />
          {t("trap.plattegrond.klimlijn", { afstand: tekening.klimlijn_afstand })}
        </span>
        <span>
          <span className="mr-1 inline-block h-0.5 w-5 border-t-2 border-dashed border-[#2563eb] align-middle" />
          {t("trap.plattegrond.looplijn")}
        </span>
        <span>
          <span className="mr-1 inline-block h-0.5 w-5 border-t-2 border-dashed border-[var(--theme-danger-color,#c0392b)] align-middle" />
          {t("trap.plattegrond.trapgatMinimaal")}
        </span>
        <span>{t("trap.plattegrond.verdreven")}</span>
        <span>
          {t("trap.plattegrond.hoofdmaten", { breedte: mm(breedte), diepte: mm(diepte) })}
        </span>
      </figcaption>
    </figure>
  );
}
