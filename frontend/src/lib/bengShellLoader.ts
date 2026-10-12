/**
 * BENG-schilmodel laden met web-ifc (zonder Fragments): per element de
 * driehoeken in wereldcoordinaten, de kleur uit het IFC en `Pset_KBA_BENG`.
 *
 * web-ifc zet de geometrie al om naar meters en Y-up: wereld = (x, z, -y).
 * Het schilmodel staat in het stelsel van het bron-IFC (afspraak met
 * ifc-ruimtebalans, 12-10-2026), vaak RD (>100 km). Daarom standaard
 * COORDINATE_TO_ORIGIN: precisie in float32; de BENG-weergave staat los van de
 * warmteverliesvlakken (schakelaar), een overlay is niet nodig.
 */
import { IfcAPI } from "web-ifc";

import { vlakInfoFromPsets, type BengVlakInfo, type RawPropertySet } from "./bengShell";

export interface BengShellElement {
  expressId: number;
  /** IFC-klasse, bv. IfcBuildingElementProxy. */
  ifcType: string;
  name: string;
  /** xyz-tripels in wereldcoordinaten (m, Y-up). */
  positions: Float32Array;
  indices: Uint32Array;
  /** Kleur uit het IFC, `#rrggbb`. */
  color: string;
  /** 1 = dekkend. */
  opacity: number;
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
    const parts = new Map<
      number,
      { pos: number[]; idx: number[]; color: [number, number, number, number] }
    >();
    api.StreamAllMeshes(modelId, (mesh) => {
      const geoms = mesh.geometries;
      for (let i = 0; i < geoms.size(); i++) {
        const pg = geoms.get(i);
        const g = api.GetGeometry(modelId, pg.geometryExpressID);
        const v = api.GetVertexArray(g.GetVertexData(), g.GetVertexDataSize());
        const ix = api.GetIndexArray(g.GetIndexData(), g.GetIndexDataSize());
        let part = parts.get(mesh.expressID);
        if (!part) {
          part = { pos: [], idx: [], color: [pg.color.x, pg.color.y, pg.color.z, pg.color.w] };
          parts.set(mesh.expressID, part);
        }
        const base = part.pos.length / 3;
        for (let k = 0; k < v.length; k += FLOATS_PER_VERTEX) {
          part.pos.push(...transform(pg.flatTransformation, v[k]!, v[k + 1]!, v[k + 2]!));
        }
        for (let k = 0; k < ix.length; k++) part.idx.push(base + ix[k]!);
        g.delete();
      }
    });

    const out: BengShellElement[] = [];
    for (const [expressId, part] of parts) {
      const psets = (await api.properties.getPropertySets(modelId, expressId, true)) as RawPropertySet[];
      const line = api.GetLine(modelId, expressId) as { Name?: { value?: unknown } | null };
      const [r, g, b, a] = part.color;
      out.push({
        expressId,
        ifcType: api.GetNameFromTypeCode(api.GetLineType(modelId, expressId)),
        name: typeof line?.Name?.value === "string" ? line.Name.value : "",
        positions: new Float32Array(part.pos),
        indices: new Uint32Array(part.idx),
        color: `#${toHex(r)}${toHex(g)}${toHex(b)}`,
        opacity: a,
        info: vlakInfoFromPsets(psets ?? []),
      });
    }
    out.sort((x, y) => x.expressId - y.expressId);
    return out;
  } finally {
    api.CloseModel(modelId);
  }
}
