/**
 * IFC-analyse — client voor `POST /api/v1/ifc/analyse` (multipart, veld "file").
 *
 * De backend levert een thermal-import v1.2 bestand + een modelcheck (qc).
 * Mock-modus: `VITE_IFC_ANALYSE_MOCK=1` geeft een vaste response uit
 * `__fixtures__/ifcAnalyseMock` (dynamic import, dus niet in de bundle zolang
 * de env-var uit staat en de code-split chunk niet wordt opgehaald).
 */
import { API_PREFIX } from "./constants";
import { SessionExpiredError } from "./backend";
import type { ThermalImportFile } from "./thermalImport";

/** Maximale uploadgrootte (bytes) — gelijk aan de backend-limiet. */
export const IFC_MAX_BYTES = 100 * 1024 * 1024;
export const IFC_MAX_MB_LABEL = "100 MB";

export type QcSeverity = "blocking" | "warning" | "info";
export type QcVerdict = "ok" | "waarschuwing" | "blokkerend";

export interface QcFinding {
  severity: QcSeverity;
  /** Stabiele code (bv. `L-GEEN-OPBOUW`); zie `lib/qcCodeTexts.ts` voor gewone taal. */
  code: string;
  /** Rapporttekst (NL, technisch). */
  message: string;
  /** Aantal gevallen; `null` als niet van toepassing. */
  count: number | null;
  /** De modelleur kan dit in het IFC oplossen. */
  modeller_action: boolean;
  /** thermal `rooms[].id`. */
  room_id?: string;
  construction_id?: string;
  subject?: string;
}

export interface QcCounts {
  blocking: number;
  warning: number;
  info: number;
}

export interface QcResult {
  verdict: QcVerdict;
  counts: QcCounts;
  modelcheck: { verdict: string; failed_rules: string[] };
  findings: QcFinding[];
}

export interface RoomExtraOpening {
  id: string;
  type: string;
  area_m2: number;
  width_mm: number;
  height_mm: number;
  sill_height_mm: number | null;
  construction_id: string;
  compass?: string | null;
  /** 0 = plat, 90 = verticaal. */
  tilt_deg: number | null;
  /** room_b. */
  adjacent: string;
}

/** Per ruimte extra gegevens voor ventilatie/BENG (afgeleid uit thermal). */
export interface RoomExtra {
  room_id: string;
  /** IfcSpace.Name bij een afwijkende LongName (bij Revit het ruimtenummer). */
  number: string | null;
  name: string;
  ifc_name: string | null;
  ifc_long_name: string | null;
  function: string | null;
  function_source: string | null;
  level: string | null;
  heated: boolean;
  floor_area_m2: number | null;
  height_m: number | null;
  volume_m3: number | null;
  openings: RoomExtraOpening[];
}

export interface IfcAnalyseStats {
  duration_s: number;
  exit_code: number;
  filename: string;
  bytes: number;
  sha256: string;
  profile: string;
}

/** Contract: `%KBA_SHARED%/uitvoer/ifc-ruimtebalans/2026-10-08-service-mock/endpoint-contract.md`. */
export interface IfcAnalyseResponse {
  engine: { name: string; version: string; commit: string; contract: string };
  /** Gezet door onze API. */
  source_filename: string;
  thermal: ThermalImportFile;
  thermal_sha256: string;
  qc: QcResult;
  rooms_extra: RoomExtra[];
  stats: IfcAnalyseStats;
  /** Volledig QC-rapport (~0,9 MB bij een groot model). */
  report?: unknown;
  /** Losse vlakken-IFC voor de viewer (tot ~3 MB), of null. */
  surfaces_ifc?: { filename: string; bytes: number; base64: string } | null;
}

/**
 * De analyse zoals die in het project wordt bewaard (envelope + sessie):
 * ZONDER `report` en `surfaces_ifc`. Die zijn groot (report ~0,9 MB, vlakken tot
 * ~3 MB bij een groot model), staan niet in de ventilatie/BENG-bron en worden
 * bij elke opslag naar server en bestand meegeschreven. `thermal`, `qc`,
 * `rooms_extra`, engine, `thermal_sha256` en stats blijven.
 */
export type IfcAnalyseStored = Omit<IfcAnalyseResponse, "report" | "surfaces_ifc">;

export function stripAnalyse(response: IfcAnalyseResponse): IfcAnalyseStored {
  const { report: _report, surfaces_ifc: _surfaces, ...stored } = response;
  return stored;
}

export type AnalyseProgress =
  | { phase: "upload"; fraction: number }
  | { phase: "analyse" };

export class IfcAnalyseError extends Error {
  readonly status: number;
  readonly code?: string;
  constructor(message: string, status: number, code?: string) {
    super(message);
    this.name = "IfcAnalyseError";
    this.status = status;
    this.code = code;
  }
}

/**
 * Vertaal een foutresponse (`{error, detail}`) naar een nette NL-melding.
 * Herkent zowel de codes van de engine-service (`BEZET`, `TE_GROOT`, ...) als
 * die van onze API (`analyse_unavailable`, `file_too_large`, ...).
 */
export function mapIfcAnalyseError(
  status: number,
  body: { error?: string; detail?: string } | null,
): IfcAnalyseError {
  const code = body?.error;
  const key = code?.toLowerCase();
  const detail = body?.detail ? ` (${body.detail})` : "";
  const make = (msg: string) => new IfcAnalyseError(msg, status, code);
  if (key === "bezet" || key === "analyse_busy") {
    return make("Er loopt al een IFC-analyse. Probeer het over een halve minuut opnieuw.");
  }
  if (status === 503 || key === "analyse_unavailable") {
    return make(`IFC-analyse niet beschikbaar${detail}`);
  }
  if (status === 413 || key === "file_too_large" || key === "te_groot") {
    return make(`Het bestand is te groot (maximaal ${IFC_MAX_MB_LABEL})${detail}.`);
  }
  if (status === 422 || key === "analyse_failed" || key === "ifc_onleesbaar") {
    return make(`Dit IFC kan niet worden gelezen of geanalyseerd${detail}`);
  }
  if (status === 504 || key === "analyse_timeout" || key === "time_out") {
    return make("De analyse duurde te lang en is afgebroken. Probeer een kleiner model.");
  }
  if (key === "engine_fout") {
    return make(`De analyse-engine is onverwacht gestopt${detail}`);
  }
  if (status === 400) {
    return make(`Het bestand is niet geaccepteerd${detail}`);
  }
  if (status === 502) {
    return make(`De analyse-service gaf een ongeldig antwoord${detail}`);
  }
  return make(`IFC-analyse mislukt (HTTP ${status})${detail}`);
}

const MOCK_DELAY_MS = 300;

function isMockMode(): boolean {
  return import.meta.env.VITE_IFC_ANALYSE_MOCK === "1";
}

async function mockAnalyse(
  onProgress?: (p: AnalyseProgress) => void,
): Promise<IfcAnalyseResponse> {
  onProgress?.({ phase: "upload", fraction: 1 });
  onProgress?.({ phase: "analyse" });
  await new Promise((r) => setTimeout(r, MOCK_DELAY_MS));
  const { ifcAnalyseMock } = await import("./__fixtures__/ifcAnalyseMock");
  return structuredClone(ifcAnalyseMock);
}

/** Upload een IFC en wacht op de analyse. */
export function analyseIfc(
  file: File,
  onProgress?: (p: AnalyseProgress) => void,
): Promise<IfcAnalyseResponse> {
  if (isMockMode()) return mockAnalyse(onProgress);

  return new Promise<IfcAnalyseResponse>((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", `${API_PREFIX}/ifc/analyse`);
    xhr.withCredentials = true;
    xhr.responseType = "text";

    xhr.upload.onprogress = (e: ProgressEvent) => {
      if (e.lengthComputable && e.total > 0) {
        onProgress?.({ phase: "upload", fraction: e.loaded / e.total });
      }
    };
    xhr.upload.onload = () => onProgress?.({ phase: "analyse" });

    xhr.onerror = () =>
      reject(new IfcAnalyseError("Geen verbinding met de server.", 0, "network"));
    xhr.ontimeout = () =>
      reject(new IfcAnalyseError("De server reageerde niet op tijd.", 0, "timeout"));

    xhr.onload = () => {
      const text = typeof xhr.responseText === "string" ? xhr.responseText : "";
      const contentType = xhr.getResponseHeader("content-type") ?? "";
      if (xhr.status === 401 || xhr.status === 403 || contentType.includes("html")) {
        reject(new SessionExpiredError("Je sessie is verlopen — log opnieuw in."));
        return;
      }
      let body: unknown = null;
      try {
        body = JSON.parse(text);
      } catch {
        body = null;
      }
      if (xhr.status >= 200 && xhr.status < 300) {
        if (body && typeof body === "object" && "thermal" in body && "qc" in body) {
          resolve(body as IfcAnalyseResponse);
        } else {
          reject(new IfcAnalyseError("Onverwacht antwoord van de analyse-service.", 502));
        }
        return;
      }
      reject(mapIfcAnalyseError(xhr.status, body as { error?: string; detail?: string } | null));
    };

    const form = new FormData();
    form.append("file", file);
    xhr.send(form);
  });
}
