# Trapontwerper v1.1 - looplijn, kier, verdrijving, toegang (2026-11-12)

Opdracht O-2026-11-12-01. Vervolg op `docs/2026-10-30-trapontwerper-v1.md`.

## 1. Looplijn en klimlijn: gemeten vanaf de buitenkant van de boom

Besluit Jochem 11-11-2026 (gelijk aan de leverancier): de looplijn ligt 300 mm vanaf de
BUITENKANT VAN DE BOOM, niet vanaf de rand van de loopbreedte.

| begrip | meetpunt |
|---|---|
| invoer `looplijn.afstand_buitenkant_boom` (standaard 300) | buitenkant van de buitenboom (het vlak aan de muurzijde) |
| rekenkern intern (`Contour`) | rand loopbreedte; afstand = invoer - boomdikte (300 - 40 = 260 bij de standaardboom) |
| klimlijn in de tekening (`KLIMLIJN_AFSTAND` 300) | buitenkant van de buitenboom |
| toets `bbl-4.26-klimlijn-zijkant` | afstand tot de buitenkant van de boom, aan beide zijden (rand loopbreedte + boomdikte) |
| informatieve klimlijnband | 300 .. buitenwerkse breedte - 300, vanaf de buitenkant van de buitenboom |
| Wattjes `wattjes-klimlijn-plaats` (1/3 - 2/5) | afstand vanaf de buitenkant van de buitenboom / binnenwerkse breedte |
| IFC `Pset_StairCommon.WalkingLineOffset` | ongewijzigd: afstand tot de binnenzijde van de loopbreedte |

Norminterpretatie: het Bbl definieert de klimlijn (bijlage I) als vloeiende lijn over de
voorkanten; tabel 4.26 eist een afstand tot "de zijkant". Dat de zijkant hier de buitenkant van
de boom is, is het besluit van 11-11, niet een letterlijke normtekst.

De oude invoersleutel `afstand_buitenzijde` bestaat niet meer: een v1-bestand met die sleutel
geeft een duidelijke fout (422) in plaats van stil 40 mm te verschuiven.

## 2. Treden tot het stootbord (geen kier)

Conventie (ongewijzigd, `onderdelen.rs`): stootbord j staat OP trede j en loopt tot de onderkant
van trede j+1; trede j loopt door tot het ACHTERvlak van stootbord j, ook in verdreven treden.
Gemeten in `tests/kier.rs` voor recht, kwart, half en de onderkwart-contour: per stootbord
0 mm2 niet gedragen, 0 mm kier, 0 mm spleet in z; positieve controle (stootbord tegen de
verkeerde trede) vindt wel een kier. trap-core had de kier van de eerdere Python-nabouw niet.

## 3. Automatische verdrijfvakken

De automatische keuze nam het vak met de grootste kleinste aantrede aan de binnenboom; dat gaf
lange vakken (bij een kwart vrijwel de hele arm verdreven) met een zichtbare overgang (Wattjes
deel VIII h2 par. 7-11 r. 851-856: eerste verdreven aantrede groter dan het reeksverschil).
Nieuw: eerst opties zonder die overgang, daarbinnen de grootste kleinste aantrede; alleen als er
geen is de beste met overgang. Effect op golden 3 (kwart met onderkwart): 6 verdreven voorkanten
(leverancier 5) in plaats van 13. De v5-referentie behoudt zijn eigen vakken uit de invoer.

## 4. Standaardwaarden (besluit Jochem 11/12-11-2026)

| veld | was | nu |
|---|---|---|
| `boom.plankbreedte` | 320 | 320 |
| `trede.dikte` | 40 | 38 |
| `boom.voorhout` | 50 (Wattjes-afwijking) | 30 (Wattjes 15-30) |
| `looplijn` | 300 uit rand loopbreedte | 300 uit buitenkant boom |

De standaardtrap van het scherm (kwartdraai 2400 + 1600, loopbreedte 850, 2800) gaat zonder
Bbl-afkeur en zonder Wattjes-afwijking door de toets (`tests/kier.rs`,
`standaardtrap_zonder_wattjes_afwijking`).

## 5. Toegang: entitlement `trap`

Patroon constructie: `/trap/ontwerp` en `/trap/schema` vereisen login (401) en de
Authentik-groep `TRAP_ENTITLEMENT_GROUP` (default `openbouwlab-trap`, 403 zonder);
`TRAP_ENABLED=false` zet de module voor iedereen uit. `/me` meldt `trap` in `entitlements`.
Frontend: sidebargroep Trap en route `/trap` alleen met het recht; zonder recht een melding.
Desktop (Tauri): zoals constructie in deel 1 uit; het Tauri-command blijft bestaan.
Lokaal testen: `scripts/trap-lokaal.ps1` start een proxy op localhost
(`scripts/lokaal-auth-proxy.mjs`) die een lokale gebruiker met de groep meegeeft.

## 6. Golden 3: kwartdraai met onderkwart (leverancierstekening)

Invoer geanonimiseerd in `crates/trap-core/tests/fixtures/invoer_kwartdraai_onderkwart.json`
(vrije contour, loopbreedte per arm verschillend); maten van de leverancier lokaal in
`tests/fixtures.local/trap/onderkwart_leverancier_maten.json`. Gelijk: 16 optreden van 198,75,
aantrede op de looplijn 194,7 (leverancier 194,5), rechte treden binnen 2 mm. Verschillen in de
verdrijving worden vastgelegd in `target/trap-core/onderkwart_vergelijking.json`.
Het scherm kan zo'n contour laden ('invoer laden', vorm "Vrije contour (geladen)").
