/**
 * Formulier van de trapontwerper en de vertaling naar de invoer van trap-core
 * (en terug, voor 'invoer laden'). Lege of ongeldige getallen zijn `null`;
 * `bouwInvoer` geeft dan een melding in plaats van een invoer.
 */
import type { Bouwsoort, Gebruiksfunctie, Kant, P2, TrapInvoer } from "../../types/trap";

export type Vorm = "recht" | "kwart_links" | "kwart_rechts" | "half_links" | "half_rechts";

export const VORMEN: Vorm[] = ["recht", "kwart_links", "kwart_rechts", "half_links", "half_rechts"];

export interface Formulier {
  naam: string;
  projectnaam: string;
  bouwlaag: string;
  vorm: Vorm;
  /** Niveau onder (afgewerkte vloer), mm. */
  onder: number | null;
  verdiepingshoogte: number | null;
  vloerdikte_boven: number | null;
  /** Armlengtes langs de buitenzijde (beschikbare lengte per arm). */
  armen: [number | null, number | null, number | null];
  breedte: number | null;
  breedte_is: "loopbreedte" | "buitenwerks";
  optreden_auto: boolean;
  optreden: number | null;
  gebruiksfunctie: Gebruiksfunctie;
  bouwsoort: Bouwsoort;
  looplijn_afstand: number | null;
  looplijn_hoek: "boog" | "scherp";
  /** Afronding van de looplijn in de hoek; leeg = boog uit de binnenhoek. */
  looplijn_straal: number | null;
  vakken_auto: boolean;
  /** Verdrijfvakken als "0-5, 5-15" (voorkantnummers). */
  vakken: string;
  boom_dikte: number | null;
  plankbreedte: number | null;
  voorhout: number | null;
  achterhout_min: number | null;
  nestdiepte: number | null;
  trede_dikte: number | null;
  neus: number | null;
  stootbord: "dicht" | "open";
  stootbord_dikte: number | null;
  welstuk: number | null;
  paal: number | null;
  doorlopend: "hoger" | "lager";
  /** Vrije hoogte boven de trap; leeg = Bbl-waarde bij de gebruiksfunctie. */
  vrije_hoogte: number | null;
  /** Optioneel trapgat als "x,y; x,y; ..." (mm, zelfde stelsel als de trap). */
  trapgat: string;
  /** Ligging van de trap (niet in het scherm, wel bewaard bij laden). */
  start: P2;
  richting: P2;
}

export const STANDAARD_FORMULIER: Formulier = {
  naam: "trap",
  projectnaam: "project",
  bouwlaag: "begane grond",
  vorm: "kwart_links",
  onder: 0,
  verdiepingshoogte: 2800,
  vloerdikte_boven: 300,
  armen: [2400, 1600, 1500],
  breedte: 850,
  breedte_is: "loopbreedte",
  optreden_auto: true,
  optreden: 15,
  gebruiksfunctie: "woonfunctie",
  bouwsoort: "nieuwbouw",
  looplijn_afstand: 300,
  looplijn_hoek: "boog",
  looplijn_straal: null,
  vakken_auto: true,
  vakken: "",
  boom_dikte: 40,
  plankbreedte: 320,
  voorhout: 50,
  achterhout_min: 20,
  nestdiepte: 10,
  trede_dikte: 40,
  neus: 42,
  stootbord: "dicht",
  stootbord_dikte: 18,
  welstuk: 80,
  paal: 90,
  doorlopend: "hoger",
  vrije_hoogte: null,
  trapgat: "",
  start: [0, 0],
  richting: [0, 1],
};

/** Getal uit een invoerveld (komma of punt); leeg of ongeldig = null. */
export function parseGetal(s: string): number | null {
  const t = s.trim().replace(",", ".");
  if (t === "") return null;
  const x = Number(t);
  return Number.isFinite(x) ? x : null;
}

export function aantalArmen(vorm: Vorm): 1 | 2 | 3 {
  if (vorm === "recht") return 1;
  return vorm.startsWith("kwart") ? 2 : 3;
}

function binnenzijde(vorm: Vorm): Kant {
  return vorm.endsWith("rechts") ? "rechts" : "links";
}

/** "0-5, 5-15" naar vakken; null bij een onleesbaar vak. */
export function parseVakken(s: string): { van: number; tot: number }[] | null {
  const delen = s
    .split(/[;,]/)
    .map((d) => d.trim())
    .filter((d) => d !== "");
  const uit: { van: number; tot: number }[] = [];
  for (const d of delen) {
    const m = /^(\d+)\s*-\s*(\d+)$/.exec(d);
    if (!m) return null;
    const van = Number(m[1]);
    const tot = Number(m[2]);
    if (tot <= van) return null;
    uit.push({ van, tot });
  }
  return uit;
}

/** "x,y; x,y; ..." naar een veelhoek (minstens 3 punten); null als onleesbaar. */
export function parseVeelhoek(s: string): P2[] | null {
  const punten = s
    .split(/[;\n]/)
    .map((d) => d.trim())
    .filter((d) => d !== "");
  const uit: P2[] = [];
  for (const p of punten) {
    const xy = p.split(/[\s,]+/).map(Number);
    if (xy.length !== 2 || xy.some((v) => !Number.isFinite(v))) return null;
    uit.push([xy[0]!, xy[1]!]);
  }
  return uit.length >= 3 ? uit : null;
}

export type Bouwresultaat = { invoer: TrapInvoer; fout?: undefined } | { invoer?: undefined; fout: string };

/** Formulier naar trap-core-invoer, of een melding welk veld ontbreekt. */
export function bouwInvoer(f: Formulier): Bouwresultaat {
  const n = aantalArmen(f.vorm);
  const verplicht: [string, number | null][] = [
    ["niveau onder", f.onder],
    ["verdiepingshoogte", f.verdiepingshoogte],
    ["vloerdikte boven", f.vloerdikte_boven],
    ["trapbreedte", f.breedte],
    ["looplijn uit buitenzijde", f.looplijn_afstand],
    ["boomdikte", f.boom_dikte],
    ["plankbreedte", f.plankbreedte],
    ["voorhout", f.voorhout],
    ["achterhout", f.achterhout_min],
    ["nestdiepte", f.nestdiepte],
    ["tredendikte", f.trede_dikte],
    ["neus", f.neus],
    ["stootborddikte", f.stootbord_dikte],
    ["welstuk", f.welstuk],
  ];
  for (let i = 0; i < n; i++) verplicht.push([`arm ${i + 1}`, f.armen[i] ?? null]);
  if (n > 1) verplicht.push(["trappaal", f.paal]);
  if (!f.optreden_auto) verplicht.push(["aantal optreden", f.optreden]);
  for (const [naam, w] of verplicht) {
    if (w === null) return { fout: `vul '${naam}' in` };
  }
  let vakken: { van: number; tot: number }[] | undefined;
  if (!f.vakken_auto && n > 1) {
    const v = parseVakken(f.vakken);
    if (!v || v.length === 0) return { fout: "verdrijfvakken onleesbaar (bv. 0-5, 5-15)" };
    vakken = v;
  }
  let trapgat: P2[] | undefined;
  if (f.trapgat.trim() !== "") {
    const g = parseVeelhoek(f.trapgat);
    if (!g) return { fout: "trapgat onleesbaar (x,y; x,y; x,y ...)" };
    trapgat = g;
  }
  const onder = f.onder!;
  const invoer: TrapInvoer = {
    naam: f.naam.trim() || "trap",
    project: { naam: f.projectnaam.trim() || "project", bouwlaag: f.bouwlaag.trim() || "bouwlaag" },
    niveaus: { onder, boven: onder + f.verdiepingshoogte!, vloerdikte_boven: f.vloerdikte_boven! },
    plattegrond: {
      soort: "armen",
      start: f.start,
      richting: f.richting,
      binnenzijde: binnenzijde(f.vorm),
      armlengtes: f.armen.slice(0, n).map((a) => a!),
      breedte: f.breedte!,
      breedte_is: f.breedte_is,
      welstuk: f.welstuk!,
    },
    boom: {
      dikte: f.boom_dikte!,
      plankbreedte: f.plankbreedte!,
      voorhout: f.voorhout!,
      achterhout_min: f.achterhout_min!,
      nestdiepte: f.nestdiepte!,
    },
    trede: { dikte: f.trede_dikte!, neus: f.neus! },
    stootbord: { soort: f.stootbord, dikte: f.stootbord_dikte! },
    looplijn: {
      afstand_buitenzijde: f.looplijn_afstand!,
      hoek: f.looplijn_hoek,
      ...(f.looplijn_straal !== null ? { straal: f.looplijn_straal } : {}),
    },
    hoekverbinding: { doorlopend: f.doorlopend },
    clearance: f.vrije_hoogte !== null ? { vrije_hoogte: f.vrije_hoogte } : {},
    gebruiksfunctie: f.gebruiksfunctie,
    bouwsoort: f.bouwsoort,
  };
  if (!f.optreden_auto) invoer.optreden = f.optreden!;
  if (n > 1) invoer.trappaal = { langs_in: f.paal!, langs_uit: f.paal! };
  if (vakken) invoer.verdrijving = { methode: "fuik", vakken };
  if (trapgat) invoer.trapgat = trapgat;
  return { invoer };
}

function vormUit(armen: number, kant: Kant): Vorm {
  if (armen === 1) return "recht";
  return `${armen === 2 ? "kwart" : "half"}_${kant}` as Vorm;
}

/**
 * Opgeslagen invoer (JSON) terug naar het formulier. Alleen plattegrond
 * 'armen' met 1-3 armen; anders een melding.
 */
export function invoerNaarFormulier(
  raw: unknown,
): { formulier: Formulier; fout?: undefined } | { formulier?: undefined; fout: string } {
  const inv = raw as Partial<TrapInvoer> | null;
  if (!inv || typeof inv !== "object" || !inv.niveaus || !inv.plattegrond) {
    return { fout: "geen trapinvoer (niveaus en plattegrond ontbreken)" };
  }
  const p = inv.plattegrond;
  if (p.soort !== "armen" || !Array.isArray(p.armlengtes) || p.armlengtes.length < 1 || p.armlengtes.length > 3) {
    return { fout: "alleen een plattegrond met 1 tot 3 armen kan in het scherm worden geladen" };
  }
  const s = STANDAARD_FORMULIER;
  const a = p.armlengtes;
  const formulier: Formulier = {
    ...s,
    naam: inv.naam ?? s.naam,
    projectnaam: inv.project?.naam ?? s.projectnaam,
    bouwlaag: inv.project?.bouwlaag ?? s.bouwlaag,
    vorm: vormUit(a.length, p.binnenzijde),
    onder: inv.niveaus.onder,
    verdiepingshoogte: inv.niveaus.boven - inv.niveaus.onder,
    vloerdikte_boven: inv.niveaus.vloerdikte_boven ?? s.vloerdikte_boven,
    armen: [a[0] ?? null, a[1] ?? s.armen[1], a[2] ?? s.armen[2]],
    breedte: p.breedte,
    breedte_is: p.breedte_is ?? "loopbreedte",
    optreden_auto: inv.optreden === undefined || inv.optreden === null,
    optreden: inv.optreden ?? s.optreden,
    gebruiksfunctie:
      inv.gebruiksfunctie === undefined || inv.gebruiksfunctie === "woonfunctie" ? "woonfunctie" : "overig",
    bouwsoort: inv.bouwsoort ?? s.bouwsoort,
    looplijn_afstand: inv.looplijn?.afstand_buitenzijde ?? s.looplijn_afstand,
    looplijn_hoek: inv.looplijn?.hoek ?? s.looplijn_hoek,
    looplijn_straal: inv.looplijn?.straal ?? null,
    vakken_auto: !inv.verdrijving?.vakken,
    vakken: (inv.verdrijving?.vakken ?? []).map((v) => `${v.van}-${v.tot}`).join(", "),
    boom_dikte: inv.boom?.dikte ?? s.boom_dikte,
    plankbreedte: inv.boom?.plankbreedte ?? s.plankbreedte,
    voorhout: inv.boom?.voorhout ?? s.voorhout,
    achterhout_min: inv.boom?.achterhout_min ?? s.achterhout_min,
    nestdiepte: inv.boom?.nestdiepte ?? s.nestdiepte,
    trede_dikte: inv.trede?.dikte ?? s.trede_dikte,
    neus: inv.trede?.neus ?? s.neus,
    stootbord: inv.stootbord?.soort ?? s.stootbord,
    stootbord_dikte: inv.stootbord?.dikte ?? s.stootbord_dikte,
    welstuk: p.welstuk ?? s.welstuk,
    paal: inv.trappaal?.langs_in ?? s.paal,
    doorlopend: inv.hoekverbinding?.doorlopend ?? s.doorlopend,
    vrije_hoogte: inv.clearance?.vrije_hoogte ?? null,
    trapgat: (inv.trapgat ?? []).map(([x, y]) => `${x},${y}`).join("; "),
    start: p.start,
    richting: p.richting,
  };
  return { formulier };
}
