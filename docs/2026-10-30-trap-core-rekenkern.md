# Trapontwerper fase 2: rekenkern trap-core (O-2026-10-30-03)

Status: branch `feat/trap-core` (vanaf `feat/trap-spike`). Niet gemerged.

## Wat de kern doet

Invoer-JSON -> trapmodel -> IFC4 (STEP P21) + toets-JSON. Puur Rust, geen I/O in de kern.

| stap | module | bron / referentie |
|---|---|---|
| invoer + JSON-schema | `invoer.rs`, `schema/trap-invoer.schema.json` | schema gegenereerd uit de types (schemars), `deny_unknown_fields` |
| plattegrond, veelhoeken | `vlak.rs` | eigen exacte bewerkingen (halfvlak met losse stukken, doorsnede met convex, oorknippen) i.p.v. shapely |
| looplijn, verdrijving | `verdrijving.rs` | port `verdrijving_contour.py`; fuikmethode Wattjes (`wattjes-deel8__h2-par7-11` r. 838-870) |
| treden, stootborden, nesten | `onderdelen.rs` | port `trap_3d.py`; nest Wattjes (`__h2-par3-4` r. 162-164) |
| bomen (uitslag, delen) | `boom.rs` | port `boom_profiel.py`; voorhoutcirkels (`__h2-par5-6` r. 165-195) |
| clearance | `clearance.rs` | port `trap_3d.clearance_volume` (IFC4 ADD1 Clearance Geometry) |
| toetsing | `toets.rs` | Bbl uit het normcorpus (kennis-v2, toestand 2026-09-24) |
| samenstellen | `bouw.rs` | psets zoals v5 + `KBA_Status` (disclaimer) |
| CLI | `src/bin/trap.rs` | `trap ontwerp <invoer.json> [--ifc] [--toets] [--tijd]`, `trap schema` |

## Ontwerpbesluiten

| onderwerp | besluit | grond |
|---|---|---|
| hoekverbinding buitenboom | koud (stomp); het HOGERE deel loopt `dikte` door het hoekvierkant, het lagere eindigt op de hoek | besluit Jochem 30-10; Wattjes `__h2-par7-11` r. 141-145 (uitkeping in de hogere boom, lagere met vlakke borst); de leverancierstrap (golden 2) doet hetzelfde |
| boven/onderkant in het hoekvierkant | bij 'hoger': bovenkant daar horizontaal op de hoogte van de hoek; bij 'lager': onderkant horizontaal | zo ligt de borst van het stompe deel geheel tegen het doorlopende deel (getest) |
| binnenhoek | trappaal verplicht; rechthoek met maat langs beide armen en optionele insteek in de loopbreedte | als v3; spil van golden 2 (115 x 68, 14 mm insteek) |
| verstek | vervalt (geen IfcHalfSpaceSolid/IfcBooleanClippingResult meer) | besluit Jochem 30-10; lost ook het web-ifc-probleem uit de spike op |
| nesten | eerst het deel van de loopstrook, dan de randen op een boomzijde `nest` naar buiten | de referentie verbreedt de strook vooraf; in een U-trap hangt dan de nestband van een andere arm aan een trede. Kwartdraai: uitkomst gelijk (golden 1) |
| verdrijfvakken | automatisch per binnenhoek een tusschenkwart met de grootste kleinste binnenaantrede; hoekvoorkant minstens 20 graden van beide armen; voorkant nooit langs de binnenzijde (< 10 graden) | sweep zoals in de referentie (vindt 0-5, 5-15 voor referentietrap) |
| aantal optreden | opgegeven, of: kleinste aantal dat aan de Bbl-optrede voldoet, tot +4, met de stapmaat het dichtst bij 630 | stapmaat = praktijkregel (Wattjes h1 niet in het corpus) |
| aantrede op de klimlijn | horizontale afstand tussen opvolgende neuspunten, ontwikkeld langs de ingestelde looplijn (meting B); de looplijn (standaard 300 uit de zijkant) is de klimlijn voor de toets; tredevlak klimlijn = B + neus | besluit Jochem 30-10 (planner msg_93536085093b, msg_496305ebd730); Wattjes `__h2-par5-6` r. 68, `__h2-par7-11` r. 550 (aantreden uitgezet op de klimlijn) |
| informatie naast het oordeel | loodrecht op de voorkant gemeten (A: vanaf de volgende voorkant, A': vanaf de eigen voorkant) en de kleinste aantrede per klimlijn in de band 300 .. breedte-300 | een strikte lezing van 'loodrecht op de voorkant' geeft lagere waarden (referentietrap: 188,3 / 163,7 / 174,7) |
| verbouw | overschrijding van het nieuwbouwniveau = melding, met de ondergrens bestaande bouw (tabel 3.20) | Bbl art. 5.4 lid 1, 5.5 |
| disclaimer | constant in de kern (`DISCLAIMER`), pset `KBA_Status` | tekst akkoord Jochem 30-10 |
| getallen in het IFC | 15 significante cijfers (als ifcopenshell) | met 17 cijfers liep een boolean in web-ifc stuk op 1e-13 |
| nesten aan boomeinden | nestvoetafdruk 1 mm voorbij het eindvlak | geen coplanair eindvlak in de boolean; volume in de boom gelijk |

## Toetsregels

| id | eis | bron |
|---|---|---|
| bbl-4.26-breedte | >= 0,8 m (alle functies) | Bbl art. 4.26 lid 1, tabel 4.26 (`wet/bbl__afdeling-4-2` r. 800-845) |
| bbl-4.26-vrije-hoogte-* | >= 2,3 m woon / 2,1 m overig; tegen het trapgat als dat is opgegeven | idem; bijlage I 'vrije hoogte' |
| bbl-4.26-aantrede | >= 0,22 m woon / 0,185 m overig op de klimlijn (meting B) | idem + bijlage I 'klimlijn'; meetwijze Wattjes |
| bbl-4.26-optrede | <= 0,188 m woon / 0,21 m overig | idem |
| bbl-4.26-tredevlak | >= 0,05 m loodrecht op de voorkant | idem |
| bbl-4.26-tredevlak-klimlijn | >= 0,23 m op de klimlijn | idem |
| bbl-4.26-klimlijn-zijkant | >= 0,3 m | idem |
| bbl-4.26-lid2-hoogteverschil | <= 4 m (niet industrie/overig) | art. 4.26 lid 2, tabel 4.24 (r. 685-808) |
| bbl-4.26a, 4.27, 4.28 | markering, bordes, leuning: handmatig / n.v.t. | r. 847-870 |
| verbouw | melding i.p.v. afkeur, ondergrens tabel 3.20 | `wet/bbl__afdeling-5-2` r. 18-50, `wet/bbl__afdeling-3-2` r. 405-437 |
| advies | stapmaat 570-630 (praktijk), klimlijn 1/3-2/5 (`__h2-par5-6` r. 321), nest <= 1/4 boomdikte (`__h2-par3-4` r. 162), voorhout 15-30 (r. 167), achterhout, overgang verdreven treden (`__h2-par7-11` r. 851-857), stootbord niet in de hoek (r. 79-81), hoekverbinding (r. 141-145) | Wattjes is een leerboek: alleen advies |

## Golden tests

| test | uitkomst |
|---|---|
| 1 referentietrap vs Python v5 (`tests/golden_referentie.rs`) | treden, welstuk, stootborden, binnenboom, trappaal, Clearance: max dV 1e-6 mm3 in Rust; IFC via ifcopenshell (`tests/golden/vergelijk.py --anders buitenboom`): 38 onderdelen, max dV 4,7e-6 mm3, dbbox 1e-11 mm, 0 schemafouten; negatieve controle (neus 40) faalt zoals bedoeld |
| 1b buitenboom koud | gesloten profielen, overlap 0 mm2, aansluiting < 0,1 mm, borst geheel gedekt, lengte doorlopend deel = arm + dikte |
| 2 nabouw leverancierstrap (`tests/golden_leverancier.rs`) | gelijk: 18 optreden, 188, 18 treden, 18 stootborden, dikten 39/13/39, spil 115 x 68; verschillen vastgelegd (zie levering) |
| 3 toetsing (`toets::tests`) | per Bbl-regel net goed / net fout, woon en overig, nieuwbouw en verbouw |
| vormen (`tests/vormen.rs`) | recht, halfdraai, open trap, buitenwerks, trapgatcontrole, foutgevallen |
| viewer (web-ifc 0.0.77) | referentietrap en voorbeeld-halfdraai: alle elementen gesloten. Nabouw leverancierstrap: buitenboom deel 1 open (6 randen); web-ifc-booleans zijn numeriek fragiel (17 vs 15 cijfers maakte bij referentietrap het verschil; kleine bogen < 0,5 mm koorde nu als lijn). ifcopenshell rekent alle bestanden exact |

## Wat (nog) niet kan

- Verdrijfmethoden anders dan de fuik (harmonisch, cirkelboog, Winkler); scheluwe trap zonder hoek.
- Halfdraai met een te korte middenarm: geen automatische vakken (melding; vakken met de hand).
- Trappaal-insteek telt niet mee in clearance en klimlijntoets (melding).
- Leuning, wellatten, bolle stootborden, lip in de hoekverbinding: niet gemodelleerd.
- Houtdatabase/plankkeuze: de plankbreedte is invoer; de achterhout-controle (getekend: negatief = nest steekt onder de boom uit) meldt een te smalle plank.
- Weergave in web-ifc niet gegarandeerd bij elke geometrie (zie viewer); een getrianguleerde terugval voor de viewer is een optie voor fase 3.
