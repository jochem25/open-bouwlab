/**
 * Koppel een onbekende materiaalnaam aan een databasemateriaal voor ALLE lagen
 * met die naam, en herbereken de U-waarde van de getroffen vlakken.
 *
 * Zelfde route als de Rc-calculator bij het bewerken van een projectconstructie:
 * lagen aanpassen in de modellerStore, daarna `syncProjectConstruction` in de
 * projectStore met de opnieuw berekende U (`getProjectConstructionUValue`).
 */
import { useModellerStore } from "../components/modeller/modellerStore";
import { getProjectConstructionUValue } from "../components/modeller/projectConstructionUtils";
import { useProjectStore } from "../store/projectStore";

/** Geeft het aantal gewijzigde projectconstructies terug. */
export function remapUnknownMaterial(fromId: string, toId: string): number {
  if (fromId === toId) return 0;
  const changed = useModellerStore.getState().remapLayerMaterial(fromId, toId);
  const project = useProjectStore.getState();
  for (const pcId of changed) {
    const pc = useModellerStore.getState().projectConstructions.find((c) => c.id === pcId);
    if (!pc) continue;
    project.syncProjectConstruction(pcId, {
      description: pc.name,
      u_value: getProjectConstructionUValue(pc),
      material_type: pc.materialType,
      vertical_position: pc.verticalPosition,
      layers: pc.layers.map((l) => ({
        materialId: l.materialId,
        thickness: l.thickness,
        lambdaOverride: l.lambdaOverride,
        stud: l.stud,
      })),
    });
  }
  return changed.length;
}
