# Trapontwerper v1 in Open Bouwlab (O-2026-10-30-14)

Datum 2026-10-30. Branch `feat/trap-module` (vanaf `origin/main` 7dc552f). Bouwt voort op de
rekenkern `trap-core` (fase 2 en klimlijn-afronding, zie `2026-10-30-trap-core-rekenkern.md` en
`2026-10-30-klimlijn-afronding.md`).

## Bedrading (bestaand modulepatroon)

| laag | waar | patroon |
|---|---|---|
| rekenkern | `crates/trap-core` (`ontwerp_uitvoer`) | puur, geen I/O; tijd komt van de aanroeper |
| API | `crates/isso51-api/src/handlers/trap.rs`: `POST /trap/ontwerp`, `GET /trap/schema` | publieke reken-route als `/beng/calculate` (body-limit + per-IP rate-limit) |
| desktop | `src-tauri/src/commands.rs`: `trap_ontwerp` | zelfde contract, fout als string |
| client | `frontend/src/lib/trapClient.ts` | `isTauri()` -> invoke, anders fetch (als `bengClient`) |
| scherm | `frontend/src/components/trap/`, route `/trap` | als de constructiemodule: invoerkolom links, uitkomst rechts, debounce 400 ms |
| 3D | `TrapViewer3D` met `lib/bengShellLoader` (web-ifc 0.0.77) | pad van de viewer-spike; geen Fragments |

## Uitvoer

`trap_core::Uitvoer`: `rapport` (toetsing, metingen, waarschuwingen, minimaal trapgat), `tekening`
(treden, bomen, palen, binnen-/buitenzijde, looplijn, klimlijn op 300 uit de buitenzijde met dezelfde
hoekvorm, trapgat uit de invoer), `ifc`, `ifc_leverancier`, `bestandsnaam`.

| IFC | clearance | waarvoor |
|---|---|---|
| `ifc` | representatie 'Clearance' (Brep) op de IfcStair + IfcBuildingElementProxy 'Clearance' (Body, type CLEARANCE, pset KBA_Clearance) | coordinatie: Zoom toont de representatie, Revit leest alleen Body (proxy) |
| `ifc_leverancier` | geen clearance-geometrie; pset KBA_Clearance en KBA_Status blijven | leverancier |

Beide hebben de disclaimer letterlijk in pset `KBA_Status`. `IfcOpties.clearance` is
`Representatie` (standaard, CLI en golden tests ongewijzigd), `RepresentatieEnProxy` of `Geen`.

## Publieke repo

De referenties uit echte projecten (v5-geometrie en -IFC van de referentietrap, gemeten maten van een
leverancierstrap) staan in `tests/fixtures.local/trap/` (gitignored). De golden tests slaan zonder die
map over; in de repo staan alleen geanonimiseerde invoerparameters.

## Keuzes

| onderwerp | keuze | reden |
|---|---|---|
| toegang | publieke reken-route, geen entitlement | patroon BENG; trapberekening bevat geen projectdata. Entitlement (als constructie) is een besluit voor Jochem |
| klimlijn in de plattegrond | aparte lijn op 300 uit de buitenzijde, zelfde afronding als de looplijn | Bbl bijlage I; valt samen met de looplijn als die op 300 staat |
| clearance in 3D | alleen de proxy tekenen | anders dubbel rood (representatie + proxy) |
| gebruiksfunctie | woonfunctie / andere (Bbl-kolommen tabel 4.26) | opdracht; de kern kent alle functies |
| trapgat | optionele coordinatenlijst | de trap is leidend; het minimale trapgat volgt uit de clearance |

## Tests

| test | wat |
|---|---|
| `cargo test -p trap-core` | rekenkern, golden (met lokale fixtures), `ifc_valide`: recht/kwart/half x beide IFC's, ifcopenshell validate met express rules 0 fouten, proxyvolume > 0 |
| `cargo test -p isso51-api trap` | route: 200 met uitvoer, 422 bij onbekend veld en bij binnenhoek zonder paal |
| `frontend/src/components/trap/trap.test.tsx` | formulier (bouw, laden/opslaan rond), plattegrond, toetsing, pagina, sidebar, NL/EN-sleutels |
| `frontend/scripts/trap-rooktest.mjs` | via het scherm: recht, kwart, half; toetsing, klimlijn, 3D, beide downloads, opslaan/laden; screenshots |

## Niet in v1

Uitslag bomen en varianten (tabs in de mockup), trapgat uit een model lezen, PDF-tekeningset,
productie-IFC per onderdeel, andere verdrijfmethoden dan de fuik.
