/**
 * Bepaalt of de trapontwerper voor de ingelogde gebruiker aan staat
 * (entitlement `trap`, zelfde patroon als constructie; besluit Jochem
 * 12-11-2026). Desktop (Tauri): altijd uit in deel 1.
 */
import { useEntitlement, type EntitlementStatus } from "./useEntitlement";

export const TRAP_ENTITLEMENT = "trap";

export function useTrapEntitlement(): EntitlementStatus {
  return useEntitlement(TRAP_ENTITLEMENT);
}
