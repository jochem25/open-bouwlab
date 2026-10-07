/**
 * Bepaalt of de constructiemodule voor de ingelogde gebruiker aan staat.
 * Bron: `entitlements` uit `GET /api/v1/me`. Desktop (Tauri): altijd uit in deel 1.
 */
import { useEffect, useState } from "react";

import { fetchAuthProfile } from "../lib/auth";
import { isTauri } from "../lib/backend";

export const CONSTRUCTIE_ENTITLEMENT = "constructie";

export type EntitlementStatus = "laden" | "aan" | "uit";

export function useConstructieEntitlement(): EntitlementStatus {
  const [status, setStatus] = useState<EntitlementStatus>(isTauri() ? "uit" : "laden");

  useEffect(() => {
    if (isTauri()) {
      setStatus("uit");
      return;
    }
    let actief = true;
    fetchAuthProfile().then((profiel) => {
      if (!actief) return;
      setStatus(profiel?.entitlements?.includes(CONSTRUCTIE_ENTITLEMENT) ? "aan" : "uit");
    });
    return () => {
      actief = false;
    };
  }, []);

  return status;
}
