# Trapontwerper fase 1: IFC-spike (O-2026-10-30-02)

Status: spike, branch `feat/trap-spike`. Niet gemerged.

## Vraag

1. Risico A: kan Rust een IFC4 schrijven dat inhoudelijk gelijk is aan de Python-referentie
   v5 (verdreven kwarttrap: IfcStair met steek, bomen, trappaal, treden, stootborden,
   echte bogen, verstek, nesten, 'Clearance'-representatie)?
2. Risico B: toont de Open Bouwlab-viewer (web-ifc 0.0.77 + @thatopen 3.4.6) dat bestand goed?

## Keuze IFC-aanpak: eigen STEP-P21-schrijver in `crates/trap-core`

| optie | voor | tegen | besluit |
|---|---|---|---|
| hergebruik in de workspace | geen nieuwe code | `isso51-ifcx` schrijft IFCX (JSON, IFC5), geen IFC4-STEP; `ifcStepGenerator.ts` is TypeScript in de frontend | nee |
| `openbim-ifc` (crates.io) | volledig schema | AGPL-3.0: dwingt AGPL af voor de verspreide app; botst met MIT | nee |
| `ifc-lite-*`, `oxideav-ifc` | MPL-2.0 / MIT | lezers/parsers, geen schrijver | nee |
| eigen P21-schrijver | ~1000 regels incl. model, alleen serde/thiserror, MIT, deterministisch | geen schema-controle in Rust | **ja**; schema-controle via ifcopenshell in de tests |

Opbouw: `p21.rs` (STEP-opmaak: reals, strings met `\X2\`, GUID's), `model.rs` (trapmodel =
uitvoer van de toekomstige rekenkern), `ifc.rs` (entiteitkeuze als v5). GUID's zijn afgeleid
van de elementnamen: dezelfde invoer geeft byte-gelijke uitvoer.

## Risico A: go

Golden test `tests/golden/vergelijk.py` (ifcopenshell 0.8.4 alleen als oracle) tegen
`tests/fixtures.local/trap/referentie_kwartdraai_v5.ifc` (lokaal, niet in de repo):

| toets | eis | gemeten |
|---|---|---|
| schemafouten (incl. express rules) | 0 | 0 (referentie en Rust) |
| aantallen per klasse | gelijk | gelijk, op de bewuste pset `KBA_Status` na (+1 pset, +1 rel, +1 waarde) |
| volume per onderdeel (40: 34 delen, 5 leden, clearance) | < 0,1 mm3 | max 8e-5 mm3 |
| bounding box per onderdeel | < 0,1 mm | max 5e-12 mm |
| negatieve controle (1 boog als lijn) | test faalt | faalt: binnenboom deel 1 dV -86 mm3 |

Invoer: `tests/fixtures.local/trap/referentie_kwartdraai_v5.json` (lokaal), geexporteerd uit de Python-keten van v5
(argumenten van `schrijf_delen` afgevangen). De verdrijving zelf rekenen is fase 2.

## Risico B: go met terugval voor het verstek

Gemeten per element: web-ifc-mesh (volume, open randen) tegen ifcopenshell, in Node en in de
browser (`frontend/trap-spike.html`, headless Chrome). Screenshots: zie levering.

| onderdeel | weergave | toelichting |
|---|---|---|
| IfcArcIndex (echte bogen) | goed | binnenboom deel 1/2: gesloten, dV < 0,001 % (tessellatie) |
| IfcBooleanResult (nesten) | goed | 74 nesten, alle meshes gesloten |
| IfcBooleanClippingResult + IfcHalfSpaceSolid (verstek) | **fout** | buitenboom deel 2: 324 open randen, volume +18 % (v5) / -16 % (Rust); deel 1: 24-26 open randen. Van buitenaf niet zichtbaar; fout in doorsneden en hoeveelheden |
| 'Clearance' (IfcFacetedBrep + IfcStyledItem op IfcStair) | goed | beide laadpaden (Fragments en web-ifc direct): rood, doorzicht 0,7 |

Oorzaak verstek: de grootte van de snijder. Een wig van ~6 m faalt net zo; een wig van
2 x dikte + 10 mm niet. web-ifc 0.0.78 is slechter (ook nesten en bogen breken).

Terugval (gebouwd, `IfcOpties.verstek = Verstek::Wig`): verstek als IfcBooleanResult DIFFERENCE
met een compacte wig (IfcExtrudedAreaSolid). IFC blijft exact (ifcopenshell: identiek volume
en bbox), alle 40 web-ifc-meshes gesloten, buitenboom deel 2 dV +392 mm3 (0,001 %).
Nadeel: RepresentationType 'CSG' in plaats van 'Clipping'. Standaard blijft `HalfSpace` (= v5)
tot het besluit valt.

## Open

- Disclaimertekst voor pset `KBA_Status` (plan, 'Status van alle uitvoer'): nu een plaatshouder
  in de test.
- BIMcollab Zoom: niet getest in deze spike (scherm vergrendeld); v5 met dezelfde structuur is
  op 29-10 in Zoom getoond.
