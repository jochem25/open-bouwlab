import type { IfcAnalyseStored } from "../lib/ifcAnalyse";

export type RoomFunctionSource = "model" | "naam" | "default";

/** Op naam gekoppeld materiaal (keyword/heuristiek), bewaard voor de checklist. */
export interface IfcMaterialMatch {
  ifc_name: string;
  material_id: string;
  confidence: "keyword" | "heuristic";
}

/**
 * Engine-voorstel voor een materiaalnaam uit `report.materialen[]` van /ifc/analyse.
 * Klein bewaard bij overname (het volledige `report` wordt niet bewaard).
 */
export interface IfcMaterialSuggestion {
  /** Materiaalnaam zoals in het model (report: `naam`). */
  name: string;
  /** Database-id dat de engine verwacht (report: `verwacht_materiaal_id`). */
  expected_material_id?: string;
  /** Voorstel in de materiaalnamen van de engine (report: `voorstel_v2_5`). */
  proposal?: string;
  /** De modelleur zou de naam in het model moeten aanpassen (report: `modelleur_moet_hernoemen`). */
  rename_advised: boolean;
}

export interface IfcImportDefaults {
  heating_system: string;
  ventilation_system: string;
  theta_e: number;
  qv10: number;
  building_type: string;
}

/**
 * Herkomst van een project dat via de IFC-import (tab "IFC-import") is
 * overgenomen, plus de volledige analyse-uitkomst.
 *
 * Bron voor toekomstige ventilatie/BENG-import; niet gebruikt door warmteverlies.
 *
 * Opslag: reist mee in de opslag-envelope (`.ifcenergy`, server-save), maar
 * `analyse` (de complete thermal.json, enkele MB) gaat NIET naar localStorage:
 * na een herstart van de app ontbreekt `analyse` dus tot de IFC opnieuw is
 * geanalyseerd of het project uit een bestand/server wordt geopend.
 */
export interface IfcImportOrigin {
  ifc_filename: string;
  engine_name: string;
  engine_version: string;
  /** ISO 8601 tijdstip van overname. */
  imported_at: string;
  /** Aantal overgenomen ruimten. */
  rooms_count: number;
  /** Thermal-room-id's van de goedgekeurde (overgenomen) ruimten. */
  approved_room_ids: string[];
  /**
   * Waar de functie van elke overgenomen verwarmde ruimte vandaan komt, op het
   * moment van overname. Klein, dus ook in localStorage: de checklist blijft zo
   * na herladen kloppen zonder `analyse`. Ontbreekt bij oudere projecten
   * (de checklist valt dan terug op herleiden uit `analyse`).
   */
  room_function_sources?: Record<string, RoomFunctionSource>;
  /** Idem voor de op naam gekoppelde materialen (keyword/heuristiek). */
  material_matches?: IfcMaterialMatch[];
  /** Engine-voorstellen per materiaalnaam (uit `report.materialen`), bewaard bij overname. */
  material_suggestions?: IfcMaterialSuggestion[];
  /**
   * Projectwaarden op het moment van overname die de import zelf als default
   * invult (niet uit de IFC komen). De checklist toont ze zolang ze
   * ongewijzigd zijn.
   */
  defaults?: IfcImportDefaults;
  /**
   * De 200-response van `/ifc/analyse` (alle ruimten, ook niet-goedgekeurde), zonder
   * `report` en `surfaces_ifc` (grootte; zie {@link IfcAnalyseStored}).
   */
  analyse?: IfcAnalyseStored;
}
