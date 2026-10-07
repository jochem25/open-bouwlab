/**
 * Typen voor de constructiemodule (voordimensionering staal en beton).
 *
 * Afgeleid van `schemas/constructie/v1/*.schema.json` en de serde-attributen
 * in `crates/constructie-core/src/{model,toets}.rs`.
 */

export type Toepassing = "vloer" | "dak";

export type Gebruiksfunctie =
  | "woon_vloer"
  | "woon_trap"
  | "woon_balkon"
  | "gemeenschappelijk"
  | "kantoor";

export type Gebouwtype =
  | "eengezinswoning1_tot3"
  | "eengezinswoning4_plus"
  | "woongebouw"
  | "kantoorgebouw";

export type Gevolgklasse = "CC1" | "CC2" | "CC3";

export type Scheidingswanden = "geen" | "tot_een" | "tot_twee" | "tot_drie";

export type Reeks = "IPE" | "HEA" | "HEB";
export type Staalsoort = "S235" | "S275" | "S355";
export type Sterkteklasse = "C20/25" | "C25/30" | "C30/37" | "C35/45";
export type Milieuklasse = "XC1" | "XC3";

export interface AlgemeneInvoer {
  toepassing: Toepassing;
  gebruiksfunctie: Gebruiksfunctie;
  gebouwtype: Gebouwtype;
  gevolgklasse?: Gevolgklasse;
  overspanning_m: number;
  permanent_kn_m2: number;
  eigen_gewicht_automatisch: boolean;
  lichte_scheidingswanden: Scheidingswanden;
  scheurgevoelige_scheidingswanden: boolean;
  uiterlijk_van_belang: boolean;
  dak_beloopbaar?: boolean;
  dak_hellingshoek_graden?: number;
  dak_afschot_procent?: number;
}

export interface StaalInvoer {
  element: "ligger";
  algemeen: AlgemeneInvoer;
  belastingbreedte_m: number;
  staalsoort: Staalsoort;
  reeksen: Reeks[];
  bovenflens_gesteund: boolean;
}

export interface BetonInvoer {
  element: "balk";
  algemeen: AlgemeneInvoer;
  belastingbreedte_m: number;
  sterkteklasse: Sterkteklasse;
  balkbreedte_mm: number;
  hoogte_automatisch: boolean;
  hoogte_mm?: number;
  milieuklasse: Milieuklasse;
  phi_hoofd_mm: number;
  phi_beugel_mm: number;
  d_g_mm: number;
}

export type Houtklasse = "C14" | "C16" | "C18" | "C20" | "C22" | "C24" | "C27" | "C30";

/** Serde-tag is `type` (intern getagd), varianten in snake_case. */
export type HoutElement =
  | { type: "balklaag"; hoh_mm: number }
  | { type: "balk"; belastingbreedte_m: number };

export interface Vloerplaat {
  dikte_mm: number;
  e_mean_n_mm2: number;
}

export interface HoutInvoer {
  algemeen: AlgemeneInvoer;
  element: HoutElement;
  sterkteklasse: Houtklasse;
  breedte_mm: number;
  hoogte_automatisch: boolean;
  hoogte_mm?: number;
  klimaatklasse: 1 | 2;
  drukrand_gesteund: boolean;
  /** Alleen bij balklaag. */
  vloerplaat?: Vloerplaat;
  /** Alleen bij balklaag. */
  vloerbreedte_m?: number;
}

export type ConstructieInvoer = StaalInvoer | BetonInvoer | HoutInvoer;

export type Materiaal = "staal" | "beton" | "hout";

export interface Bron {
  norm: string;
  artikel: string;
  nb: boolean;
}

/** Serde-tag is `status` (intern getagd), varianten in snake_case. */
export type ToetsStatus =
  | { status: "voldoet" }
  | { status: "voldoet_niet" }
  | { status: "niet_getoetst"; reden: string; blokkeert_advies: boolean }
  | { status: "aanname"; tekst: string };

export interface Toets {
  id: string;
  naam: string;
  formule: string;
  waarde: number | null;
  grens: number | null;
  eenheid: string;
  uc: number | null;
  bron: Bron;
  status: ToetsStatus;
}

export type MeldingSoort = "waarschuwing" | "signalering" | "info";

export interface Melding {
  soort: MeldingSoort;
  tekst: string;
  bron: Bron | null;
}

export interface BetonKeuze {
  aantal: number;
  diameter_mm: number;
  a_s_prov_mm2: number;
  beugel_diameter_mm: number;
  beugel_s_mm: number;
  cot_theta: number;
}

export interface Kandidaat {
  naam: string;
  hoogte_mm: number;
  eigen_gewicht_kn_m: number;
  gewicht_kg_m: number | null;
  toetsen: Toets[];
  meldingen: Melding[];
  afgewezen: string | null;
  tussenwaarden: Record<string, number>;
  beton: BetonKeuze | null;
}

export interface Maatgevend {
  toets_id: string;
  naam: string;
  uc: number;
  uitleg: string;
}

export interface Kengetallen {
  constructiehoogte_mm: number;
  l_h: number;
  eigen_gewicht_kn_m: number;
  gewicht_kg_m: number | null;
  eigenfrequentie_hz: number | null;
}

export interface Advies {
  naam: string;
  hoogte_mm: number;
  maatgevend: Maatgevend | null;
  kengetallen: Kengetallen;
  toetsen: Toets[];
  beton: BetonKeuze | null;
}

export interface Alternatief {
  naam: string;
  positie: "onder" | "boven";
  voldoet: boolean;
  max_uc: number | null;
  maatgevend: string | null;
  toelichting: string;
}

export interface Resultaat {
  disclaimer: string;
  kern_versie: string;
  normedities: Array<[string, string]>;
  gevolgklasse: Gevolgklasse;
  kandidaten: Kandidaat[];
  advies: Advies | null;
  laagste_bouwhoogte: Advies | null;
  alternatieven: Alternatief[];
  meldingen: Melding[];
}

export interface RapportProject {
  naam?: string;
  opsteller?: string;
}
