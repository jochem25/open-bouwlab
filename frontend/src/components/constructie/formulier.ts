/**
 * Formulierstate van de constructiemodule en de omzetting naar API-invoer.
 * Velden zijn `number | null` zodat een leeg veld als "ongeldig" herkend wordt.
 */
import type {
  AlgemeneInvoer,
  BetonInvoer,
  Gebouwtype,
  Gebruiksfunctie,
  Gevolgklasse,
  Materiaal,
  Milieuklasse,
  Reeks,
  Scheidingswanden,
  StaalInvoer,
  Staalsoort,
  Sterkteklasse,
  Toepassing,
} from "../../types/constructie";

export interface Formulier {
  toepassing: Toepassing;
  gebruiksfunctie: Gebruiksfunctie;
  gebouwtype: Gebouwtype;
  /** "auto" volgt het gebouwtype. */
  gevolgklasse: "auto" | Gevolgklasse;
  overspanning_m: number | null;
  permanent_kn_m2: number | null;
  eigen_gewicht_automatisch: boolean;
  lichte_scheidingswanden: Scheidingswanden;
  scheurgevoelige_scheidingswanden: boolean;
  uiterlijk_van_belang: boolean;
  dak_beloopbaar: boolean;
  dak_hellingshoek_graden: number | null;
  dak_afschot_procent: number | null;
  belastingbreedte_m: number | null;
  // staal
  staalsoort: Staalsoort;
  reeksen: Reeks[];
  bovenflens_gesteund: boolean;
  // beton
  sterkteklasse: Sterkteklasse;
  balkbreedte_mm: number | null;
  hoogte_automatisch: boolean;
  hoogte_mm: number | null;
  milieuklasse: Milieuklasse;
  phi_hoofd_mm: number | null;
  phi_beugel_mm: number | null;
  d_g_mm: number | null;
}

export const STANDAARD_FORMULIER: Formulier = {
  toepassing: "vloer",
  gebruiksfunctie: "woon_vloer",
  gebouwtype: "eengezinswoning1_tot3",
  gevolgklasse: "auto",
  overspanning_m: 5,
  permanent_kn_m2: 1,
  eigen_gewicht_automatisch: true,
  lichte_scheidingswanden: "geen",
  scheurgevoelige_scheidingswanden: false,
  uiterlijk_van_belang: true,
  dak_beloopbaar: false,
  dak_hellingshoek_graden: 0,
  dak_afschot_procent: 1.6,
  belastingbreedte_m: 3,
  staalsoort: "S235",
  reeksen: ["IPE", "HEA", "HEB"],
  bovenflens_gesteund: true,
  sterkteklasse: "C30/37",
  balkbreedte_mm: 300,
  hoogte_automatisch: true,
  hoogte_mm: 450,
  milieuklasse: "XC1",
  phi_hoofd_mm: 20,
  phi_beugel_mm: 8,
  d_g_mm: 16,
};

/** Minimale gevolgklasse per gebouwtype (spiegelt de rekenkern; server blijft leidend). */
export const MINIMALE_GEVOLGKLASSE: Record<Gebouwtype, Gevolgklasse> = {
  eengezinswoning1_tot3: "CC1",
  eengezinswoning4_plus: "CC2",
  woongebouw: "CC2",
  kantoorgebouw: "CC2",
};

const KLASSE_VOLGORDE: Gevolgklasse[] = ["CC1", "CC2", "CC3"];

/** Gevolgklassen die voor dit gebouwtype toegestaan zijn (>= minimum). */
export function toegestaneGevolgklassen(gebouwtype: Gebouwtype): Gevolgklasse[] {
  const min = KLASSE_VOLGORDE.indexOf(MINIMALE_GEVOLGKLASSE[gebouwtype]);
  return KLASSE_VOLGORDE.slice(min);
}

function bouwAlgemeen(f: Formulier): AlgemeneInvoer | null {
  if (f.overspanning_m === null || f.permanent_kn_m2 === null) return null;
  const a: AlgemeneInvoer = {
    toepassing: f.toepassing,
    gebruiksfunctie: f.gebruiksfunctie,
    gebouwtype: f.gebouwtype,
    overspanning_m: f.overspanning_m,
    permanent_kn_m2: f.permanent_kn_m2,
    eigen_gewicht_automatisch: f.eigen_gewicht_automatisch,
    lichte_scheidingswanden: f.lichte_scheidingswanden,
    scheurgevoelige_scheidingswanden: f.scheurgevoelige_scheidingswanden,
    uiterlijk_van_belang: f.uiterlijk_van_belang,
  };
  if (f.gevolgklasse !== "auto" && toegestaneGevolgklassen(f.gebouwtype).includes(f.gevolgklasse)) {
    a.gevolgklasse = f.gevolgklasse;
  }
  if (f.toepassing === "dak") {
    if (f.dak_hellingshoek_graden === null || f.dak_afschot_procent === null) return null;
    a.dak_beloopbaar = f.dak_beloopbaar;
    a.dak_hellingshoek_graden = f.dak_hellingshoek_graden;
    a.dak_afschot_procent = f.dak_afschot_procent;
  }
  return a;
}

/** Zet het formulier om naar API-invoer; `null` zolang het formulier onvolledig is. */
export function bouwInvoer(f: Formulier, materiaal: "staal"): StaalInvoer | null;
export function bouwInvoer(f: Formulier, materiaal: "beton"): BetonInvoer | null;
export function bouwInvoer(f: Formulier, materiaal: Materiaal): StaalInvoer | BetonInvoer | null;
export function bouwInvoer(f: Formulier, materiaal: Materiaal): StaalInvoer | BetonInvoer | null {
  const algemeen = bouwAlgemeen(f);
  if (!algemeen || f.belastingbreedte_m === null) return null;
  if (materiaal === "staal") {
    if (f.reeksen.length === 0) return null;
    return {
      element: "ligger",
      algemeen,
      belastingbreedte_m: f.belastingbreedte_m,
      staalsoort: f.staalsoort,
      reeksen: f.reeksen,
      bovenflens_gesteund: f.bovenflens_gesteund,
    };
  }
  if (
    f.balkbreedte_mm === null ||
    f.phi_hoofd_mm === null ||
    f.phi_beugel_mm === null ||
    f.d_g_mm === null ||
    (!f.hoogte_automatisch && f.hoogte_mm === null)
  ) {
    return null;
  }
  const invoer: BetonInvoer = {
    element: "balk",
    algemeen,
    belastingbreedte_m: f.belastingbreedte_m,
    sterkteklasse: f.sterkteklasse,
    balkbreedte_mm: f.balkbreedte_mm,
    hoogte_automatisch: f.hoogte_automatisch,
    milieuklasse: f.milieuklasse,
    phi_hoofd_mm: f.phi_hoofd_mm,
    phi_beugel_mm: f.phi_beugel_mm,
    d_g_mm: f.d_g_mm,
  };
  if (!f.hoogte_automatisch && f.hoogte_mm !== null) invoer.hoogte_mm = f.hoogte_mm;
  return invoer;
}

/** Parse een Input-waarde (string met punt) naar getal of `null` bij leeg/ongeldig. */
export function parseGetal(raw: string): number | null {
  const s = raw.trim().replace(",", ".");
  if (s === "") return null;
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
}
