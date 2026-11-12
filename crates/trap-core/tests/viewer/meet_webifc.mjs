// Spike O-2026-10-30-02: meet per element wat web-ifc (versie van de Open Bouwlab-viewer,
// 0.0.77) van een IFC maakt: volume (mm3), bbox (mm, Z-up) en open randen (0 = gesloten mesh).
// Vergelijk met ifcopenshell (tests/golden/vergelijk.py) om weergavefouten te vinden.
// Gebruik: node meet_webifc.mjs <frontend/node_modules/web-ifc/web-ifc-api-node.js> <bestand.ifc> <uit.json>
import fs from "node:fs";
import { createRequire } from "node:module";
const [, , apiPad, ifcPad, uitPad] = process.argv;
const require = createRequire(import.meta.url);
const WebIFC = require(apiPad);
const api = new WebIFC.IfcAPI();
await api.Init();
const id = api.OpenModel(new Uint8Array(fs.readFileSync(ifcPad)), { COORDINATE_TO_ORIGIN: false });
const res = {};
function mul(m, p) {
  return [m[0]*p[0]+m[4]*p[1]+m[8]*p[2]+m[12], m[1]*p[0]+m[5]*p[1]+m[9]*p[2]+m[13], m[2]*p[0]+m[6]*p[1]+m[10]*p[2]+m[14]];
}
api.StreamAllMeshes(id, (mesh) => {
  const line = api.GetLine(id, mesh.expressID);
  const naam = `${api.GetNameFromTypeCode(line.type)}|${line.Name?.value}`;
  const r = res[naam] ?? (res[naam] = { volume_mm3: 0, driehoeken: 0, geometrieen: 0, kleuren: [], min: [1e99,1e99,1e99], max: [-1e99,-1e99,-1e99] });
  for (let i = 0; i < mesh.geometries.size(); i++) {
    const pg = mesh.geometries.get(i);
    const g = api.GetGeometry(id, pg.geometryExpressID);
    const v = api.GetVertexArray(g.GetVertexData(), g.GetVertexDataSize());
    const ix = api.GetIndexArray(g.GetIndexData(), g.GetIndexDataSize());
    const m = pg.flatTransformation;
    const P = [];
    for (let k = 0; k < v.length; k += 6) {
      const w = mul(m, [v[k], v[k+1], v[k+2]]); // web-ifc: meters, Y-up -> terug naar Z-up mm
      const q = [w[0]*1000, -w[2]*1000, w[1]*1000];
      P.push(q);
      for (let a = 0; a < 3; a++) { r.min[a] = Math.min(r.min[a], q[a]); r.max[a] = Math.max(r.max[a], q[a]); }
    }
    let vol = 0;
    for (let k = 0; k < ix.length; k += 3) {
      const [a, b, c] = [P[ix[k]], P[ix[k+1]], P[ix[k+2]]];
      vol += (a[0]*(b[1]*c[2]-b[2]*c[1]) - a[1]*(b[0]*c[2]-b[2]*c[0]) + a[2]*(b[0]*c[1]-b[1]*c[0])) / 6;
    }
    r.volume_mm3 += vol; { const key = (p) => p.map(x => Math.round(x*100)).join(","); const E = new Map(); for (let k = 0; k < ix.length; k += 3) { const t = [key(P[ix[k]]), key(P[ix[k+1]]), key(P[ix[k+2]])]; for (let j = 0; j < 3; j++) { const a = t[j], b = t[(j+1)%3]; const e = a < b ? a+"|"+b : b+"|"+a; E.set(e, (E.get(e) ?? 0) + 1); } } r.open_randen = (r.open_randen ?? 0) + [...E.values()].filter(n => n !== 2).length; r.pts = P.length; } r.driehoeken += ix.length / 3; r.geometrieen += 1;
    r.kleuren.push([pg.color.x, pg.color.y, pg.color.z, pg.color.w].map(x => +x.toFixed(3)));
    g.delete();
  }
});
// fouten/waarschuwingen van web-ifc
res.__fouten = api.GetAllErrors ? JSON.stringify(api.GetAllErrors(id)) : "n.v.t.";
fs.writeFileSync(uitPad, JSON.stringify(res, null, 1));
console.log(ifcPad, Object.keys(res).length - 1, "elementen met mesh");
api.CloseModel(id);
