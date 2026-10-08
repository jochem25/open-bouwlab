/**
 * Ruimtefunctie uit het IFC-model (thermal `rooms[].function`, bron
 * `function_source`), met als terugval de naam-keywords en daarna de
 * default.
 *
 * Volgorde (`resolveRoomFunction`):
 *  1. model:   Bbl-/pset-term in de IFC -> `RoomFunction` ({@link MODEL_FUNCTION_MAP});
 *  2. naam:    keywords in de ruimtenaam (`roomFunctionFromName`);
 *  3. default: woonkamer (hoogste temperatuur, veilige kant), gemarkeerd.
 *
 * Gemeten waarden uit 2786 (profiel 2786): "overige gebruiksfunctie" (7),
 * "toiletruimte" (6), "Badruimte" (4), "verkeersruimte" (3),
 * "bijeenkomstfunctie" (3), "bijeenkomst" (2), "afval" (1), "meterruimte" (1),
 * null (2).
 */
import type { Project, RoomFunction } from "../types";
import type { RoomExtra } from "./ifcAnalyse";
import { roomFunctionFromName } from "./roomFunctionFromName";
import type { ThermalRoom } from "./thermalImport";

/** Ruimtefunctie-termen (Bbl/pset) -> `RoomFunction`. Sleutels in kleine letters. */
export const MODEL_FUNCTION_MAP: Record<string, RoomFunction> = {
  toiletruimte: "toilet",
  badruimte: "bathroom",
  badkamer: "bathroom",
  verkeersruimte: "hallway",
  bergruimte: "storage",
  afval: "storage",
  meterruimte: "storage",
  "technische ruimte": "storage",
  keuken: "kitchen",
  kookruimte: "kitchen",
  slaapvertrek: "bedroom",
  slaapruimte: "bedroom",
  slaapkamer: "bedroom",
  verblijfsruimte: "living_room",
  woonkamer: "living_room",
  zolder: "attic",
};

/**
 * GEBRUIKSFUNCTIES (Bbl) zeggen iets over het hele gebouw of de zone, niet over
 * de ruimte: geen ruimtefunctie, dus geen match uit het model (door naar de naam).
 */
export const USAGE_FUNCTIONS: ReadonlySet<string> = new Set([
  "overige gebruiksfunctie",
  "bijeenkomst",
  "bijeenkomstfunctie",
  "kantoorfunctie",
  "woonfunctie",
  "industriefunctie",
  "winkelfunctie",
  "onderwijsfunctie",
  "sportfunctie",
  "logiesfunctie",
  "gezondheidszorgfunctie",
  "celfunctie",
]);

/** `RoomFunction` voor een modelterm (case-insensitief), of null. */
export function roomFunctionFromModel(term: string | null | undefined): RoomFunction | null {
  if (!term) return null;
  const key = term.trim().toLowerCase();
  if (!key || USAGE_FUNCTIONS.has(key)) return null;
  return MODEL_FUNCTION_MAP[key] ?? null;
}

export type FunctionSource = "model" | "naam" | "default";

export interface ResolvedRoomFunction {
  function: RoomFunction;
  source: FunctionSource;
}

export function resolveRoomFunction(
  name: string,
  modelTerm?: string | null,
): ResolvedRoomFunction {
  const fromModel = roomFunctionFromModel(modelTerm);
  if (fromModel) return { function: fromModel, source: "model" };
  const guess = roomFunctionFromName(name);
  return guess.recognized
    ? { function: guess.function, source: "naam" }
    : { function: "living_room", source: "default" };
}

/** Modelterm van een ruimte: thermal `function`, anders `rooms_extra[].function`. */
export function modelTermFor(
  roomId: string,
  thermalRooms: ThermalRoom[],
  roomsExtra: RoomExtra[] | undefined,
): string | null {
  const t = thermalRooms.find((r) => r.id === roomId)?.function;
  if (t) return t;
  return roomsExtra?.find((r) => r.room_id === roomId)?.function ?? null;
}

/**
 * Zet de functie van verwarmde project-ruimten: model, dan naam, dan default.
 * Onverwarmde ruimten houden hun functie. `custom_temperature` blijft
 * ongemoeid (functie-default geldt).
 */
export function applyRoomFunctions(
  project: Project,
  thermalRooms: ThermalRoom[],
  roomsExtra?: RoomExtra[],
): Project {
  const heated = new Set(thermalRooms.filter((r) => r.type === "heated").map((r) => r.id));
  return {
    ...project,
    rooms: project.rooms.map((room) =>
      heated.has(room.id)
        ? {
            ...room,
            function: resolveRoomFunction(
              room.name,
              modelTermFor(room.id, thermalRooms, roomsExtra),
            ).function,
          }
        : room,
    ),
  };
}
