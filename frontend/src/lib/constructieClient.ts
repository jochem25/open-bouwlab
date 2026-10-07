/**
 * API-client voor de constructiemodule (voordimensionering staal en beton).
 * Rekenkern draait server-side; fetch met credentials (Authentik-cookie).
 */
import { API_PREFIX } from "./constants";
import type {
  BetonInvoer,
  Materiaal,
  RapportProject,
  Resultaat,
  StaalInvoer,
} from "../types/constructie";

const BASE = `${API_PREFIX}/constructie`;

/** Fout met HTTP-status, zodat de UI 403 (geen entitlement) kan onderscheiden. */
export class ConstructieFout extends Error {
  readonly status: number;
  constructor(message: string, status: number) {
    super(message);
    this.name = "ConstructieFout";
    this.status = status;
  }
}

async function leesFout(res: Response): Promise<ConstructieFout> {
  let bericht = res.statusText || `HTTP ${res.status}`;
  try {
    const json = (await res.json()) as { error?: string };
    if (json.error) bericht = json.error;
  } catch {
    // geen JSON-body
  }
  return new ConstructieFout(bericht, res.status);
}

function post(pad: string, body: unknown, signal?: AbortSignal): Promise<Response> {
  return fetch(`${BASE}/${pad}`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify(body),
    signal,
  });
}

export async function berekenConstructie(
  materiaal: Materiaal,
  invoer: StaalInvoer | BetonInvoer,
  signal?: AbortSignal,
): Promise<Resultaat> {
  const res = await post(materiaal, invoer, signal);
  if (!res.ok) throw await leesFout(res);
  return (await res.json()) as Resultaat;
}

export async function genereerConstructieRapport(
  materiaal: Materiaal,
  invoer: StaalInvoer | BetonInvoer,
  project?: RapportProject,
): Promise<Blob> {
  const res = await post("rapport", { materiaal, invoer, project });
  if (!res.ok) throw await leesFout(res);
  return res.blob();
}

/** Veilige bestandsnaam: constructie-<materiaal>-<advies>.pdf */
export function rapportBestandsnaam(materiaal: Materiaal, adviesNaam: string | null): string {
  const slug = (adviesNaam ?? "geen-advies")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `constructie-${materiaal}-${slug || "geen-advies"}.pdf`;
}
