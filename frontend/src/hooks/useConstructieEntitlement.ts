/**
 * Bepaalt of de constructiemodule voor de ingelogde gebruiker aan staat.
 * Bron: `entitlements` uit `GET /api/v1/me`. Desktop (Tauri): altijd uit in deel 1.
 */
import { useEntitlement, type EntitlementStatus } from "./useEntitlement";

export type { EntitlementStatus } from "./useEntitlement";

export const CONSTRUCTIE_ENTITLEMENT = "constructie";

export function useConstructieEntitlement(): EntitlementStatus {
  return useEntitlement(CONSTRUCTIE_ENTITLEMENT);
}
