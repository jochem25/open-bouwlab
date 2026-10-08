/** Herkomst van een project dat via de IFC-import (tab "IFC-import") is overgenomen. */
export interface IfcImportOrigin {
  ifc_filename: string;
  engine_name: string;
  engine_version: string;
  /** ISO 8601 tijdstip van overname. */
  imported_at: string;
  /** Aantal overgenomen ruimten. */
  rooms_count: number;
}
