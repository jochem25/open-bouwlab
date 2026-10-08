/**
 * Checklist "Nog in te vullen na IFC-import".
 *
 * Pure functie op de HUIDIGE projectstaat: items verdwijnen vanzelf zodra ze
 * zijn opgelost. Twee severities:
 *  - "ontbreekt": geen veilige default, telt nu als 0 W/K (U-waarde ontbreekt).
 *  - "default":   de import vulde een veilige waarde in; controleer hem.
 */
import type { ProjectConstruction } from "../components/modeller/types";
import type { Project } from "../types";
import type { IfcImportOrigin, IfcMaterialMatch } from "../types/ifcImport";
import {
  BUILDING_TYPE_LABELS,
  HEATING_SYSTEM_LABELS,
  ROOM_FUNCTION_LABELS,
  ROOM_FUNCTION_TEMPERATURES,
  VENTILATION_SYSTEM_LABELS,
} from "./constants";
import { matchIfcMaterial } from "./ifcMaterialMatcher";
import { getMaterialById } from "./materialsDatabase";
import { modelTermFor, resolveRoomFunction } from "./roomFunctionFromModel";
import { roomFunctionFromName } from "./roomFunctionFromName";
import type { ThermalConstruction } from "./thermalImport";

export type ChecklistSeverity = "ontbreekt" | "default";

export interface ChecklistItem {
  id: string;
  severity: ChecklistSeverity;
  tekst: string;
  /** Route van het tabblad waar het item op te lossen is. */
  link: string;
  aantal: number;
  /** Uitklapbare regels (bv. gekoppelde materialen ter controle). */
  details?: string[];
}

const EXTERIOR_TYPES = new Set(["exterior", "ground", "water"]);
const MAX_DESCRIPTIONS = 4;

const label = (map: Record<string, string>, key: string): string => map[key] ?? key;

const fmt1 = (n: number): string =>
  n.toLocaleString("nl-NL", { minimumFractionDigits: 1, maximumFractionDigits: 1 });

/**
 * Een laag rekent als R = 0 wanneer het materiaal niet in de database staat en
 * er geen lambda is meegeleverd (`rcCalculation.ts`: unknown-material pad
 * gebruikt alleen `lambdaOverride`). Dat maakt de U-waarde te hoog.
 */
function unknownMaterialLayers(
  rooms: Project["rooms"],
  projectConstructions: ProjectConstruction[],
): { name: string; area: number; constructions: number }[] {
  const byMaterial = new Map<string, { area: number; ids: Set<string> }>();
  for (const pc of projectConstructions) {
    const unknown = pc.layers.filter(
      (l) => !getMaterialById(l.materialId) && !(l.lambdaOverride !== undefined && l.lambdaOverride > 0),
    );
    if (unknown.length === 0) continue;
    let area = 0;
    for (const room of rooms) {
      for (const ce of room.constructions) {
        if (ce.project_construction_id === pc.id) area += ce.area;
      }
    }
    if (area <= 0) continue;
    for (const l of unknown) {
      const entry = byMaterial.get(l.materialId) ?? { area: 0, ids: new Set<string>() };
      entry.area += area;
      entry.ids.add(pc.id);
      byMaterial.set(l.materialId, entry);
    }
  }
  return [...byMaterial.entries()]
    .map(([name, v]) => ({ name, area: v.area, constructions: v.ids.size }))
    .sort((a, b) => b.area - a.area);
}

/**
 * Materialen in de goedgekeurde ruimten die via keyword/heuristiek aan de
 * database zijn gekoppeld (niet exact, niet onbekend). Bedoeld om bij de
 * overname te bewaren (`ifcImport.material_matches`).
 */
export function computeMaterialMatches(
  thermal: { constructions: ThermalConstruction[] } | undefined,
  approvedIds: string[],
): IfcMaterialMatch[] {
  if (!thermal) return [];
  const approved = new Set(approvedIds);
  const names = new Set<string>();
  for (const c of thermal.constructions) {
    if (!approved.has(c.room_a)) continue;
    for (const l of c.layers ?? []) {
      if (!(typeof l.lambda === "number" && l.lambda > 0) && l.type !== "air_gap") {
        names.add(l.material);
      }
    }
  }
  const out: IfcMaterialMatch[] = [];
  for (const name of [...names].sort()) {
    const m = matchIfcMaterial(name);
    if (!m.material || (m.confidence !== "keyword" && m.confidence !== "heuristic")) continue;
    out.push({ ifc_name: name, material_id: m.material.id, confidence: m.confidence });
  }
  return out;
}

/** "IFC-naam -> materiaal (λ)" per koppeling; onbekende material_id's vallen weg. */
function materialMatchLines(matches: IfcMaterialMatch[]): string[] {
  const lines: string[] = [];
  for (const mm of matches) {
    const material = getMaterialById(mm.material_id);
    if (!material) continue;
    const value =
      material.rdFixed !== null
        ? `Rd ${material.rdFixed.toLocaleString("nl-NL")}`
        : `λ ${material.lambda?.toLocaleString("nl-NL")}`;
    lines.push(`${mm.ifc_name} -> ${material.name} (${value})`);
  }
  return lines;
}

function linkedMaterialLines(origin: IfcImportOrigin): string[] {
  // Bewaarde koppelingen (blijven na herladen); anders herleiden uit `analyse` (oude projecten).
  const matches =
    origin.material_matches ??
    computeMaterialMatches(origin.analyse?.thermal, origin.approved_room_ids);
  return materialMatchLines(matches);
}

export interface MissingU {
  /** Vlakken met catalogus (wanden, vloeren, daken) zonder U-waarde. */
  surfaces: number;
  outsideSurfaces: number;
  /** Openingen (geen catalogus) zonder U-waarde. */
  openings: number;
  outsideOpenings: number;
  /** m² van vlakken en openingen samen. */
  area: number;
  /** m² van alleen de vlakken met catalogus. */
  surfaceArea: number;
  /** Aantal vlakken per catalogus-beschrijving. */
  byDescription: Map<string, number>;
}

/**
 * Elementen met `u_value` 0/leeg en `area > 0`: die tellen als 0 W/K. Geen
 * enkel grenstype mag legitiem U = 0 hebben (ook adjacent_room, ground en water
 * rekenen met de U-waarde van het element); alleen open verbindingen ("-oc",
 * vaste U) en elementen zonder oppervlak worden overgeslagen.
 * Gedeeld door de checklist en de melding op de Resultaten-pagina.
 */
export function findMissingU(rooms: Project["rooms"]): MissingU {
  const out: MissingU = {
    surfaces: 0,
    outsideSurfaces: 0,
    openings: 0,
    outsideOpenings: 0,
    area: 0,
    surfaceArea: 0,
    byDescription: new Map(),
  };
  for (const room of rooms) {
    for (const ce of room.constructions) {
      if (ce.u_value > 0 || !(ce.area > 0) || ce.id.includes("-oc")) continue;
      const outside = EXTERIOR_TYPES.has(ce.boundary_type);
      out.area += ce.area;
      if (ce.catalog_ref) {
        out.surfaces += 1;
        out.surfaceArea += ce.area;
        if (outside) out.outsideSurfaces += 1;
        const label = ce.description || ce.catalog_ref;
        out.byDescription.set(label, (out.byDescription.get(label) ?? 0) + 1);
      } else {
        out.openings += 1;
        if (outside) out.outsideOpenings += 1;
      }
    }
  }
  return out;
}

export function buildIfcImportChecklist(
  project: Project,
  origin: IfcImportOrigin | null | undefined,
  projectConstructions: ProjectConstruction[] = [],
): ChecklistItem[] {
  if (!origin) return [];
  const approved = new Set(origin.approved_room_ids);
  const rooms = project.rooms.filter((r) => approved.has(r.id));
  const items: ChecklistItem[] = [];

  // --- U-waarden ontbreken ------------------------------------------------
  const missing = findMissingU(rooms);
  const { surfaces, outsideSurfaces, openings, outsideOpenings, byDescription } = missing;
  const area = missing.surfaceArea;
  if (surfaces > 0) {
    const geenOpbouw = (origin.analyse?.qc.findings ?? [])
      .filter((f) => f.code === "L-GEEN-OPBOUW")
      .reduce((n, f) => n + (f.count ?? 1), 0);
    const top = [...byDescription.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, MAX_DESCRIPTIONS)
      .map(([d, n]) => `${d} (${n})`)
      .join(", ");
    items.push({
      id: "u-constructies",
      severity: "ontbreekt",
      aantal: surfaces,
      link: "/constructies",
      tekst:
        `${surfaces} vlakken (${fmt1(area)} m², waarvan ${outsideSurfaces} aan buiten/grond/water) ` +
        `hebben geen U-waarde en tellen nu als 0 W/K: ${top}.` +
        (geenOpbouw > 0
          ? ` De IFC-analyse meldt ${geenOpbouw}× bouwdeel zonder laagopbouw (L-GEEN-OPBOUW): vul de lagen in het model aan.`
          : ""),
    });
  }
  if (openings > 0) {
    items.push({
      id: "u-openingen",
      severity: "ontbreekt",
      aantal: openings,
      link: "/constructies",
      tekst:
        `${openings} openingen (ramen/deuren, waarvan ${outsideOpenings} aan buiten/grond/water) ` +
        `hebben geen U-waarde en tellen nu als 0 W/K.`,
    });
  }

  // --- Lagen met onbekend materiaal en zonder lambda -----------------------
  const unknown = unknownMaterialLayers(rooms, projectConstructions);
  if (unknown.length > 0) {
    const list = unknown
      .map((u) => `${u.name} (${fmt1(u.area)} m², ${u.constructions} constructies)`)
      .join(", ");
    items.push({
      id: "materiaal-onbekend",
      severity: "ontbreekt",
      aantal: unknown.length,
      link: "/constructies",
      tekst: `λ onbekend, laag telt als R = 0 — U te hoog: ${list}.`,
    });
  }
  const linked = linkedMaterialLines(origin);
  if (linked.length > 0) {
    items.push({
      id: "materiaal-gekoppeld",
      severity: "default",
      aantal: linked.length,
      link: "/constructies",
      details: linked,
      tekst: `${linked.length} materialen zijn op naam gekoppeld aan de database. Controleer de gekozen λ.`,
    });
  }

  // --- Vlakken naar niet-overgenomen ruimten -----------------------------------
  const projectRoomIds = new Set(project.rooms.map((r) => r.id));
  let orphaned = 0;
  for (const room of rooms) {
    for (const ce of room.constructions) {
      if (ce.catalog_ref && ce.adjacent_room_id && !projectRoomIds.has(ce.adjacent_room_id)) {
        orphaned += 1;
      }
    }
  }
  if (orphaned > 0) {
    items.push({
      id: "wees-vlakken",
      severity: "default",
      aantal: orphaned,
      link: "/rooms",
      tekst: `${orphaned} vlakken grenzen aan niet-overgenomen ruimten en tellen als 0 W/K.`,
    });
  }

  // --- Ruimtefunctie: model -> naam -> default --------------------------------
  const thermalRooms = origin.analyse?.thermal.rooms ?? [];
  const roomsExtra = origin.analyse?.rooms_extra;
  let fromModel = 0;
  let fromName = 0;
  let unrecognized = 0;
  for (const room of rooms) {
    // Onverwarmde ruimten krijgen bij import vast "storage"; geen schatting.
    const type = thermalRooms.find((r) => r.id === room.id)?.type;
    if (type !== undefined && type !== "heated") continue;
    // Bewaarde bron (blijft na herladen); anders herleiden uit `analyse` (oude projecten).
    const stored = origin.room_function_sources;
    if (stored && !(room.id in stored)) continue; // onverwarmd of niet geclassificeerd
    const source =
      stored?.[room.id] ??
      resolveRoomFunction(room.name, modelTermFor(room.id, thermalRooms, roomsExtra)).source;
    if (source === "default") {
      if (room.function === "living_room" && room.custom_temperature == null) unrecognized += 1;
    } else if (source === "naam") {
      // Opgelost zodra de functie niet meer de naam-schatting is.
      if (room.function === roomFunctionFromName(room.name).function) fromName += 1;
    } else if (stored) {
      // Uit het model: zonder analyse is de oorspronkelijke waarde niet te herleiden;
      // het item blijft staan tot het paneel wordt weggeklikt.
      fromModel += 1;
    } else if (
      room.function ===
      resolveRoomFunction(room.name, modelTermFor(room.id, thermalRooms, roomsExtra)).function
    ) {
      fromModel += 1;
    }
  }
  const livingTemp = ROOM_FUNCTION_TEMPERATURES["living_room"] ?? 22;
  if (unrecognized > 0) {
    items.push({
      id: "functie-onbekend",
      severity: "default",
      aantal: unrecognized,
      link: "/rooms",
      tekst:
        `${unrecognized} vertrekken: functie niet herkend; staat op ` +
        `${ROOM_FUNCTION_LABELS["living_room"] ?? "woonkamer"} (${livingTemp} °C, default). Kies de juiste functie.`,
    });
  }
  if (fromName > 0) {
    items.push({
      id: "functie-geschat",
      severity: "default",
      aantal: fromName,
      link: "/rooms",
      tekst: `${fromName} vertrekken: functie geschat uit de naam. Controleer functie en temperatuur.`,
    });
  }
  if (fromModel > 0) {
    items.push({
      id: "functie-model",
      severity: "default",
      aantal: fromModel,
      link: "/rooms",
      tekst: `${fromModel} vertrekken: functie uit het model overgenomen. Controleer functie en temperatuur.`,
    });
  }

  // --- Projectdefaults -------------------------------------------------------
  const d = origin.defaults;
  if (d) {
    const onHeatingDefault = rooms.filter((r) => r.heating_system === d.heating_system).length;
    if (onHeatingDefault > 0) {
      items.push({
        id: "verwarming",
        severity: "default",
        aantal: onHeatingDefault,
        link: "/rooms",
        tekst: `${onHeatingDefault} vertrekken gebruiken het standaard verwarmingssysteem (${label(HEATING_SYSTEM_LABELS, d.heating_system)}).`,
      });
    }
    if (project.ventilation.system_type === d.ventilation_system) {
      items.push({
        id: "ventilatie",
        severity: "default",
        aantal: 1,
        link: "/warmteverlies/instellingen",
        tekst: `Ventilatiesysteem staat op de default (${label(VENTILATION_SYSTEM_LABELS, d.ventilation_system)}).`,
      });
    }
    if (
      project.climate.theta_e === d.theta_e &&
      project.building.qv10 === d.qv10 &&
      project.building.building_type === d.building_type
    ) {
      items.push({
        id: "klimaat",
        severity: "default",
        aantal: 1,
        link: "/project",
        tekst:
          `Klimaat en gebouw staan op de projectdefaults (θe ${d.theta_e} °C, ` +
          `${label(BUILDING_TYPE_LABELS, d.building_type)}` +
          (d.qv10 === 0
            ? `). qv10 is niet ingevuld: 0 rekent met de gunstigste luchtdichtheidsklasse (≤ 50 dm³/s).`
            : `, qv10 ${d.qv10}).`),
      });
    }
  }

  return items;
}
