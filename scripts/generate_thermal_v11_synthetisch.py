"""Genereer de synthetische thermal-import v1.1-geometriefixture.

Vervangt een fixture die uit een echt project kwam (publieke repo, geen
klantdata). Het gebouw is een rechthoekig raster van ruimten op drie
verdiepingen; elke wand, vloer en dak draagt vertices zoals de Revit-raycast
exporter ze levert. Deterministisch: zelfde uitvoer bij elke run.

Gebruik:
    python scripts/generate_thermal_v11_synthetisch.py
(schrijft tests/fixtures/thermal-import-v11-geometry.json, LF, ASCII)
"""

from __future__ import annotations

import json
from pathlib import Path

COLS = 4  # ruimten in x-richting
ROWS = 3  # ruimten in y-richting
FLOORS = 3
DX = 3.6  # m
DY = 4.2  # m
H = 2.8  # m
TRUE_NORTH_DEG = 46.0
OUT = (
    Path(__file__).resolve().parent.parent
    / "tests/fixtures/thermal-import-v11-geometry.json"
)
UNHEATED = {(0, 0, 0), (3, 2, 0), (0, 2, 2)}  # (col, row, floor) bergingen

# Kompas bij true north 46: gevel -y = ZW, +x = ZO, +y = NO, -x = NW.
COMPASS = {"-y": "SW", "+x": "SE", "+y": "NE", "-x": "NW"}

LAYERS_EXT_WALL = [
    {"type": "solid", "thickness_mm": 60.0, "material": "hout_vuren_generiek"},
    {"type": "solid", "thickness_mm": 140.0, "material": "n7_isolatie_PIR"},
    {"type": "solid", "thickness_mm": 18.0, "material": "hout_vuren_generiek"},
]
LAYERS_INT_WALL = [
    {"type": "solid", "thickness_mm": 100.0, "material": "hout_vuren_generiek"},
]
LAYERS_FLOOR = [
    {"type": "solid", "thickness_mm": 200.0, "material": "hout_vuren_generiek"},
]
LAYERS_GROUND = [
    {"type": "solid", "thickness_mm": 120.0, "material": "hout_vuren_generiek"},
    {"type": "solid", "thickness_mm": 120.0, "material": "n7_isolatie_resol"},
]
LAYERS_ROOF = [
    {"type": "solid", "thickness_mm": 160.0, "material": "hout_vuren_generiek"},
    {"type": "solid", "thickness_mm": 120.0, "material": "n7_isolatie_PIR"},
    {"type": "solid", "thickness_mm": 2.0, "material": "h7_zink_dak_fels"},
]


def r3(v: float) -> float:
    return round(v, 3)


def room_id(c: int, r: int, f: int) -> str:
    return f"room-{f * COLS * ROWS + r * COLS + c + 1}"


def main() -> None:
    rooms: list[dict] = [
        {"id": "outside", "type": "outside", "name": "Buiten"},
        {"id": "ground", "type": "ground", "name": "Grond"},
    ]
    constructions: list[dict] = []
    openings: list[dict] = []

    def add_con(room_a, room_b, orientation, compass, vertices, layers, type_name, area):
        cid = f"con-{len(constructions) + 1}"
        constructions.append(
            {
                "id": cid,
                "room_a": room_a,
                "room_b": room_b,
                "orientation": orientation,
                "compass": compass,
                "gross_area_m2": r3(area),
                "layers": layers,
                "vertices": [[r3(x), r3(y), r3(z)] for x, y, z in vertices],
                "revit_type_name": type_name,
            }
        )
        return cid

    def add_opening(con_id, kind, x0, y0, x1, y1, z0, width, height, type_name):
        oid = f"open-{len(openings) + 1}"
        openings.append(
            {
                "id": oid,
                "construction_id": con_id,
                "type": kind,
                "width_mm": width * 1000.0,
                "height_mm": height * 1000.0,
                "sill_height_mm": z0 * 1000.0,
                "vertices": [
                    [r3(x0), r3(y0), r3(z0)],
                    [r3(x0), r3(y0), r3(z0 + height)],
                    [r3(x1), r3(y1), r3(z0 + height)],
                    [r3(x1), r3(y1), r3(z0)],
                ],
                "revit_type_name": type_name,
            }
        )

    for f in range(FLOORS):
        z0, z1 = f * H, (f + 1) * H
        for r in range(ROWS):
            for c in range(COLS):
                rid = room_id(c, r, f)
                x0, y0 = c * DX, r * DY
                x1, y1 = x0 + DX, y0 + DY
                heated = (c, r, f) not in UNHEATED
                n = f * COLS * ROWS + r * COLS + c + 1
                rooms.append(
                    {
                        "id": rid,
                        "name": f"{n}. {'Verblijfsruimte' if heated else 'Berging'}",
                        "type": "heated" if heated else "unheated",
                        "level": f"{f:02d}_verdieping",
                        "area_m2": r3(DX * DY),
                        "height_m": H,
                        "volume_m3": r3(DX * DY * H),
                        "boundary_polygon": [
                            [r3(x0), r3(y0)],
                            [r3(x1), r3(y0)],
                            [r3(x1), r3(y1)],
                            [r3(x0), r3(y1)],
                        ],
                        "revit_id": 1000000 + n,
                    }
                )
                rect = [(x0, y0, None), (x1, y0, None), (x1, y1, None), (x0, y1, None)]

                # Vloer: op grond (begane grond) of boven de ruimte eronder.
                below = "ground" if f == 0 else room_id(c, r, f - 1)
                add_con(
                    rid, below, "floor", None,
                    [(x, y, z0) for x, y, _ in rect],
                    LAYERS_GROUND if f == 0 else LAYERS_FLOOR,
                    "vloer_grond" if f == 0 else "verdiepingsvloer",
                    DX * DY,
                )
                if f == FLOORS - 1:
                    add_con(
                        rid, "outside", "roof", None,
                        [(x, y, z1) for x, y, _ in rect],
                        LAYERS_ROOF, "dak", DX * DY,
                    )

                # Gevels: elke buitenwand in twee segmenten (zoals de raycast ze splitst).
                facades = []
                if r == 0:
                    facades.append(("-y", (x0, y0), (x1, y0)))
                if r == ROWS - 1:
                    facades.append(("+y", (x1, y1), (x0, y1)))
                if c == 0:
                    facades.append(("-x", (x0, y1), (x0, y0)))
                if c == COLS - 1:
                    facades.append(("+x", (x1, y0), (x1, y1)))
                for side, (ax, ay), (bx, by) in facades:
                    mx, my = (ax + bx) / 2, (ay + by) / 2
                    seg_len = ((bx - ax) ** 2 + (by - ay) ** 2) ** 0.5 / 2
                    for i, ((px, py), (qx, qy)) in enumerate(
                        (((ax, ay), (mx, my)), ((mx, my), (bx, by)))
                    ):
                        cid = add_con(
                            rid, "outside", "wall", COMPASS[side],
                            [(px, py, z0), (qx, qy, z0), (qx, qy, z1), (px, py, z1)],
                            LAYERS_EXT_WALL, "buitenwand", seg_len * H,
                        )
                        if i == 0 and heated:
                            # Raam van 1,2 x 1,4 m midden in het eerste segment.
                            t0 = (seg_len - 1.2) / 2 / seg_len
                            t1 = 1.0 - t0
                            add_opening(
                                cid, "window",
                                px + (qx - px) * t0, py + (qy - py) * t0,
                                px + (qx - px) * t1, py + (qy - py) * t1,
                                0.9, 1.2, 1.4, "raam_1200x1400",
                            )
                        if i == 1 and f == 0 and side == "-y" and c == 1:
                            add_opening(
                                cid, "door", px + 0.3, py, px + 1.3, py,
                                0.0, 1.0, 2.3, "voordeur_1000x2300",
                            )

                # Binnenwanden: alleen naar de buur in +x en +y (elk paar eenmaal).
                if c < COLS - 1:
                    add_con(
                        rid, room_id(c + 1, r, f), "wall", None,
                        [(x1, y0, z0), (x1, y1, z0), (x1, y1, z1), (x1, y0, z1)],
                        LAYERS_INT_WALL, "binnenwand", DY * H,
                    )
                if r < ROWS - 1:
                    add_con(
                        rid, room_id(c, r + 1, f), "wall", None,
                        [(x1, y1, z0), (x0, y1, z0), (x0, y1, z1), (x1, y1, z1)],
                        LAYERS_INT_WALL, "binnenwand", DX * H,
                    )

    doc = {
        "version": "1.1",
        "source": "synthetisch",
        "project_name": "Synthetisch v1.1-geometrie",
        "exported_at": "2026-10-12T00:00:00",
        "true_north_deg": TRUE_NORTH_DEG,
        "rooms": rooms,
        "constructions": constructions,
        "openings": openings,
        "open_connections": [],
    }
    text = json.dumps(doc, indent=2, ensure_ascii=True) + "\n"
    OUT.write_text(text, encoding="ascii", newline="\n")


if __name__ == "__main__":
    main()
