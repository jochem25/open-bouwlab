/**
 * Organisatie van de ingelogde gebruiker en de gekozen rapporthuisstijl.
 *
 * Bron: `organisatie` / `organisatie_fout` uit `GET /api/v1/me`. De backend
 * leidt de organisatie af uit de Authentik-groep `org-<id>` en controleert
 * de huisstijl bij elk rapport opnieuw; deze store is alleen voor de UI
 * (keuzelijst, organisatienaam in rapporten).
 *
 * Desktop (Tauri) heeft geen login: daar blijft de organisatie leeg.
 */
import { create } from "zustand";

import { fetchAuthProfile, type AuthOrganisatie, type AuthOrganisatieFout } from "../lib/auth";
import { isTauri } from "../lib/backend";

interface OrganisatieState {
  /** `laden` tot `/me` beantwoord is; `klaar` ook als er geen profiel is. */
  status: "idle" | "laden" | "klaar";
  /** Is er een ingelogd profiel (anders: lokale dev of desktop)? */
  ingelogd: boolean;
  organisatie: AuthOrganisatie | null;
  fout: AuthOrganisatieFout | null;
  /** Gekozen huisstijl; `null` = standaard van de organisatie. */
  gekozenHuisstijl: string | null;
  laad: () => Promise<void>;
  kiesHuisstijl: (huisstijl: string | null) => void;
}

export const useOrganisatieStore = create<OrganisatieState>((set, get) => ({
  status: "idle",
  ingelogd: false,
  organisatie: null,
  fout: null,
  gekozenHuisstijl: null,

  laad: async () => {
    if (get().status !== "idle") return;
    if (isTauri()) {
      set({ status: "klaar" });
      return;
    }
    set({ status: "laden" });
    const profiel = await fetchAuthProfile();
    set({
      status: "klaar",
      ingelogd: profiel !== null,
      organisatie: profiel?.organisatie ?? null,
      fout: profiel?.organisatie_fout ?? null,
    });
  },

  kiesHuisstijl: (huisstijl) => {
    const org = get().organisatie;
    const geldig = huisstijl !== null && org?.huisstijlen.includes(huisstijl);
    set({ gekozenHuisstijl: geldig ? huisstijl : null });
  },
}));

/** Organisatienaam voor auteur/adviseur in rapporten; leeg zonder organisatie. */
export function organisatieNaam(): string {
  return useOrganisatieStore.getState().organisatie?.naam ?? "";
}

/**
 * Huisstijl die met een rapport mee moet, of `undefined` voor de standaard
 * van de organisatie (de backend vult die dan zelf in).
 */
export function gekozenHuisstijl(): string | undefined {
  const { organisatie, gekozenHuisstijl: keuze } = useOrganisatieStore.getState();
  if (!organisatie || keuze === null || keuze === organisatie.standaard_huisstijl) {
    return undefined;
  }
  return keuze;
}
