/**
 * Bepaalt of een module (entitlement) voor de ingelogde gebruiker aan staat.
 * Bron: `entitlements` uit `GET /api/v1/me`. Desktop (Tauri): altijd uit in deel 1.
 * Gedeeld door de constructiemodule en de trapontwerper (O-2026-11-12-01).
 */
import { useEffect, useState } from "react";

import { fetchAuthProfile } from "../lib/auth";
import { isTauri } from "../lib/backend";

export type EntitlementStatus = "laden" | "aan" | "uit";

export function useEntitlement(naam: string): EntitlementStatus {
  const [status, setStatus] = useState<EntitlementStatus>(isTauri() ? "uit" : "laden");

  useEffect(() => {
    if (isTauri()) {
      setStatus("uit");
      return;
    }
    let actief = true;
    fetchAuthProfile().then((profiel) => {
      if (!actief) return;
      setStatus(profiel?.entitlements?.includes(naam) ? "aan" : "uit");
    });
    return () => {
      actief = false;
    };
  }, [naam]);

  return status;
}
