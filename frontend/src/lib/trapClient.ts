/**
 * Client van de trapontwerper (Tauri invoke of web fetch), naar het patroon van
 * bengClient:
 *  - Tauri: `invoke("trap_ontwerp", { invoer })` -> `Result<Uitvoer, String>`;
 *  - web: `POST {API_PREFIX}/trap/ontwerp` met de invoer als body; 422 =
 *    ongeldige invoer of geen oplossing (`detail` is leesbaar).
 */
import { isTauri } from "./backend";
import { API_PREFIX } from "./constants";
import type { TrapInvoer, TrapUitvoer } from "../types/trap";

/** Fout met de leesbare melding van de rekenkern. */
export class TrapFout extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TrapFout";
  }
}

export async function ontwerpTrap(invoer: TrapInvoer, signal?: AbortSignal): Promise<TrapUitvoer> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    try {
      return await invoke<TrapUitvoer>("trap_ontwerp", { invoer });
    } catch (err) {
      throw new TrapFout(err instanceof Error ? err.message : String(err));
    }
  }
  const res = await fetch(`${API_PREFIX}/trap/ontwerp`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify(invoer),
    signal,
  });
  if (!res.ok) {
    const body = (await res.json().catch(() => ({}))) as { detail?: string };
    throw new TrapFout(body.detail ?? `HTTP ${res.status}`);
  }
  return (await res.json()) as TrapUitvoer;
}

/**
 * Tekstbestand bewaren: in Tauri via de opslagdialoog, anders (of als dat
 * faalt) als browserdownload. Zelfde aanpak als exportIfcEnergy.
 */
export async function bewaarTekst(
  bestandsnaam: string,
  inhoud: string,
  filter: { name: string; extensions: string[] },
  mime: string,
): Promise<void> {
  if (isTauri()) {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const { writeTextFile } = await import("@tauri-apps/plugin-fs");
      const pad = await save({ defaultPath: bestandsnaam, filters: [filter] });
      if (!pad) return;
      await writeTextFile(pad, inhoud);
      return;
    } catch (err) {
      console.error("Tauri-opslaan mislukt, terugval op download:", err);
    }
  }
  const url = URL.createObjectURL(new Blob([inhoud], { type: mime }));
  const a = document.createElement("a");
  a.href = url;
  a.download = bestandsnaam;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}
