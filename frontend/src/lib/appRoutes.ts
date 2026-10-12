/**
 * Routes die op meer plekken nodig zijn (navigatie, ribbon, redirects).
 *
 * IFC-import staat sinds 12-10-2026 op projectniveau (Jochem): het resultaat
 * gaat naar Warmteverlies en/of BENG. Oude paden blijven werken via een
 * redirect, zodat bladwijzers en links niet dood lopen.
 */
export const IFC_IMPORT_ROUTE = "/project/ifc-import";

export const LEGACY_REDIRECTS: ReadonlyArray<{ from: string; to: string }> = [
  { from: "/ifc-import", to: IFC_IMPORT_ROUTE },
  { from: "/ifc-reconstructie", to: IFC_IMPORT_ROUTE },
];
