# -*- coding: utf-8 -*-
"""Schemacontrole van gegenereerde IFC's (O-2026-10-30-14).

Per bestand: ifcopenshell.validate met express rules = 0 fouten; daarnaast het
verwachte aantal clearance-proxies en 'Clearance'-representaties.

Gebruik: python -I valideer.py <bestand.ifc>:<proxies>:<representaties> ...
Exit 0 = alles in orde; de uitvoer is per bestand een regel JSON.
"""
from __future__ import annotations

import json
import logging
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.shape
import ifcopenshell.validate

M3_NAAR_DM3 = 1000.0  # ifcopenshell.geom geeft altijd meters


def controleer(pad: str, proxies: int, reps: int) -> dict:
    f = ifcopenshell.open(pad)
    logger = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, logger, express_rules=True)
    n_proxy = len([p for p in f.by_type("IfcBuildingElementProxy") if p.Name == "Clearance"])
    n_rep = len([r for r in f.by_type("IfcShapeRepresentation") if r.RepresentationIdentifier == "Clearance"])
    instellingen = ifcopenshell.geom.settings()
    volumes = []
    for p in f.by_type("IfcBuildingElementProxy"):
        vorm = ifcopenshell.geom.create_shape(instellingen, p)
        volumes.append(round(ifcopenshell.util.shape.get_volume(vorm.geometry) * M3_NAAR_DM3, 1))
    fouten = [str(s.get("message", s))[:200] for s in logger.statements]
    return {
        "bestand": pad,
        "schemafouten": len(fouten),
        "fouten": fouten[:5],
        "proxies": n_proxy,
        "proxy_volume_dm3": volumes,
        "clearance_representaties": n_rep,
        "ok": not fouten and n_proxy == proxies and n_rep == reps and all(v > 0 for v in volumes),
    }


def main() -> int:
    logging.disable(logging.WARNING)
    ok = True
    for arg in sys.argv[1:]:
        pad, proxies, reps = arg.rsplit(":", 2)
        r = controleer(pad, int(proxies), int(reps))
        print(json.dumps(r, ensure_ascii=False))
        ok &= r["ok"]
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
