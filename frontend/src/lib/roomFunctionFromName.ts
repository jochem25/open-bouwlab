/**
 * Ruimtefunctie afleiden uit de ruimtenaam (IFC-import).
 *
 * Twee lagen:
 *  - {@link matchRoomFunction}: de oorspronkelijke keyword-mapping van de
 *    modeller-IFC-import (Nederlandse modeller-sleutels, ongewijzigd verplaatst
 *    uit `components/modeller/ifc-import.ts`).
 *  - {@link roomFunctionFromName}: vertaling naar de echte `RoomFunction`-enum
 *    van het project, voor het IFC-tab-overnamepad.
 *
 * "Eerste match wint": de volgorde van {@link FUNCTION_KEYWORDS} is bepalend.
 * "Gang/keuken" wordt dus keuken (22 degC, de veilige kant ten opzichte van
 * gang 20 degC) en "Bijkeuken" wordt eveneens keuken. Een leidend nummer
 * ("13. Toilet") speelt geen rol: er wordt op trefwoorden gezocht.
 */
import type { Project, RoomFunction } from "../types";
import type { ThermalRoom } from "./thermalImport";

export const FUNCTION_KEYWORDS: [RegExp, string][] = [
  [/woonkamer|huiskamer|living|zitkamer/i, "woonkamer"],
  [/slaapkamer|bedroom/i, "slaapkamer"],
  [/keuken|kitchen/i, "keuken"],
  [/badkamer|bathroom/i, "badkamer"],
  [/toilet|wc/i, "toilet"],
  [/hal|gang|entree|corridor|overloop/i, "hal"],
  [/berging|storage|opslag/i, "berging"],
  [/garage/i, "garage"],
  [/kantoor|office|studeerkamer|werkruimte/i, "kantoor"],
  [/wasruimte|laundry|bijkeuken/i, "bijkeuken"],
  [/zolder|attic/i, "zolder"],
  [/kelder|basement|souterrain/i, "kelder"],
];

/** Modeller-sleutel ("woonkamer", "hal", ...) of "custom" bij geen match. */
export function matchRoomFunction(name: string): string {
  for (const [pattern, func] of FUNCTION_KEYWORDS) {
    if (pattern.test(name)) return func;
  }
  return "custom";
}

/** Modeller-sleutel -> project-`RoomFunction`. */
const TO_ROOM_FUNCTION: Record<string, RoomFunction> = {
  woonkamer: "living_room",
  slaapkamer: "bedroom",
  keuken: "kitchen",
  badkamer: "bathroom",
  toilet: "toilet",
  hal: "hallway",
  berging: "storage",
  garage: "storage",
  kantoor: "living_room",
  bijkeuken: "kitchen",
  zolder: "attic",
  kelder: "storage",
};

export interface RoomFunctionGuess {
  function: RoomFunction;
  /** false: geen trefwoord gevonden; `living_room` (hoogste temperatuur) als veilige default. */
  recognized: boolean;
}

export function roomFunctionFromName(name: string): RoomFunctionGuess {
  // Afwijkingen op de modeller-mapping: overloop is een eigen functie
  // (landing) en "leefruimte" is een woonkamer.
  if (/overloop/i.test(name)) return { function: "landing", recognized: true };
  if (/leefruimte/i.test(name)) return { function: "living_room", recognized: true };
  const key = matchRoomFunction(name);
  const fn = TO_ROOM_FUNCTION[key];
  return fn ? { function: fn, recognized: true } : { function: "living_room", recognized: false };
}

/**
 * Zet de functie van verwarmde project-ruimten op basis van de naam.
 * Onverwarmde ruimten houden hun functie (storage) uit `applyEditsToProject`.
 * `custom_temperature` blijft ongemoeid (de import levert `null`, dus de
 * functie-default geldt), net zoals bij een functiewissel in de RoomEditor.
 */
export function applyRoomFunctionsFromNames(
  project: Project,
  thermalRooms: ThermalRoom[],
): Project {
  const heated = new Set(thermalRooms.filter((r) => r.type === "heated").map((r) => r.id));
  return {
    ...project,
    rooms: project.rooms.map((room) =>
      heated.has(room.id)
        ? { ...room, function: roomFunctionFromName(room.name).function }
        : room,
    ),
  };
}
