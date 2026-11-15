# Trapontwerper headless - assemblagecontract (2026-11-15)

Opdracht O-2026-11-15-05 (S1). De bestaande rekenkern `trap-core` levert een trap als
onderdeel-IFC volgens het assemblagecontract `kba-assemblage/1`
(`%KBA_SHARED%\conventies\assemblagecontract\contract.md`). Het scherm `/trap` is
ongewijzigd (bevroren); zonder contract is de IFC-uitvoer byte-gelijk aan fe8e6b6.

## 1. Snel

```text
cargo build --release -p trap-core
trap contract <invoer.json> [--uit <map>] [--tijd <ISO 8601>] [--corpus <kennis-v2>]
trap contract-schema          # JSON-schema van de invoer
```

Voorbeeld (map `crates/trap-core/tests/fixtures/contract/`):

```text
trap contract trap-01.json --uit C:\temp\trap
```

Uitvoer, met het onderdeel-id als naam:

| bestand | inhoud |
|---|---|
| `trap-01.ifc` | contract-IFC (IFC4, mm): koppelen in Revit (Link IFC, origin to origin) |
| `trap-01.toets.json` | toetsing zoals de API hem geeft (regels, resultaat, minimaal trapgat) |
| `trap-01.toetsverslag.md` | Bbl-toets met artikelnummers, `artikel_status` uit het normcorpus en 'gerekend met' (de effectieve invoer) |
| `trap-01.pdf` | geannoteerde plattegrond A3: treden, bomen, paal, looplijn, klimlijn, trapgat/sparing, maten, oordeel per regel |

Exit 0 = gelukt; anders een melding op stderr en exit 1. De kern weigert in plaats van te
raden (zie 4).

## 2. Invoer (schema `kba-trap-headless/1`)

Schema: `crates/trap-core/schema/trap-headless.schema.json` (kopie naast de contractschema's).

```json
{
  "schema": "kba-trap-headless/1",
  "onderdeel": "trap-01",
  "naam": "trap 01",
  "project": "project_test.json",
  "bouwlaag": "00",
  "doorbreekt": "01",
  "houtsoort": "vuren",
  "trap": { "plattegrond": { "soort": "armen", "start": [4000, 2000], "...": "..." } }
}
```

| veld | betekenis |
|---|---|
| `onderdeel` | id van het onderdeel (`^[a-z0-9][a-z0-9-]*$`), deel van elke GUID. Nooit wijzigen of hergebruiken |
| `naam` | IfcStair.Name; mag wijzigen zonder GUID-wijziging. Weg = onderdeel-id |
| `project` | pad naar `project.json` (relatief aan de invoer) of het object zelf; schema `projectoorsprong.schema.json` |
| `bouwlaag` | bouwlaagcode waarop de trap staat; geeft `niveaus.onder` (peil) |
| `doorbreekt` | bouwlaagcode van de doorbroken vloer; weg = de eerstvolgende erboven. Geeft `niveaus.boven` (peil) en `vloerdikte_boven` (vloerpakket) |
| `houtsoort` | vuren, grenen, douglas, lariks, eiken, beuken, essen, esdoorn, meranti, merbau, accoya, hardhout, zachthout, multiplex, mdf. Wordt NAA.K.T. `hout_<soort>_generiek` op trap en elk deel |
| `nlsfb` | optioneel `{code, omschrijving}`; weg = 24.11 (recht) of 24.12 (niet-recht), uit de trapvorm |
| `trap` | de gewone trapinvoer (`trap-invoer.schema.json`): plattegrond in PROJECTCOORDINATEN, trapgat, looplijn, verdrijving, boom, trede, stootbord, trappaal, gebruiksfunctie, bouwsoort, ... ZONDER `naam`, `project`, `niveaus` en materiaalvelden |

Verdiepingshoogte = peil(doorbreekt) - peil(bouwlaag); die staat niet in de invoer, zodat
er geen tweede bron is voor een maat die al in het project staat.

## 3. Wat het contract-IFC bevat

| regel (contract.md) | in het IFC |
|---|---|
| O1-O3 | IfcProject/Site/Building/BuildingStorey met naam en GUID uit project.json; storey op peil |
| O4 | IfcMapConversion naar EPSG:28992 / NAP (Eastings in meter, Scale 0.001) |
| G | GlobalId = uuid5 over `<project>|<onderdeel>|<element>`; zelfde sleutels als de Python-assembler, dus revisies houden hun GUID's (Revit herlaadt zonder dubbelingen) |
| K1-K5 | IfcStair met PredefinedType; NL-SfB tabel 1; een IfcMaterial NAA.K.T. op trap, steek en elk deel; Pset_StairCommon/Pset_StairFlightCommon |
| K6 | pset KBA_Assemblage (contractversie, project, onderdeel, bouwlaag, invoerbestand + sha256) |
| S | IfcBuildingElementProxy SPARINGSVOLUME = het trapgat (uit de invoer, anders het minimale trapgat uit de clearance), van o.k. vloerpakket tot peil van de doorbroken laag; pset KBA_Sparing |
| staptitel | IfcStairFlight heet `steek` (zonder trapnaam: stabiele sleutel) |

De clearance staat als representatie 'Clearance' op de IfcStair (geen proxy).

## 4. Weigeringen (exit 1, melding)

- `schema` is niet `kba-trap-headless/1` (bv. project.json als invoer opgegeven);
- `trap.naam`, `trap.project`, `trap.niveaus`, `trap.boom.materiaal` of `trap.trappaal.materiaal` in de invoer;
- onbekende bouwlaag, doorbroken laag niet boven de bouwlaag, of zonder `vloerpakket`;
- ongeldig onderdeel-id, NL-SfB-code geen trapcode (24.10-24.15, 24.30/24.31), onbekende houtsoort;
- project.json in strijd met zijn schema (contractversie, id, laagcodes, crs/NAP, dubbele code);
- GUID-botsing (twee objecten met dezelfde sleutel).

## 5. Toetsverslag en normcorpus

Het verslag zoekt per Bbl-verwijzing in de toetsregel (`Bbl art. 4.26 lid 1`, ...) de
bronregel in `kennis-v2\wet\<bron_id>.md` en neemt `artikel_status` en `bron_uri` over, met
de toestand van het corpus (`geldig_van`). Het Bbl wijzigt 1 januari en 1 juli; ligt de
toestand voor het laatste wijzigingsmoment, dan staat er "MOGELIJK VEROUDERD". Zonder corpus
(`--corpus` of `%KBA_SHARED%`) staat er "niet geverifieerd" - nooit een aangenomen status.

## 6. Claude als bediening (MCP)

De MCP-server in `mcp-server/` (`npm install`, daarna `npx tsx src/index.ts`) heeft twee tools:

| tool | doet |
|---|---|
| `trap_contract` | `invoerPath`, optioneel `uitMap`, `tijd`, `corpus`: draait `trap contract` en geeft de paden plus een samenvatting (optrede/aantrede, Bbl voldoet_niet, handmatig) |
| `trap_contract_schema` | het invoerschema |

Binary: `TRAP_BIN`, anders `target/release/trap(.exe)`. De tool doet niets wat de CLI niet
doet; een weigering komt terug als `isError` met de melding van de CLI.

## 7. Toets (klaar-als) en golden tests

- `tests/contract.rs`: golden Voorstraat 172 (geanonimiseerd, `fixtures/contract/referentie_*.json`):
  zelfde trapmodel en toetsing als via de gewone invoer; lokaal ook tegen de Python-referentie v5.
  Verder: gouden GUID-vectoren gelijk aan de Python-referentie, revisie = zelfde GUID's,
  trapnaam wijzigt geen GUID, sparing = trapgat, weigeringen.
- Contractcontrole en IDS: `controleer.py`, `IfcStair.ids`, `IDS_NAAKT-v2.5.ids` (zie de levering).
- Revit-koppeling: meetverslag in
  `%KBA_SHARED%\uitvoer\assemblage\O-2026-11-15-05-S1-trap-headless\LEVERING.md`.
