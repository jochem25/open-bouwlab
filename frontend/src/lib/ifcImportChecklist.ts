/**
 * Checklist "Nog in te vullen na IFC-import".
 *
 * Pure functie op de HUIDIGE projectstaat: items verdwijnen vanzelf zodra ze
 * zijn opgelost. Twee severities:
 *  - "ontbreekt": geen veilige default, telt nu als 0 W/K (U-waarde ontbreekt).
 *  - "default":   de import vulde een veilige waarde in; controleer hem.
 */
import type { Project } from "../types";
import type { IfcImportOrigin } from "../types/ifcImport";
import { ROOM_FUNCTION_LABELS, ROOM_FUNCTION_TEMPERATURES } from "./constants";
import { roomFunctionFromName } from "./roomFunctionFromName";

export type ChecklistSeverity = "ontbreekt" | "default";

export interface ChecklistItem {
  id: string;
  severity: ChecklistSeverity;
  tekst: string;
  /** Route van het tabblad waar het item op te lossen is. */
  link: string;
  aantal: number;
}

const EXTERIOR_TYPES = new Set(["exterior", "ground", "water"]);
const MAX_DESCRIPTIONS = 4;

const fmt1 = (n: number): string =>
  n.toLocaleString("nl-NL", { minimumFractionDigits: 1, maximumFractionDigits: 1 });

export function buildIfcImportChecklist(
  project: Project,
  origin: IfcImportOrigin | null | undefined,
): ChecklistItem[] {
  if (!origin) return [];
  const approved = new Set(origin.approved_room_ids);
  const rooms = project.rooms.filter((r) => approved.has(r.id));
  const items: ChecklistItem[] = [];

  // --- U-waarden ontbreken ------------------------------------------------
  const byDescription = new Map<string, number>();
  let surfaces = 0;
  let outsideSurfaces = 0;
  let area = 0;
  let openings = 0;
  let outsideOpenings = 0;
  for (const room of rooms) {
    for (const ce of room.constructions) {
      if (ce.u_value > 0) continue;
      const outside = EXTERIOR_TYPES.has(ce.boundary_type);
      if (ce.catalog_ref) {
        surfaces += 1;
        if (outside) outsideSurfaces += 1;
        area += ce.area;
        const label = ce.description || ce.catalog_ref;
        byDescription.set(label, (byDescription.get(label) ?? 0) + 1);
      } else if (!ce.id.includes("-oc")) {
        // Openingen (geen catalogus). Open verbindingen ("-oc") hebben een vaste U.
        openings += 1;
        if (outside) outsideOpenings += 1;
      }
    }
  }
  if (surfaces > 0) {
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
        `hebben geen U-waarde en tellen nu als 0 W/K: ${top}.`,
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

  // --- Ruimtefunctie --------------------------------------------------------
  const thermalType = new Map(
    (origin.analyse?.thermal.rooms ?? []).map((r) => [r.id, r.type]),
  );
  let guessed = 0;
  let unrecognized = 0;
  for (const room of rooms) {
    // Onverwarmde ruimten krijgen bij import vast "storage"; geen naam-schatting.
    const type = thermalType.get(room.id);
    if (type !== undefined && type !== "heated") continue;
    const guess = roomFunctionFromName(room.name);
    if (guess.recognized && room.function === guess.function) guessed += 1;
    if (!guess.recognized && room.function === "living_room" && room.custom_temperature == null) {
      unrecognized += 1;
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
        `${unrecognized} vertrekken: functie niet herkend uit de naam; staat op ` +
        `${ROOM_FUNCTION_LABELS["living_room"] ?? "woonkamer"} (${livingTemp} °C). Kies de juiste functie.`,
    });
  }
  if (guessed > 0) {
    items.push({
      id: "functie-geschat",
      severity: "default",
      aantal: guessed,
      link: "/rooms",
      tekst: `${guessed} vertrekken: functie geschat uit de naam. Controleer functie en temperatuur.`,
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
        tekst: `${onHeatingDefault} vertrekken gebruiken het standaard verwarmingssysteem (${d.heating_system}).`,
      });
    }
    if (project.ventilation.system_type === d.ventilation_system) {
      items.push({
        id: "ventilatie",
        severity: "default",
        aantal: 1,
        link: "/warmteverlies/instellingen",
        tekst: `Ventilatiesysteem staat op de default (${d.ventilation_system}).`,
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
          `Klimaat en gebouw staan op de projectdefaults (θe ${d.theta_e} °C, qv10 ${d.qv10}, ` +
          `${d.building_type}).`,
      });
    }
  }

  return items;
}
