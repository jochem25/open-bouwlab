import type { IfcAnalyseResponse } from "../lib/ifcAnalyse";

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
   * Projectwaarden op het moment van overname die de import zelf als default
   * invult (niet uit de IFC komen). De checklist toont ze zolang ze
   * ongewijzigd zijn.
   */
  defaults?: IfcImportDefaults;
  /** De ongewijzigde 200-response van `/ifc/analyse` (alle ruimten, ook niet-goedgekeurde). */
  analyse?: IfcAnalyseResponse;
}
