/**
 * Varianten van de contract-fixture met extra bevindingen. Testdata, bewust
 * niet in de mock zelf: de mock is een byte-kopie van het sidecar-contract.
 */
import type { IfcAnalyseResponse, QcFinding } from "../ifcAnalyse";

import { ifcAnalyseMock, MOCK_ROOM_HOOG, MOCK_ROOM_LAAG } from "./ifcAnalyseMock";

export function finding(
  partial: Pick<QcFinding, "severity" | "code" | "message"> & Partial<QcFinding>,
): QcFinding {
  return { count: null, modeller_action: false, ...partial };
}

export function withFindings(base: IfcAnalyseResponse, extra: QcFinding[]): IfcAnalyseResponse {
  const findings = [...base.qc.findings, ...extra];
  const count = (s: QcFinding["severity"]) => findings.filter((f) => f.severity === s).length;
  const verdict =
    count("blocking") > 0 ? "blokkerend" : count("warning") > 0 ? "waarschuwing" : "ok";
  return {
    ...base,
    qc: {
      ...base.qc,
      verdict,
      counts: { blocking: count("blocking"), warning: count("warning"), info: count("info") },
      findings,
    },
  };
}

/** Contract-fixture + 1 blokkerende bevinding (ruimte "laag") + 2 waarschuwingen. */
export const ifcAnalyseBlocking: IfcAnalyseResponse = withFindings(ifcAnalyseMock, [
  finding({
    severity: "blocking",
    code: "L-GEEN-OPBOUW",
    message: "Bouwdeel zonder laagopbouw.",
    count: 3,
    modeller_action: true,
    room_id: MOCK_ROOM_LAAG,
  }),
  finding({
    severity: "warning",
    code: "M-HOOGTE-OPGETROKKEN",
    message: "Ruimte stopt onder het plafond.",
    count: 2,
    room_id: MOCK_ROOM_HOOG,
  }),
  finding({ severity: "warning", code: "X-ANDERS", message: "Onbekende melding." }),
]);
