/** Mock-response voor `VITE_IFC_ANALYSE_MOCK=1` en tests. Synthetische data. */
import type { ThermalImportFile } from "../thermalImport";
import type { IfcAnalyseResponse } from "../ifcAnalyse";

import thermal from "./thermal-import-v12-ongelijke-hoogte.json";

export const MOCK_ROOM_HOOG = "room-0000000000000000000008";
export const MOCK_ROOM_LAAG = "room-000000000000000000000A";

export const ifcAnalyseMock: IfcAnalyseResponse = {
  engine: { name: "ifc-ruimtebalans", version: "0.0.0-mock" },
  source_filename: "voorbeeld.ifc",
  thermal: thermal as unknown as ThermalImportFile,
  qc: {
    verdict: "blokkerend",
    findings: [
      {
        severity: "blocking",
        code: "ruimte_niet_gesloten",
        message: "De schil van deze ruimte is niet gesloten; vlakken ontbreken.",
        room_id: MOCK_ROOM_LAAG,
      },
      {
        severity: "warning",
        code: "hoogte_afwijkend",
        message: "De vrije hoogte wijkt af van de buurruimte (2,6 m tegen 2,4 m).",
        room_id: MOCK_ROOM_HOOG,
      },
      {
        severity: "warning",
        code: "materiaal_onbekend",
        message: "Een constructie heeft geen laagopbouw; de U-waarde moet handmatig worden ingevuld.",
        construction_id: "c-53ae51399158737e",
      },
    ],
  },
  stats: { duration_s: 1.2 },
};
