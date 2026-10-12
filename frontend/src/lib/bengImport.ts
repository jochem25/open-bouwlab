/**
 * beng-import v1 client: `POST {API_PREFIX}/import/beng` (alleen web).
 *
 * Contract: `docs/beng-import-v1-contract.md`. Het document komt van
 * ifc-ruimtebalans: als los `.beng.json` (knop op de BENG-pagina) of als
 * `beng` in de IFC-analyse-respons (`uitvoer=thermal,beng`).
 *  - 200: `BengImportResult` (`status` compleet of onvolledig);
 *  - 400: geen JSON; 422: contractfout (`detail` met JSON-pad, letterlijk tonen).
 *
 * Er is (nog) geen Tauri-command voor deze import; in de desktop-app geeft de
 * client een duidelijke fout.
 */
import type { BengGeometry } from "../types/bengGeometry";
import { isTauri } from "./backend";
import { API_PREFIX } from "./constants";

/** Spiegelt `BengImportResult` (`openaec_project_shared::beng_import`). */
export interface BengImportResult {
  status: "compleet" | "onvolledig";
  /** Alleen bij compleet; heeft `bron` gezet. */
  beng_geometry?: BengGeometry;
  /** JSON-pointers naar ontbrekende Rc/U- en U_w-waarden (onvolledig). */
  ontbrekend: string[];
  /** Meldingen van bron en import (`code: tekst`); de eerste is "Te controleren: ...". */
  meldingen: string[];
  /** Optionele blokken die v1 niet leest. */
  genegeerd: string[];
  /** IFC-route nog niet geijkt (R-3); nu altijd true. */
  te_controleren: boolean;
  project_name?: string;
  source: string;
}

/** Uitkomst van `/import/beng` op het `beng`-document uit een IFC-analyse. */
export type BengCheck =
  | { phase: "loading" }
  | { phase: "ready"; result: BengImportResult }
  | { phase: "error"; message: string };

export class BengImportError extends Error {
  readonly status: number;
  constructor(message: string, status: number) {
    super(message);
    this.name = "BengImportError";
    this.status = status;
  }
}

/** Importeer een beng-import-document (al geparst object of ruwe JSON-tekst). */
export async function importBengDocument(doc: unknown): Promise<BengImportResult> {
  if (isTauri()) {
    throw new BengImportError(
      "BENG-geometrie importeren kan nu alleen in de webversie.",
      0,
    );
  }
  const body = typeof doc === "string" ? doc : JSON.stringify(doc);
  const res = await fetch(`${API_PREFIX}/import/beng`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body,
  });
  const parsed = (await res.json().catch(() => null)) as
    | (BengImportResult & { error?: string; detail?: string })
    | null;
  if (!res.ok) {
    const detail = parsed?.detail ?? res.statusText ?? "";
    const msg =
      res.status === 400
        ? `Geen geldig JSON-bestand${detail ? ` (${detail})` : ""}`
        : res.status === 422
          ? `Het bestand voldoet niet aan beng-import v1: ${detail}`
          : `BENG-import mislukt (HTTP ${res.status})${detail ? `: ${detail}` : ""}`;
    throw new BengImportError(msg, res.status);
  }
  if (!parsed || (parsed.status !== "compleet" && parsed.status !== "onvolledig")) {
    throw new BengImportError("Onverwacht antwoord van de BENG-import.", 502);
  }
  return parsed;
}

/** Korte samenvatting voor een melding of banner. */
export function bengImportSummary(r: BengImportResult): string {
  if (r.status === "onvolledig") {
    return `Onvolledig: ${r.ontbrekend.length} waarde(n) ontbreken (Rc/U of U_w).`;
  }
  const zones = r.beng_geometry?.zones ?? [];
  const vlakken = zones.reduce((n, z) => n + (z.gevels?.length ?? 0), 0);
  return `${zones.length} rekenzone(s), ${vlakken} vlakken${r.te_controleren ? " - te controleren" : ""}.`;
}
