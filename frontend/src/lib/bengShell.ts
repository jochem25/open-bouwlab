/**
 * BENG-schil in de IFC-import-viewer: pure logica (geen three, geen web-ifc).
 *
 * Bron: `<naam>_beng.ifc` van ifc-ruimtebalans (`beng_ifc` in de analyse-
 * respons). Per vlak een `Pset_KBA_BENG` met gevel_id, zone, vlak_type,
 * grenst_aan, orientatie, helling_deg, A_T_m2, constructie_ref en herkomst;
 * ramen met `window_def`; belendingen met `vlak_type` "belending" of zonder Pset.
 * De kleur komt uit het IFC zelf: ifc-ruimtebalans legt de kleuren vast, wij
 * bouwen alleen de legenda uit (begrenzing, orientatie) + die kleur.
 */

export const BENG_PSET = "Pset_KBA_BENG";

/** Vaste velden van een schilvlak (Pset_KBA_BENG), alle optioneel. */
export interface BengVlakInfo {
  gevel_id?: string;
  zone?: string;
  vlak_type?: string;
  grenst_aan?: string;
  orientatie?: string;
  helling_deg?: number;
  A_T_m2?: number;
  constructie_ref?: string;
  herkomst?: string;
  /** Kozijnmerk (alleen bij ramen). */
  window_def?: string;
  /** Overige Pset-velden (bv. omschrijving, bedekking), als tekst. */
  extra: Record<string, string>;
}

const KNOWN_FIELDS = [
  "gevel_id",
  "zone",
  "vlak_type",
  "grenst_aan",
  "orientatie",
  "helling_deg",
  "A_T_m2",
  "constructie_ref",
  "herkomst",
  "window_def",
] as const;
const NUMERIC_FIELDS = new Set<string>(["helling_deg", "A_T_m2"]);

/** Vorm van `IfcAPI.properties.getPropertySets(..., recursive=true)` (deels). */
export interface RawPropertySet {
  Name?: { value?: unknown } | null;
  HasProperties?: Array<{
    Name?: { value?: unknown } | null;
    NominalValue?: { value?: unknown } | null;
  } | null> | null;
}

/** Lees `Pset_KBA_BENG` uit de property sets van een element; null als hij ontbreekt. */
export function vlakInfoFromPsets(psets: RawPropertySet[]): BengVlakInfo | null {
  const pset = psets.find((p) => p?.Name?.value === BENG_PSET);
  if (!pset) return null;
  const info: BengVlakInfo = { extra: {} };
  for (const prop of pset.HasProperties ?? []) {
    const name = prop?.Name?.value;
    const value = prop?.NominalValue?.value;
    if (typeof name !== "string" || value === undefined || value === null) continue;
    if ((KNOWN_FIELDS as readonly string[]).includes(name)) {
      if (NUMERIC_FIELDS.has(name)) {
        const n = typeof value === "number" ? value : Number(value);
        if (Number.isFinite(n)) (info as unknown as Record<string, number>)[name] = n;
      } else {
        (info as unknown as Record<string, string>)[name] = String(value);
      }
    } else {
      info.extra[name] = String(value);
    }
  }
  return info;
}

// ---------------------------------------------------------------------------
// Orientatie en begrenzing: labels
// ---------------------------------------------------------------------------

/** Canonieke korte orientatie (NTA 8800: 8 sectoren + horizontaal). */
export type OrientatieKort = "N" | "NO" | "O" | "ZO" | "Z" | "ZW" | "W" | "NW" | "H";

const ORIENTATIE_ORDER: OrientatieKort[] = ["N", "NO", "O", "ZO", "Z", "ZW", "W", "NW", "H"];

const ORIENTATIE_ALIAS: Record<string, OrientatieKort> = {
  n: "N",
  noord: "N",
  no: "NO",
  noord_oost: "NO",
  noordoost: "NO",
  o: "O",
  oost: "O",
  zo: "ZO",
  zuid_oost: "ZO",
  zuidoost: "ZO",
  z: "Z",
  zuid: "Z",
  zw: "ZW",
  zuid_west: "ZW",
  zuidwest: "ZW",
  w: "W",
  west: "W",
  nw: "NW",
  noord_west: "NW",
  noordwest: "NW",
  h: "H",
  hor: "H",
  horizontaal: "H",
};

/** Normaliseer een orientatie uit de Pset (korte code of snake_case-naam). */
export function normalizeOrientatie(raw: string | undefined): OrientatieKort | null {
  if (!raw) return null;
  return ORIENTATIE_ALIAS[raw.trim().toLowerCase().replace(/[\s-]+/g, "_")] ?? null;
}

const ORIENTATIE_LABEL: Record<OrientatieKort, string> = {
  N: "noord",
  NO: "noordoost",
  O: "oost",
  ZO: "zuidoost",
  Z: "zuid",
  ZW: "zuidwest",
  W: "west",
  NW: "noordwest",
  H: "horizontaal",
};

const GRENST_AAN_LABEL: Record<string, string> = {
  buitenlucht: "Buitenlucht",
  vloer_op_maaiveld_boven_kruipruimte: "Vloer boven kruipruimte",
  vloer_op_maaiveld_boven_grond: "Vloer op grond",
  vloer_op_maaiveld_boven_onverwarmde_kelder: "Vloer boven onverwarmde kelder",
  vloer_onder_maaiveld_boven_kruipruimte: "Vloer onder maaiveld, kruipruimte",
  vloer_onder_maaiveld_boven_grond: "Vloer onder maaiveld, grond",
  vloer_onder_maaiveld_boven_onverwarmde_kelder: "Vloer onder maaiveld, kelder",
  kelderwand_grond: "Kelderwand tegen grond",
  sterk_geventileerd: "Sterk geventileerde ruimte",
  water: "Water",
  aangrenzende_verwarmde_ruimte: "Aangrenzend gebouw (belending)",
  aor_forfaitair: "AOR",
  aos_forfaitair: "AOS / AVR",
};

export function grenstAanLabel(raw: string | undefined): string {
  if (!raw) return "Onbekend";
  return GRENST_AAN_LABEL[raw] ?? raw.replace(/_/g, " ");
}

/** Is dit vlak (deels) bedekt door een belending? */
export function isDeelsBedekt(info: BengVlakInfo): boolean {
  const b = info.extra.bedekking?.toLowerCase();
  if (b === "deels") return true;
  return /BENG-BELENDING-DEELS/i.test(info.herkomst ?? "");
}

/** Soort element in de BENG-schil. */
export type BengElementKind = "vlak" | "raam" | "belending" | "overig";

export function elementKind(info: BengVlakInfo | null): BengElementKind {
  if (!info) return "overig";
  if (info.window_def || info.vlak_type === "raam") return "raam";
  if (info.vlak_type === "belending" && !info.gevel_id) return "belending";
  return info.gevel_id ? "vlak" : "overig";
}

/**
 * Legendasleutel: begrenzing, bij buitenlucht (en AOS) met orientatie, deels
 * bedekte belendingsvlakken apart. Ramen, belendingen en overige elementen
 * hebben een eigen sleutel.
 */
export function legendKey(info: BengVlakInfo | null): string {
  const kind = elementKind(info);
  if (kind !== "vlak" || !info) return kind;
  const ga = info.grenst_aan ?? "onbekend";
  if (ga === "buitenlucht" || ga === "aos_forfaitair") {
    return `${ga}:${normalizeOrientatie(info.orientatie) ?? "?"}`;
  }
  if (ga === "aangrenzende_verwarmde_ruimte" && isDeelsBedekt(info)) return `${ga}:deels`;
  return ga;
}

export function legendLabel(key: string): string {
  if (key === "raam") return "Raam (uitgesneden)";
  if (key === "belending") return "Belending (gebouw ernaast)";
  if (key === "overig") return "Overig element";
  const [ga, sub] = key.split(":");
  if (sub === "deels") return `${grenstAanLabel(ga)}, deels bedekt`;
  if (sub) {
    const o = sub as OrientatieKort;
    return `${grenstAanLabel(ga)} ${ORIENTATIE_LABEL[o] ?? "orientatie onbekend"}`;
  }
  return grenstAanLabel(ga);
}

export interface LegendInput {
  info: BengVlakInfo | null;
  /** Kleur uit het IFC als `#rrggbb`. */
  color: string;
}

export interface LegendEntry {
  key: string;
  label: string;
  color: string;
  count: number;
  /** Som A_T (m2) van de vlakken in deze groep. */
  a_t_m2: number;
}

function legendRank(key: string): number {
  const [ga, sub] = key.split(":");
  const base =
    ga === "buitenlucht" ? 0 : ga === "aos_forfaitair" ? 2 : key === "raam" ? 8 : key === "belending" ? 9 : key === "overig" ? 10 : 1;
  const o = ORIENTATIE_ORDER.indexOf(sub as OrientatieKort);
  return base * 100 + Math.max(o, 0);
}

/**
 * Legenda uit de geladen elementen. Een groep krijgt de kleur van het eerste
 * element; een tweede kleur binnen dezelfde groep telt als afwijking
 * (`mixedColors`), zodat een verschil met de legenda van ifc-ruimtebalans opvalt.
 */
export function buildLegend(items: LegendInput[]): { entries: LegendEntry[]; mixedColors: string[] } {
  const map = new Map<string, LegendEntry>();
  const mixed = new Set<string>();
  for (const it of items) {
    const key = legendKey(it.info);
    const e = map.get(key);
    const a = elementKind(it.info) === "vlak" ? (it.info?.A_T_m2 ?? 0) : 0;
    if (e) {
      e.count += 1;
      e.a_t_m2 += a;
      if (e.color.toLowerCase() !== it.color.toLowerCase()) mixed.add(key);
    } else {
      map.set(key, { key, label: legendLabel(key), color: it.color, count: 1, a_t_m2: a });
    }
  }
  const entries = [...map.values()].sort(
    (x, y) => legendRank(x.key) - legendRank(y.key) || x.key.localeCompare(y.key),
  );
  return { entries, mixedColors: [...mixed].sort() };
}

// ---------------------------------------------------------------------------
// Vlakinfo
// ---------------------------------------------------------------------------

const NUM = new Intl.NumberFormat("nl-NL", { maximumFractionDigits: 2 });

/** Regels voor het infopaneel bij een geklikt vlak (label, waarde). */
export function vlakInfoRows(info: BengVlakInfo | null, fallbackName = ""): Array<[string, string]> {
  if (!info) return [["Element", fallbackName || "zonder Pset_KBA_BENG"]];
  const kind = elementKind(info);
  const rows: Array<[string, string]> = [];
  const push = (label: string, v: string | undefined) => {
    if (v !== undefined && v !== "") rows.push([label, v]);
  };
  if (kind === "raam") {
    push("Kozijnmerk", info.window_def);
    push("In gevel", info.gevel_id);
  } else {
    push("Gevel-id", info.gevel_id);
  }
  push("Zone", info.zone);
  push("Vlaktype", info.vlak_type);
  if (info.grenst_aan) push("Grenst aan", `${grenstAanLabel(info.grenst_aan)} (${info.grenst_aan})`);
  const o = normalizeOrientatie(info.orientatie);
  if (info.orientatie) push("Orientatie", o ? `${o} (${ORIENTATIE_LABEL[o]})` : info.orientatie);
  if (info.helling_deg !== undefined) push("Helling", `${NUM.format(info.helling_deg)} graden`);
  if (info.A_T_m2 !== undefined) push("A_T", `${NUM.format(info.A_T_m2)} m2`);
  push("Constructie", info.constructie_ref);
  if (isDeelsBedekt(info)) push("Belending", "deels bedekt");
  for (const [k, v] of Object.entries(info.extra)) {
    if (k === "bedekking") continue;
    push(k, v);
  }
  push("Herkomst", info.herkomst);
  return rows;
}

// ---------------------------------------------------------------------------
// Weergave-modus
// ---------------------------------------------------------------------------

export type ViewerMode = "warmteverlies" | "beng";

/**
 * Begin-modus: de BENG-schil als die er is en de BENG-import 'te controleren'
 * is (BENG uit IFC, R-3); anders warmteverlies.
 */
export function defaultViewerMode(opts: { hasBengShell: boolean; teControleren: boolean }): ViewerMode {
  return opts.hasBengShell && opts.teControleren ? "beng" : "warmteverlies";
}

// ---------------------------------------------------------------------------
// Controle model tegen beng.json (BA-13 aan onze kant)
// ---------------------------------------------------------------------------

interface BengDocLike {
  beng_geometry?: { zones?: Array<{ gevels?: Array<{ id?: string }> }> };
}

/** gevel_id's die alleen in het model of alleen in beng.json staan. */
export function crossCheckGevelIds(
  infos: Array<BengVlakInfo | null>,
  bengDoc: unknown,
): { alleenModel: string[]; alleenDocument: string[] } {
  const model = new Set(
    infos.filter((i): i is BengVlakInfo => elementKind(i) === "vlak").map((i) => i.gevel_id!),
  );
  const doc = new Set<string>();
  for (const z of (bengDoc as BengDocLike | null)?.beng_geometry?.zones ?? []) {
    for (const g of z.gevels ?? []) if (g.id) doc.add(g.id);
  }
  return {
    alleenModel: [...model].filter((id) => !doc.has(id)).sort(),
    alleenDocument: [...doc].filter((id) => !model.has(id)).sort(),
  };
}
