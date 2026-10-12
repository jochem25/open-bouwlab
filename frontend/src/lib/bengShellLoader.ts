/**
 * BENG-schilmodel laden met web-ifc (zonder Fragments): per element de
 * driehoeken in wereldcoordinaten, de kleur uit het IFC en `Pset_KBA_BENG`.
 *
 * web-ifc zet de geometrie al om naar meters en Y-up: wereld = (x, z, -y).
 * Het schilmodel staat in het stelsel van het bron-IFC (afspraak met
 * ifc-ruimtebalans, 12-10-2026), vaak RD (>100 km). Daarom standaard
 * COORDINATE_TO_ORIGIN: precisie in float32; de BENG-weergave staat los van de
 * warmteverliesvlakken (schakelaar), een overlay is niet nodig.
 *
 * Kleur en doorzicht gelden per geplaatste geometrie, niet per element: een
 * gevel_id kan stukken met verschillende kleur hebben (het belendingsdeel van
 * een deels bedekte wand is magenta, kleurklasse belending_deels).
 */
import { IfcAPI } from "web-ifc";

import { vlakInfoFromPsets, type BengVlakInfo, type RawPropertySet } from "./bengShell";

/** Stukken van een element met dezelfde kleur en hetzelfde doorzicht. */
export interface BengShellPart {
  /** xyz-tripels in wereldcoordinaten (m, Y-up). */
  positions: Float32Array;
  indices: Uint32Array;
  /** Kleur uit het IFC, `#rrggbb`. */
  color: string;
  /** 1 = dekkend. */
  opacity: number;
}

export interface BengShellElement {
  expressId: number;
  /** IFC-klasse, bv. IfcBuildingElementProxy. */
  ifcType: string;
  name: string;
  /** xyz-tripels in wereldcoordinaten (m, Y-up). */
  positions: Float32Array;
  indices: Uint32Array;
  /** Kleur van het grootste deel (meeste driehoeken), `#rrggbb`. */
  color: string;
  /** Doorzicht van het grootste deel; 1 = dekkend. */
  opacity: number;
  /** Per kleur en doorzicht; de viewer tekent elk deel met zijn eigen materiaal. */
  parts: BengShellPart[];
  info: BengVlakInfo | null;
}

export interface LoadOptions {
  /** Map met web-ifc.wasm (browser: "/wasm/"); in node niet nodig. */
  wasmPath?: string;
  /** Model naar de oorsprong schuiven (default true). */
  coordinateToOrigin?: boolean;
}

const FLOATS_PER_VERTEX = 6; // x y z nx ny nz

function toHex(c: number): string {
  return Math.round(Math.min(1, Math.max(0, c)) * 255)
    .toString(16)
    .padStart(2, "0");
}

/** Pas een kolom-major 4x4 toe op (x, y, z). */
function transform(m: ArrayLike<number>, x: number, y: number, z: number): [number, number, number] {
  return [
    m[0]! * x + m[4]! * y + m[8]! * z + m[12]!,
    m[1]! * x + m[5]! * y + m[9]! * z + m[13]!,
    m[2]! * x + m[6]! * y + m[10]! * z + m[14]!,
  ];
}

/** Laad het schilmodel; gooit bij een onleesbaar bestand. */
export async function loadBengShell(bytes: Uint8Array, opts: LoadOptions = {}): Promise<BengShellElement[]> {
  const api = new IfcAPI();
  if (opts.wasmPath) api.SetWasmPath(opts.wasmPath, true);
  await api.Init();
  const modelId = api.OpenModel(bytes, { COORDINATE_TO_ORIGIN: opts.coordinateToOrigin ?? true });
  if (modelId < 0) throw new Error("BENG-schilmodel is geen leesbaar IFC.");
  try {
    type Acc = { pos: number[]; idx: number[]; color: string; opacity: number };
    // expressID -> (kleur|doorzicht) -> stukken
    const parts = new Map<number, Map<string, Acc>>();
    api.StreamAllMeshes(modelId, (mesh) => {
      const geoms = mesh.geometries;
      let byStyle = parts.get(mesh.expressID);
      if (!byStyle) {
        byStyle = new Map();
        parts.set(mesh.expressID, byStyle);
      }
      for (let i = 0; i < geoms.size(); i++) {
        const pg = geoms.get(i);
        const color = `#${toHex(pg.color.x)}${toHex(pg.color.y)}${toHex(pg.color.z)}`;
        const opacity = pg.color.w;
        const styleKey = `${color}|${opacity}`;
        let acc = byStyle.get(styleKey);
        if (!acc) {
          acc = { pos: [], idx: [], color, opacity };
          byStyle.set(styleKey, acc);
        }
        const g = api.GetGeometry(modelId, pg.geometryExpressID);
        const v = api.GetVertexArray(g.GetVertexData(), g.GetVertexDataSize());
        const ix = api.GetIndexArray(g.GetIndexData(), g.GetIndexDataSize());
        const base = acc.pos.length / 3;
        for (let k = 0; k < v.length; k += FLOATS_PER_VERTEX) {
          acc.pos.push(...transform(pg.flatTransformation, v[k]!, v[k + 1]!, v[k + 2]!));
        }
        for (let k = 0; k < ix.length; k++) acc.idx.push(base + ix[k]!);
        g.delete();
      }
    });

    const out: BengShellElement[] = [];
    for (const [expressId, byStyle] of parts) {
      if (byStyle.size === 0) continue;
      const psets = (await api.properties.getPropertySets(modelId, expressId, true)) as RawPropertySet[];
      const line = api.GetLine(modelId, expressId) as { Name?: { value?: unknown } | null };
      const elParts: BengShellPart[] = [...byStyle.values()].map((a) => ({
        positions: new Float32Array(a.pos),
        indices: new Uint32Array(a.idx),
        color: a.color,
        opacity: a.opacity,
      }));
      const main = elParts.reduce((m, p) => (p.indices.length > m.indices.length ? p : m));
      // Alle stukken samen, voor kader en tests.
      const positions = new Float32Array(elParts.reduce((n, p) => n + p.positions.length, 0));
      const indices = new Uint32Array(elParts.reduce((n, p) => n + p.indices.length, 0));
      let po = 0;
      let io = 0;
      for (const p of elParts) {
        positions.set(p.positions, po);
        for (let k = 0; k < p.indices.length; k++) indices[io + k] = p.indices[k]! + po / 3;
        po += p.positions.length;
        io += p.indices.length;
      }
      out.push({
        expressId,
        ifcType: api.GetNameFromTypeCode(api.GetLineType(modelId, expressId)),
        name: typeof line?.Name?.value === "string" ? line.Name.value : "",
        positions,
        indices,
        color: main.color,
        opacity: main.opacity,
        parts: elParts,
        info: vlakInfoFromPsets(psets ?? []),
      });
    }
    out.sort((x, y) => x.expressId - y.expressId);
    return out;
  } finally {
    api.CloseModel(modelId);
  }
}
