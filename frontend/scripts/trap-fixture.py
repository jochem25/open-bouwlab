# -*- coding: utf-8 -*-
"""Maak de frontend-fixture van de trapontwerper uit de CLI-uitvoer.

Gebruik (vanuit de repo-root):
    cargo run -q -p trap-core --bin trap -- uitvoer <invoer.json> --tijd 2026-10-30T00:00:00 > uit.json
    python -I frontend/scripts/trap-fixture.py uit.json frontend/src/components/trap/__fixtures__/kwart.uitvoer.json

De IFC-teksten worden ingekort (de tests renderen ze niet) en getallen op 0,01 mm
afgerond, zodat de fixture klein en leesbaar blijft.
"""
from __future__ import annotations

import json
import sys

DECIMALEN = 2


def rond(x: object) -> object:
    if isinstance(x, float):
        return round(x, DECIMALEN)
    if isinstance(x, list):
        return [rond(i) for i in x]
    if isinstance(x, dict):
        return {k: rond(v) for k, v in x.items()}
    return x


def main() -> int:
    bron, doel = sys.argv[1], sys.argv[2]
    with open(bron, encoding="utf-8") as f:
        d = json.load(f)
    d["ifc"] = "ISO-10303-21; (ingekort)"
    d["ifc_leverancier"] = "ISO-10303-21; (ingekort, leverancier)"
    with open(doel, "w", encoding="utf-8", newline="\n") as f:
        json.dump(rond(d), f, ensure_ascii=False)
        f.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
