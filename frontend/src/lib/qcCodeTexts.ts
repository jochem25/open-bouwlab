/**
 * Gewone-taalteksten bij QC-codes van de IFC-analyse-engine.
 *
 * De codes zijn stabiel (contract `endpoint-contract.md`); de `message` uit de
 * engine is technisch. Alleen codes met een zeker bekende betekenis staan hier;
 * een onbekende code toont de technische message.
 */
const QC_CODE_TEXTS: Record<string, string> = {
  "L-GEEN-OPBOUW": "Bouwdeel zonder laagopbouw in het model; U-waarde ontbreekt",
  "M-HOOGTE-OPGETROKKEN":
    "Ruimte stopt onder het plafond; hoogte is doorgetrokken tot de constructie",
  "C-GEEN-MAAIVELD": "Maaiveldhoogte niet bepaald; geen vlak is als grond geleverd",
  "B24-GEEN-MAAIVELD": "Maaiveldhoogte niet bepaald; er is niets als grond geleverd",
  "N-TRUENORTH-NUL":
    "Noordrichting staat op 0 graden; controleer of het model op het noorden is georiënteerd",
};

/** Gewone-taaltekst voor een code, of `message` als de code onbekend is. */
export function qcCodeText(code: string, message: string): string {
  return QC_CODE_TEXTS[code] ?? message;
}

export function isKnownQcCode(code: string): boolean {
  return code in QC_CODE_TEXTS;
}
