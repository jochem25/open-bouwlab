/**
 * Types van de trapontwerper (trap-core, O-2026-10-30-14). Spiegelt de
 * Rust-structs `trap_core::Invoer` (invoer) en `trap_core::Uitvoer` (uitvoer).
 * Maten in mm.
 */

export type P2 = [number, number];
export type Kant = "links" | "rechts";
export type Gebruiksfunctie = "woonfunctie" | "overig";
export type Bouwsoort = "nieuwbouw" | "verbouw";

export interface ArmenPlattegrond {
  soort: "armen";
  start: P2;
  richting: P2;
  binnenzijde: Kant;
  armlengtes: number[];
  breedte: number;
  breedte_is?: "loopbreedte" | "buitenwerks";
  welstuk?: number;
}

/** Vrije contour: randen van de loopbreedte als polylijnen (alleen via 'invoer laden'). */
export interface ContourPlattegrond {
  soort: "contour";
  binnenzijde: P2[];
  buitenzijde: P2[];
  kant_binnen: Kant;
  welstuk?: number;
}

export interface TrappaalInvoer {
  langs_in?: number;
  langs_uit?: number;
  insteek_in?: number;
  insteek_uit?: number;
  boven_boom?: number;
  materiaal?: string;
}

export interface TrapInvoer {
  naam: string;
  project?: { naam?: string; bouwlaag?: string; z_bouwlaag?: number };
  niveaus: { onder: number; boven: number; vloerdikte_boven?: number };
  plattegrond: ArmenPlattegrond | ContourPlattegrond;
  optreden?: number;
  boom?: {
    dikte?: number;
    plankbreedte?: number;
    voorhout?: number;
    achterhout_min?: number;
    nestdiepte?: number;
    materiaal?: string;
  };
  trede?: { dikte?: number; neus?: number };
  stootbord?: { soort?: "dicht" | "open"; dikte?: number };
  trappaal?: TrappaalInvoer;
  verdrijving?: { methode?: "fuik"; vakken?: { van: number; tot: number }[] };
  /** afstand_buitenkant_boom: looplijn vanaf de buitenkant van de buitenboom (besluit 11-11-2026). */
  looplijn?: { afstand_buitenkant_boom?: number; hoek?: "boog" | "scherp"; straal?: number };
  hoekverbinding?: { doorlopend?: "hoger" | "lager" };
  clearance?: { vrije_hoogte?: number };
  gebruiksfunctie?: string;
  bouwsoort?: Bouwsoort;
  trapgat?: P2[];
}

export type Oordeel =
  | "voldoet"
  | "voldoet_niet"
  | "melding"
  | "advies_voldoet"
  | "advies_afwijking"
  | "handmatig"
  | "niet_van_toepassing";

export interface TrapRegel {
  id: string;
  onderwerp: string;
  eis: string;
  grenswaarde: number | null;
  waarde: number | null;
  eenheid: string;
  oordeel: Oordeel;
  toelichting: string;
  bron: string;
  bron_id: string;
  regel: string;
}

export interface Voorkant {
  nummer: number;
  p_binnen: P2;
  p_loop: P2;
  p_buiten: P2;
  verdreven: boolean;
}

export interface TrapRapport {
  trap: string;
  disclaimer: string;
  resultaat: {
    hoogte: number;
    optreden: number;
    aantreden: number;
    optrede: number;
    aantrede_looplijn: number;
    aantreden_binnenzijde: number[];
    aantreden_buitenzijde: number[];
    looplijn_lengte: number;
    vakken: { van: number; tot: number }[];
    voorkanten: Voorkant[];
    loopbreedte: number;
    vrije_hoogte: number;
    treden: number;
    stootborden: number;
  };
  regels: TrapRegel[];
  trapgat_minimaal: P2[];
  waarschuwingen: string[];
}

export interface TrapTekening {
  binnenzijde: P2[];
  buitenzijde: P2[];
  looplijn: P2[];
  klimlijn: P2[];
  klimlijn_afstand: number;
  treden: P2[][];
  bomen: P2[][];
  palen: P2[][];
  trapgat: P2[] | null;
}

export interface TrapUitvoer {
  rapport: TrapRapport;
  tekening: TrapTekening;
  ifc: string;
  ifc_leverancier: string;
  bestandsnaam: string;
}
