"""Synthetisch BENG-schilmodel voor de viewer-tests (geen klantdata).

Volgt het voorstel aan ifc-ruimtebalans (12-10-2026, msg_6f05c7a7cc9c): een
vlak per gevel_id als IfcBuildingElementProxy met kleur (IfcStyledItem) en
Pset_KBA_BENG; ramen als eigen element met window_def; een belending grijs en
transparant zonder BENG-velden behalve vlak_type 'belending'.

Tussenwoning 5 x 8 m, hoogte 5,6 m, eenheid mm. Draaien met een Python waarin
ifcopenshell staat (bv. de venv van ifc-ruimtebalans):

    python scripts/generate_beng_schil_synthetisch.py

Schrijft frontend/src/lib/__fixtures__/beng-schil-synthetisch.ifc.
"""

from __future__ import annotations

from pathlib import Path

import ifcopenshell
import ifcopenshell.guid

UIT = Path(__file__).resolve().parent.parent / (
    "frontend/src/lib/__fixtures__/beng-schil-synthetisch.ifc"
)
B, D, H = 5000.0, 8000.0, 5600.0

# (r, g, b, transparantie)
KLEUR = {
    "grond": (0.57, 0.25, 0.05, 0.0),
    "buitenlucht_Z": (0.94, 0.33, 0.20, 0.0),
    "buitenlucht_N": (0.15, 0.39, 0.92, 0.0),
    "buitenlucht_horizontaal": (0.55, 0.55, 0.85, 0.0),
    "belending": (0.09, 0.64, 0.29, 0.0),
    "belending_deels": (0.85, 0.65, 0.10, 0.0),
    "raam": (0.60, 0.85, 1.0, 0.3),
    "belending_element": (0.6, 0.6, 0.6, 0.7),
}

# gevel_id, vlak_type, grenst_aan, orientatie, helling, A_T, constructie, kleur,
# buitenrand, gaten, extra
VLAKKEN = [
    ("g-rz-1-01", "vloer", "vloer_op_maaiveld_boven_grond", "", 0.0, 40.0,
     "od-vloer", "grond",
     [(0, 0, 0), (B, 0, 0), (B, D, 0), (0, D, 0)], [], {}),
    ("g-rz-1-02", "dak", "buitenlucht", "horizontaal", 0.0, 40.0,
     "od-dak", "buitenlucht_horizontaal",
     [(0, 0, H), (0, D, H), (B, D, H), (B, 0, H)], [], {}),
    ("g-rz-1-03", "gevel", "buitenlucht", "Z", 90.0, 28.0,
     "od-gevel", "buitenlucht_Z",
     [(0, 0, 0), (0, 0, H), (B, 0, H), (B, 0, 0)],
     [[(1000, 0, 800), (1000, 0, 2400), (4000, 0, 2400), (4000, 0, 800)]], {}),
    ("g-rz-1-04", "gevel", "buitenlucht", "N", 90.0, 28.0,
     "od-gevel", "buitenlucht_N",
     [(0, D, 0), (B, D, 0), (B, D, H), (0, D, H)], [], {}),
    ("g-rz-1-05", "gevel", "aangrenzende_verwarmde_ruimte", "W", 90.0, 44.8,
     "od-wws", "belending",
     [(0, 0, 0), (0, D, 0), (0, D, H), (0, 0, H)], [], {"omschrijving": "belending"}),
    ("g-rz-1-06", "gevel", "aangrenzende_verwarmde_ruimte", "O", 90.0, 44.8,
     "od-wws", "belending_deels",
     [(B, 0, 0), (B, 0, H), (B, D, H), (B, D, 0)], [],
     {"omschrijving": "belending", "bedekking": "deels"}),
]

RAMEN = [
    ("wd-A", "g-rz-1-03", 4.8,
     [(1000, 0, 800), (4000, 0, 800), (4000, 0, 2400), (1000, 0, 2400)]),
]


def main() -> None:
    f = ifcopenshell.file(schema="IFC4")
    oorsprong = f.createIfcAxis2Placement3D(f.createIfcCartesianPoint((0.0, 0.0, 0.0)))
    ctx = f.createIfcGeometricRepresentationContext(
        None, "Model", 3, 1e-5, oorsprong, f.createIfcDirection((0.0, 1.0))
    )
    body = f.createIfcGeometricRepresentationSubContext(
        "Body", "Model", None, None, None, None, ctx, None, "MODEL_VIEW", None
    )
    eenheden = f.createIfcUnitAssignment([
        f.createIfcSIUnit(None, "LENGTHUNIT", "MILLI", "METRE"),
        f.createIfcSIUnit(None, "AREAUNIT", None, "SQUARE_METRE"),
        f.createIfcSIUnit(None, "PLANEANGLEUNIT", None, "RADIAN"),
    ])
    project = f.createIfcProject(
        ifcopenshell.guid.new(), None, "beng-schil synthetisch", None, None, None,
        None, [ctx], eenheden,
    )
    plaats = f.createIfcLocalPlacement(None, oorsprong)
    gebouw = f.createIfcBuilding(ifcopenshell.guid.new(), None, "tussenwoning",
                                 None, None, plaats)
    f.createIfcRelAggregates(ifcopenshell.guid.new(), None, None, None, project, [gebouw])

    stijlen = {}
    for naam, (r, g, b, t) in KLEUR.items():
        kleur = f.createIfcColourRgb(None, r, g, b)
        rendering = f.createIfcSurfaceStyleRendering(kleur, t, None, None, None, None,
                                                     None, None, "NOTDEFINED")
        stijlen[naam] = f.createIfcSurfaceStyle(naam, "BOTH", [rendering])

    elementen = []

    def vlak(naam: str, rand, gaten, stijl: str, props: dict) -> None:
        def lus(punten):
            return f.createIfcPolyLoop(
                [f.createIfcCartesianPoint(tuple(float(c) for c in p)) for p in punten]
            )

        grenzen = [f.createIfcFaceOuterBound(lus(rand), True)]
        grenzen += [f.createIfcFaceBound(lus(g), True) for g in gaten]
        schil = f.createIfcOpenShell([f.createIfcFace(grenzen)])
        item = f.createIfcShellBasedSurfaceModel([schil])
        f.createIfcStyledItem(item, [stijlen[stijl]], None)
        rep = f.createIfcShapeRepresentation(body, "Body", "SurfaceModel", [item])
        el = f.createIfcBuildingElementProxy(
            ifcopenshell.guid.new(), None, naam, None, None, plaats,
            f.createIfcProductDefinitionShape(None, None, [rep]), None, None,
        )
        waarden = []
        for k, v in props.items():
            if isinstance(v, float):
                w = f.createIfcReal(v)
            else:
                w = f.createIfcLabel(str(v)) if len(str(v)) < 255 else f.createIfcText(str(v))
            waarden.append(f.createIfcPropertySingleValue(k, None, w, None))
        pset = f.createIfcPropertySet(ifcopenshell.guid.new(), None, "Pset_KBA_BENG",
                                      None, waarden)
        f.createIfcRelDefinesByProperties(ifcopenshell.guid.new(), None, None, None,
                                          [el], pset)
        elementen.append(el)

    for (gid, vt, ga, ori, hel, at, cref, stijl, rand, gaten, extra) in VLAKKEN:
        props = {
            "gevel_id": gid, "zone": "rz-1", "vlak_type": vt, "grenst_aan": ga,
            "orientatie": ori, "helling_deg": hel, "A_T_m2": at, "constructie_ref": cref,
            "herkomst": "ifc_afgeleid: NTA 8800 K.1.2/K.1.4 binnenwerks (synthetisch)",
        }
        props.update(extra)
        vlak(gid, rand, gaten, stijl, props)
    for wd, host, at, rand in RAMEN:
        vlak(wd, rand, [], "raam", {
            "window_def": wd, "gevel_id": host, "vlak_type": "raam", "A_T_m2": at,
            "herkomst": "ifc_afgeleid: IfcWindow (synthetisch)",
        })
    vlak("belending west", [(-300, -200, 0), (-300, D + 200, 0), (-300, D + 200, H + 300),
                            (-300, -200, H + 300)], [], "belending_element",
         {"vlak_type": "belending"})

    f.createIfcRelContainedInSpatialStructure(ifcopenshell.guid.new(), None, None, None,
                                              elementen, gebouw)
    UIT.parent.mkdir(parents=True, exist_ok=True)
    f.write(str(UIT))
    print(UIT, UIT.stat().st_size, "bytes")


if __name__ == "__main__":
    main()
