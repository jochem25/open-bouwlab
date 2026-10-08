import type { IfcImportOrigin } from "../../types/ifcImport";

/** Korte herkomstregel: "Overgenomen uit <bestand> (<engine> <versie>), <n> ruimten, <datum>". */
export function IfcImportOriginNote({ origin }: { origin: IfcImportOrigin }) {
  const date = new Date(origin.imported_at);
  const when = Number.isNaN(date.getTime()) ? origin.imported_at : date.toLocaleString("nl-NL");
  return (
    <p
      data-testid="ifc-origin"
      className="rounded-md border border-[var(--oaec-border-subtle)] px-4 py-2 text-sm text-on-surface-secondary"
    >
      Herkomst: {origin.rooms_count} ruimten overgenomen uit{" "}
      <span className="font-medium text-on-surface">{origin.ifc_filename}</span> (
      {origin.engine_name} {origin.engine_version}), {when}.
    </p>
  );
}
