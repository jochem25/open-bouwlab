/**
 * Mock-response voor `VITE_IFC_ANALYSE_MOCK=1` en tests: byte-kopie van
 * `analyse_200_v12_fixture.json` uit het sidecar-contract (synthetisch), plus
 * `source_filename` (dat zet onze API).
 */
import type { IfcAnalyseResponse } from "../ifcAnalyse";

import fixture from "./analyse_200_v12_fixture.json";

export const MOCK_ROOM_HOOG = "room-0000000000000000000008";
export const MOCK_ROOM_LAAG = "room-000000000000000000000A";

export const ifcAnalyseMock = {
  ...fixture,
  source_filename: "voorbeeld.ifc",
} as unknown as IfcAnalyseResponse;
