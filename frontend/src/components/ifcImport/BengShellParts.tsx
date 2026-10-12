/**
 * Onderdelen van de BENG-schilweergave in de IFC-import-viewer, los van three
 * zodat ze in de SSR-tests renderen: schakelaar, legenda en vlakinfo.
 */
import type { LegendEntry, ViewerMode } from "../../lib/bengShell";
import { vlakInfoRows, type BengVlakInfo } from "../../lib/bengShell";

const MODES: Array<{ mode: ViewerMode; label: string }> = [
  { mode: "warmteverlies", label: "Warmteverlies" },
  { mode: "beng", label: "BENG-schil" },
];

export function ViewerModeSwitch({
  mode,
  bengAvailable,
  onChange,
}: {
  mode: ViewerMode;
  /** false: geen BENG-schilmodel in deze analyse (knop uitgeschakeld met uitleg). */
  bengAvailable: boolean;
  onChange: (mode: ViewerMode) => void;
}) {
  return (
    <div
      role="radiogroup"
      aria-label="Weergave"
      data-testid="viewer-mode-switch"
      className="inline-flex overflow-hidden rounded border border-[var(--oaec-border-subtle)] text-xs"
    >
      {MODES.map(({ mode: m, label }) => {
        const disabled = m === "beng" && !bengAvailable;
        const active = mode === m;
        return (
          <button
            key={m}
            type="button"
            role="radio"
            aria-checked={active}
            disabled={disabled}
            data-mode={m}
            title={
              disabled
                ? "Geen BENG-schilmodel in deze analyse: upload opnieuw met 'Ook BENG-schil'."
                : undefined
            }
            onClick={() => onChange(m)}
            className={`px-2.5 py-1 ${
              active ? "bg-primary text-on-accent font-semibold" : "bg-surface-alt hover:underline"
            } ${disabled ? "cursor-not-allowed opacity-50" : ""}`}
          >
            {label}
          </button>
        );
      })}
    </div>
  );
}

const fmt = new Intl.NumberFormat("nl-NL", { maximumFractionDigits: 1 });

export function BengLegend({ entries, mixedColors }: { entries: LegendEntry[]; mixedColors: string[] }) {
  return (
    <div className="flex flex-col gap-0.5" data-testid="beng-legend">
      {entries.map((e) => (
        <div key={e.key} className="flex items-center gap-1.5" data-key={e.key}>
          <span className="h-2 w-3 rounded-sm" style={{ backgroundColor: e.color }} />
          {e.extraColors?.map((c) => (
            <span
              key={c}
              className="-ml-1 h-2 w-3 rounded-sm"
              style={{ backgroundColor: c }}
              title="Deel van een element in een andere kleur (bv. magenta: belendingsdeel van een deels bedekte wand)"
            />
          ))}
          <span>{e.label}</span>
          <span className="text-on-surface-muted">
            ({e.count}
            {e.a_t_m2 > 0 ? `, ${fmt.format(e.a_t_m2)} m2` : ""}
            {e.merken ? `, ${e.merken} kozijnmerken, kleur per merk` : ""})
          </span>
        </div>
      ))}
      {mixedColors.length > 0 && (
        <div className="mt-1 text-amber-500">Meer kleuren binnen: {mixedColors.join(", ")}</div>
      )}
    </div>
  );
}

export function BengVlakInfoPanel({
  info,
  name,
  onClose,
}: {
  info: BengVlakInfo | null;
  name: string;
  onClose?: () => void;
}) {
  const rows = vlakInfoRows(info, name);
  return (
    <div data-testid="beng-vlakinfo" className="rounded border border-[var(--oaec-border-subtle)] p-2 text-xs">
      <div className="mb-1 flex items-center justify-between">
        <span className="font-semibold">Vlakinfo BENG-schil</span>
        {onClose && (
          <button type="button" className="hover:underline" onClick={onClose}>
            Sluiten
          </button>
        )}
      </div>
      <table className="w-full">
        <tbody>
          {rows.map(([k, v]) => (
            <tr key={k}>
              <th scope="row" className="pr-3 text-left align-top font-normal text-on-surface-muted">
                {k}
              </th>
              <td className="break-words">{v}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
