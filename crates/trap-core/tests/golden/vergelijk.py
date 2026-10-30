# -*- coding: utf-8 -*-
"""Golden test O-2026-10-30-02/-03: Rust-IFC tegen de Python-referentie (v5).

ifcopenshell is hier alleen test-oracle. Toetst:
1. schemafouten (ifcopenshell.validate, incl. express rules) = 0 in beide;
2. aantallen per IFC-klasse gelijk (verschillen worden gemeld; de Rust-uitvoer
   heeft bewust een pset KBA_Status met de disclaimer extra);
3. per onderdeel (op naam) volume en bounding box gelijk binnen 0,1 mm3 / 0,1 mm,
   ook voor de 'Clearance'-representatie van de IfcStair.

Gebruik: python -I vergelijk.py <referentie.ifc> <rust.ifc> [uitvoer.json]
        [--anders <deel van naam>]...
--anders: onderdelen die bewust afwijken (fase 2: 'buitenboom', koude hoek i.p.v.
verstek). Die worden gemeld, niet getoetst; klasseverschillen worden dan alleen
gemeld (het verstek zat in IfcBooleanClippingResult/IfcHalfSpaceSolid).
Exit 0 = geslaagd.
"""
from __future__ import annotations

import json
import sys
from collections import Counter

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.shape
import ifcopenshell.validate
import numpy as np

M_NAAR_MM = 1000.0  # ifcopenshell.geom geeft altijd meters
TOL_VOLUME_MM3 = 0.1
TOL_BBOX_MM = 0.1
# bewuste uitbreiding t.o.v. v5: pset KBA_Status (1 pset, 1 rel, 1 waarde)
VERWACHT_EXTRA = {"IfcPropertySet": 1, "IfcRelDefinesByProperties": 1,
                  "IfcPropertySingleValue": 1}


def schemafouten(f) -> list[str]:
    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, log, express_rules=True)
    return [str(s)[:300] for s in log.statements]


def maat(settings, obj) -> dict:
    vorm = ifcopenshell.geom.create_shape(settings, obj)
    g = vorm.geometry if hasattr(vorm, "geometry") else vorm  # vasthouden (lesson 29-10)
    v = np.asarray(g.verts, dtype=float).reshape(-1, 3) * M_NAAR_MM
    return {"volume_mm3": ifcopenshell.util.shape.get_volume(g) * M_NAAR_MM**3,
            "bbox_min": v.min(axis=0).tolist(), "bbox_max": v.max(axis=0).tolist(),
            "hoekpunten": len(v)}


def onderdelen(f) -> dict[str, dict]:
    s = ifcopenshell.geom.settings()
    s.set("use-world-coords", True)
    uit = {}
    for el in f.by_type("IfcBuildingElementPart") + f.by_type("IfcMember"):
        uit[f"{el.is_a()}|{el.Name}"] = maat(s, el)
    lokaal = ifcopenshell.geom.settings()
    for trap in f.by_type("IfcStair"):
        for rep in trap.Representation.Representations:
            if rep.RepresentationIdentifier == "Clearance":
                uit["IfcStair|Clearance"] = maat(lokaal, rep.Items[0])  # één trap per bestand
    return uit


def main(ref_pad: str, rust_pad: str, uit_pad: str | None, anders: list[str]) -> int:
    ref, rust = ifcopenshell.open(ref_pad), ifcopenshell.open(rust_pad)
    res: dict = {"referentie": ref_pad, "rust": rust_pad,
                 "ifcopenshell": ifcopenshell.version}
    res["schemafouten"] = {"referentie": schemafouten(ref), "rust": schemafouten(rust)}
    a = Counter(e.is_a() for e in ref)
    b = Counter(e.is_a() for e in rust)
    verschil = {k: b[k] - a[k] for k in sorted(set(a) | set(b)) if b[k] != a[k]}
    res["aantallen"] = {"referentie": dict(sorted(a.items())), "rust": dict(sorted(b.items()))}
    res["aantallen_verschil (rust - ref)"] = verschil
    res["aantallen_onverwacht"] = {} if anders else {
        k: v for k, v in verschil.items() if VERWACHT_EXTRA.get(k) != v}
    res["bewust_anders"] = anders
    od_ref, od_rust = onderdelen(ref), onderdelen(rust)
    rijen, fout, gemeld = [], [], []
    for naam in sorted(set(od_ref) | set(od_rust)):
        r, q = od_ref.get(naam), od_rust.get(naam)
        if r is None or q is None:
            fout.append(f"{naam}: ontbreekt in {'rust' if q is None else 'referentie'}")
            continue
        dv = q["volume_mm3"] - r["volume_mm3"]
        db = max(abs(x - y) for x, y in zip(q["bbox_min"] + q["bbox_max"],
                                            r["bbox_min"] + r["bbox_max"]))
        rij = {"onderdeel": naam, "volume_ref_mm3": round(r["volume_mm3"], 4),
               "volume_rust_mm3": round(q["volume_mm3"], 4), "dvolume_mm3": dv, "dbbox_mm": db}
        if any(a in naam for a in anders):
            gemeld.append(rij)
            continue
        rijen.append(rij)
        if abs(dv) > TOL_VOLUME_MM3 or db > TOL_BBOX_MM or r["volume_mm3"] <= 0:
            fout.append(f"{naam}: dV {dv:.6f} mm3, dbbox {db:.6f} mm, Vref {r['volume_mm3']:.1f}")
    res["onderdelen"] = rijen
    res["bewust_anders_gemeld"] = gemeld
    res["max |dvolume| mm3"] = max(abs(x["dvolume_mm3"]) for x in rijen)
    res["max dbbox mm"] = max(x["dbbox_mm"] for x in rijen)
    res["fouten"] = fout + [f"schemafouten {k}: {len(v)}" for k, v in res["schemafouten"].items() if v] \
        + [f"aantal {k}: {v:+d}" for k, v in res["aantallen_onverwacht"].items()]
    res["geslaagd"] = not res["fouten"]
    tekst = json.dumps(res, indent=1, ensure_ascii=True)
    if uit_pad:
        open(uit_pad, "w", encoding="utf-8").write(tekst)
    print(json.dumps({k: res[k] for k in ("schemafouten", "aantallen_verschil (rust - ref)",
                                          "max |dvolume| mm3", "max dbbox mm", "fouten",
                                          "bewust_anders_gemeld", "geslaagd")}, indent=1))
    print("onderdelen vergeleken:", len(rijen))
    return 0 if res["geslaagd"] else 1


if __name__ == "__main__":
    args, anders = [], []
    it = iter(sys.argv[1:])
    for a in it:
        if a == "--anders":
            anders.append(next(it))
        else:
            args.append(a)
    sys.exit(main(args[0], args[1], args[2] if len(args) > 2 else None, anders))
