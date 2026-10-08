import { useMemo, useState } from "react";
import { Link } from "react-router-dom";

import { buildIfcImportChecklist, type ChecklistItem } from "../../lib/ifcImportChecklist";
import { useProjectStore } from "../../store/projectStore";
import { useModellerStore } from "../modeller/modellerStore";

export const DISMISS_KEY = "ifcImport.checklistDismissed";

function readDismissed(): boolean {
  try {
    return typeof sessionStorage !== "undefined" && sessionStorage.getItem(DISMISS_KEY) === "1";
  } catch {
    return false;
  }
}

const LINK_LABEL: Record<string, string> = {
  "/constructies": "Constructies",
  "/rooms": "Vertrekken",
  "/project": "Project",
  "/warmteverlies/instellingen": "Instellingen",
};

const SEVERITY_BOX: Record<ChecklistItem["severity"], string> = {
  ontbreekt: "border-red-400 text-red-400",
  default:
    "border-[var(--theme-warning-border)] bg-[var(--theme-warning-bg)] text-[var(--theme-warning-text)]",
};
const SEVERITY_LABEL: Record<ChecklistItem["severity"], string> = {
  ontbreekt: "Ontbreekt",
  default: "Default",
};

/** Presentatie, los van de store (testbaar via SSR). */
export function ChecklistView({
  items,
  onDismiss,
}: {
  items: ChecklistItem[];
  onDismiss?: () => void;
}) {
  const [open, setOpen] = useState(true);
  if (items.length === 0) return null;
  return (
    <section
      aria-label="Nog in te vullen na IFC-import"
      data-testid="ifc-checklist"
      className="rounded-lg border border-[var(--oaec-border-subtle)] bg-[var(--oaec-bg-lighter)]"
    >
      <div className="flex items-center justify-between px-4 py-2">
        <button
          type="button"
          className="text-sm font-medium text-on-surface"
          aria-expanded={open}
          onClick={() => setOpen((o) => !o)}
        >
          {open ? "▾" : "▸"} Nog in te vullen na IFC-import ({items.length})
        </button>
        {onDismiss && (
          <button
            type="button"
            aria-label="Verberg checklist"
            className="text-on-surface-muted hover:text-on-surface"
            onClick={onDismiss}
          >
            &times;
          </button>
        )}
      </div>
      {open && (
        <ul className="flex flex-col gap-2 px-4 pb-3">
          {items.map((item) => (
            <li
              key={item.id}
              data-testid={`checklist-${item.id}`}
              className={`flex items-start justify-between gap-3 rounded-md border px-3 py-2 text-sm ${SEVERITY_BOX[item.severity]}`}
            >
              <span>
                <strong className="mr-2">{SEVERITY_LABEL[item.severity]}</strong>
                <span className="text-on-surface">{item.tekst}</span>
                {item.details && item.details.length > 0 && (
                  <details className="mt-1 text-xs text-on-surface-secondary">
                    <summary className="cursor-pointer">Toon {item.details.length} koppelingen</summary>
                    <ul className="mt-1 list-disc pl-5">
                      {item.details.map((d) => (
                        <li key={d}>{d}</li>
                      ))}
                    </ul>
                  </details>
                )}
              </span>
              <Link to={item.link} className="shrink-0 underline">
                {LINK_LABEL[item.link] ?? item.link}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** Live checklist uit de projectstore; zichtbaar zolang er een IFC-import is en items open staan. */
export function IfcImportChecklistPanel() {
  const project = useProjectStore((s) => s.project);
  const origin = useProjectStore((s) => s.ifcImport);
  const projectConstructions = useModellerStore((s) => s.projectConstructions);
  const [dismissed, setDismissed] = useState(readDismissed);
  const items = useMemo(() => buildIfcImportChecklist(project, origin, projectConstructions),
    [project, origin, projectConstructions],);

  if (dismissed || !origin) return null;
  return (
    <ChecklistView
      items={items}
      onDismiss={() => {
        try {
          sessionStorage.setItem(DISMISS_KEY, "1");
        } catch {
          // sessionStorage niet beschikbaar: alleen lokaal verbergen.
        }
        setDismissed(true);
      }}
    />
  );
}
