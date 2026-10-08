/** Presentatie-helpers voor de IFC-import-pagina (geen React). */
import type { QcFinding } from "./ifcAnalyse";
import type {
  ThermalConstruction,
  ThermalImportFile,
  ThermalOpening,
  ThermalRoom,
} from "./thermalImport";
import { isPseudoRoom } from "./thermalImport";

/** Echte ruimten (verwarmd/onverwarmd) — geen outside/ground/water. */
export function realRooms(file: ThermalImportFile): ThermalRoom[] {
  return file.rooms.filter((r) => !isPseudoRoom(r));
}

export function roomVolume(room: ThermalRoom): number | undefined {
  if (room.volume_m3 != null) return room.volume_m3;
  if (room.area_m2 != null && room.height_m != null) return room.area_m2 * room.height_m;
  return undefined;
}

export function totalFloorArea(rooms: ThermalRoom[]): number {
  return rooms.reduce((s, r) => s + (r.area_m2 ?? 0), 0);
}

/** Schilsom: som gross_area van vlakken naar buiten/grond/water/onverwarmd (vanuit room_a). */
export function envelopeArea(file: ThermalImportFile): number {
  const byId = new Map(file.rooms.map((r) => [r.id, r]));
  let sum = 0;
  for (const c of file.constructions) {
    const other = byId.get(c.room_b);
    if (!other || other.type === "heated") continue;
    sum += c.gross_area_m2;
  }
  return sum;
}

type Orientation = ThermalConstruction["orientation"];

/** Zelfde regel als `ThermalOrientation::mirrored` in thermal.rs. */
export function mirrorOrientation(o: Orientation): Orientation {
  if (o === "floor") return "ceiling";
  if (o === "ceiling" || o === "roof") return "floor";
  return "wall";
}

export interface RoomSurface {
  construction: ThermalConstruction;
  /** Soort vlak gezien vanuit deze ruimte. */
  orientation: Orientation;
  otherRoomId: string;
  openings: ThermalOpening[];
}

/**
 * Vlakken van een ruimte zoals de import ze ziet: room_a-kant ongewijzigd,
 * room_b-kant gespiegeld (vloer<->plafond) tenzij het een v1.2-paarkant is
 * (pair_id, beide ruimten echt): dan levert de andere ruimte zijn eigen kant.
 */
export function roomSurfaces(file: ThermalImportFile, roomId: string): RoomSurface[] {
  const byId = new Map(file.rooms.map((r) => [r.id, r]));
  const isReal = (id: string) => {
    const r = byId.get(id);
    return !!r && !isPseudoRoom(r);
  };
  const openingsBy = new Map<string, ThermalOpening[]>();
  for (const o of file.openings ?? []) {
    const list = openingsBy.get(o.construction_id) ?? [];
    list.push(o);
    openingsBy.set(o.construction_id, list);
  }
  const out: RoomSurface[] = [];
  for (const c of file.constructions) {
    const paired =
      file.version === "1.2" && !!c.pair_id && isReal(c.room_a) && isReal(c.room_b);
    if (c.room_a === roomId) {
      out.push({
        construction: c,
        orientation: c.orientation,
        otherRoomId: c.room_b,
        openings: openingsBy.get(c.id) ?? [],
      });
    } else if (c.room_b === roomId && !paired) {
      out.push({
        construction: c,
        orientation: isReal(c.room_a) ? mirrorOrientation(c.orientation) : c.orientation,
        otherRoomId: c.room_a,
        openings: openingsBy.get(c.id) ?? [],
      });
    }
  }
  return out;
}

export const ORIENTATION_LABEL: Record<Orientation, string> = {
  wall: "Wand",
  floor: "Vloer",
  ceiling: "Plafond",
  roof: "Dak",
};

/** "buiten" / "grond" / ... of de naam van de buurruimte. */
export function adjacentLabel(file: ThermalImportFile, otherRoomId: string): string {
  const r = file.rooms.find((x) => x.id === otherRoomId);
  if (!r) return "onbekend";
  switch (r.type) {
    case "outside":
      return "Buiten";
    case "ground":
      return "Grond";
    case "water":
      return "Water";
    case "unheated":
      return `Onverwarmd: ${r.name}`;
    default:
      return r.name;
  }
}

export function openingArea(o: ThermalOpening): number {
  return (o.width_mm * o.height_mm) / 1_000_000;
}

export function findingsForRoom(findings: QcFinding[], roomId: string): QcFinding[] {
  return findings.filter((f) => f.room_id === roomId);
}

export function hasBlocking(findings: QcFinding[], roomId: string): boolean {
  return findingsForRoom(findings, roomId).some((f) => f.severity === "blocking");
}

/**
 * Standaard goedkeuring: alles aangevinkt, behalve ruimten met een blokkerende
 * bevinding. Reden: een blokkerende bevinding betekent dat de ruimte
 * aantoonbaar niet klopt; die moet de gebruiker bewust aanvinken. Niets
 * aanvinken zou elke import een klik-karwei maken.
 */
export function defaultApproved(file: ThermalImportFile, findings: QcFinding[]): Set<string> {
  return new Set(
    realRooms(file)
      .filter((r) => !hasBlocking(findings, r.id))
      .map((r) => r.id),
  );
}
