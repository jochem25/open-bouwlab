/**
 * Bestemmingen van een IFC-import (projectniveau, Jochem 12-10-2026): na de
 * analyse kiest de gebruiker waarheen het resultaat gaat. Nu Warmteverlies
 * (thermal -> vertrekken) en BENG (beng-import -> project.beng_geometry).
 *
 * Uitbreiden (bv. ventilatie): een id aan {@link ImportDestinationId} en een
 * entry in {@link IMPORT_DESTINATIONS} met eigen `status`; de UI en de
 * overname-afhandeling lopen over deze lijst, niet over vaste namen.
 */
import type { BengCheck } from "./bengImport";
import type { FilteredImport } from "./ifcImportFilter";
import { findMissingU } from "./ifcImportChecklist";
import type { IfcImportOrigin } from "../types/ifcImport";

export type ImportDestinationId = "warmteverlies" | "beng";

export type DestinationState =
  | "compleet"
  | "onvolledig"
  | "te_controleren"
  | "niet_beschikbaar"
  | "bezig"
  | "fout";

export interface DestinationStatus {
  state: DestinationState;
  /** Een regel voor naast het keuzevak. */
  summary: string;
  /** Wat nog ontbreekt (ontbrekende waarden, paden). */
  ontbrekend: string[];
  /** Overige meldingen. */
  meldingen: string[];
}

/** Alles wat een bestemming nodig heeft om haar status te bepalen. */
export interface DestinationContext {
  filtered: FilteredImport;
  approvedCount: number;
  blockingCount: number;
  /** null: de analyse had geen BENG-uitvoer. */
  bengCheck: BengCheck | null;
}

export interface ImportDestination {
  id: ImportDestinationId;
  label: string;
  /** Waar de gebruiker na overname heen gaat als dit de eerste gekozen bestemming is. */
  route: string;
  status: (ctx: DestinationContext) => DestinationStatus;
}

/** Kan deze status overgenomen worden? */
export function isSelectable(s: DestinationStatus): boolean {
  return s.state === "compleet" || s.state === "onvolledig" || s.state === "te_controleren";
}

const MAX_LIST = 10;

function capped(list: string[]): string[] {
  return list.length > MAX_LIST
    ? [...list.slice(0, MAX_LIST), `en ${list.length - MAX_LIST} meer`]
    : list;
}

function warmteverliesStatus(ctx: DestinationContext): DestinationStatus {
  if (ctx.approvedCount === 0) {
    return { state: "niet_beschikbaar", summary: "Geen ruimten goedgekeurd.", ontbrekend: [], meldingen: [] };
  }
  const missing = findMissingU(ctx.filtered.importResult.project.rooms);
  const n = missing.surfaces + missing.openings;
  const meldingen =
    ctx.blockingCount > 0 ? [`${ctx.blockingCount} blokkerende bevinding(en) in de goedgekeurde ruimten`] : [];
  if (n > 0) {
    return {
      state: "onvolledig",
      summary: `${ctx.approvedCount} ruimten; ${n} vlak(ken) zonder U-waarde (tellen als 0 W/K).`,
      ontbrekend: capped([...missing.byDescription].map(([d, c]) => `U-waarde: ${d} (${c} vlakken)`)
        .concat(missing.openings > 0 ? [`U-waarde: ${missing.openings} opening(en)`] : [])),
      meldingen,
    };
  }
  return { state: "compleet", summary: `${ctx.approvedCount} ruimten, alle U-waarden bekend.`, ontbrekend: [], meldingen };
}

function bengStatus(ctx: DestinationContext): DestinationStatus {
  const c = ctx.bengCheck;
  if (!c) {
    return {
      state: "niet_beschikbaar",
      summary: "Geen BENG-uitvoer in deze analyse: upload opnieuw met 'Ook BENG-schil'.",
      ontbrekend: [],
      meldingen: [],
    };
  }
  if (c.phase === "loading") return { state: "bezig", summary: "BENG-import controleren...", ontbrekend: [], meldingen: [] };
  if (c.phase === "error") return { state: "fout", summary: c.message, ontbrekend: [], meldingen: [] };
  const r = c.result;
  const zones = r.beng_geometry?.zones ?? [];
  const vlakken = zones.reduce((n, z) => n + (z.gevels?.length ?? 0), 0);
  if (r.status === "onvolledig") {
    return {
      state: "onvolledig",
      summary: `${r.ontbrekend.length} waarde(n) ontbreken (Rc/U of U_w); niets wordt ingevuld.`,
      ontbrekend: capped(r.ontbrekend),
      meldingen: r.meldingen,
    };
  }
  return {
    state: r.te_controleren ? "te_controleren" : "compleet",
    summary: `${zones.length} rekenzone(s), ${vlakken} vlakken${r.te_controleren ? "; te controleren (IFC-route nog niet geijkt)" : ""}.`,
    ontbrekend: [],
    meldingen: r.meldingen,
  };
}

export const IMPORT_DESTINATIONS: ReadonlyArray<ImportDestination> = [
  { id: "warmteverlies", label: "Warmteverlies (vertrekken)", route: "/rooms", status: warmteverliesStatus },
  { id: "beng", label: "BENG (geometrie NTA 8800)", route: "/beng", status: bengStatus },
];

/**
 * Overnemen naar BENG kan alleen met een compleet document: bij onvolledig vult
 * de import niets in (contract beng-import v1). Warmteverlies mag ook onvolledig
 * (vlakken zonder U tellen als 0 W/K, zoals tot nu toe).
 */
export function canTakeOver(id: ImportDestinationId, s: DestinationStatus): boolean {
  if (!isSelectable(s)) return false;
  return id === "beng" ? s.state !== "onvolledig" : true;
}

/** Begin-keuze: elke bestemming die over te nemen is. */
export function defaultSelection(statuses: Record<ImportDestinationId, DestinationStatus>): Set<ImportDestinationId> {
  return new Set(
    IMPORT_DESTINATIONS.filter((d) => canTakeOver(d.id, statuses[d.id])).map((d) => d.id),
  );
}

/** Route na overname: de eerste gekozen bestemming in registervolgorde. */
export function routeAfterImport(chosen: ReadonlySet<ImportDestinationId>): string {
  return IMPORT_DESTINATIONS.find((d) => chosen.has(d.id))?.route ?? "/project";
}

/**
 * Bestemmingen van een opgeslagen import. Projecten van voor 12-10-2026 kennen
 * het veld niet: hun import ging altijd naar Warmteverlies (migratie zonder
 * dataverlies; de rest van de herkomst blijft onaangeroerd).
 */
export function originDestinations(origin: IfcImportOrigin | null | undefined): ImportDestinationId[] {
  if (!origin) return [];
  if (Array.isArray(origin.destinations)) {
    const known = new Set(IMPORT_DESTINATIONS.map((d) => d.id));
    return origin.destinations.filter((d): d is ImportDestinationId => known.has(d as ImportDestinationId));
  }
  return ["warmteverlies"];
}
