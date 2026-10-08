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
  code: string;
  message: string;
  room_id?: string;
  construction_id?: string;
}

export interface IfcAnalyseResponse {
  engine: { name: string; version: string };
  source_filename: string;
  thermal: ThermalImportFile;
  qc: { verdict: QcVerdict; findings: QcFinding[] };
  stats?: { duration_s?: number };
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

/** Vertaal een foutresponse (`{error, detail}`) naar een nette NL-melding. */
export function mapIfcAnalyseError(
  status: number,
  body: { error?: string; detail?: string } | null,
): IfcAnalyseError {
  const code = body?.error;
  const detail = body?.detail ? ` (${body.detail})` : "";
  if (status === 503 || code === "analyse_unavailable") {
    return new IfcAnalyseError(`IFC-analyse niet beschikbaar${detail}`, status, code);
  }
  if (status === 413 || code === "file_too_large") {
    return new IfcAnalyseError(
      `Het bestand is te groot (maximaal ${IFC_MAX_MB_LABEL}).`,
      status,
      code,
    );
  }
  if (status === 422 || code === "analyse_failed") {
    return new IfcAnalyseError(`De IFC kon niet worden geanalyseerd${detail}`, status, code);
  }
  if (status === 504 || code === "analyse_timeout") {
    return new IfcAnalyseError(
      "De analyse duurde te lang en is afgebroken. Probeer een kleiner model.",
      status,
      code,
    );
  }
  if (status === 502) {
    return new IfcAnalyseError(
      `De analyse-service gaf een ongeldig antwoord${detail}`,
      status,
      code,
    );
  }
  return new IfcAnalyseError(`IFC-analyse mislukt (HTTP ${status})${detail}`, status, code);
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
