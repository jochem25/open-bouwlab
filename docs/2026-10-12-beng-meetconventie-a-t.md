# Meetconventie A_T in de BENG-geometrie

Datum: 2026-10-12. Geldt voor `BengBoundary::bruto_buiten_opp_m2`
(`crates/openaec-project-shared/src/beng_geometry.rs`) en het contract
beng-import v1.

## Besluit

`bruto_buiten_opp_m2` is de geprojecteerde oppervlakte **A_T** volgens NTA 8800
bijlage K.1.2, met de afsnijvlakken van K.1.4 (via §6.9.1 en §6.9.2), bruto:
inclusief ramen en deuren.

De veldnaam zegt "buiten", maar de norm meet niet buitenwerks. De naam blijft
staan omdat het blok 1-op-1 op Uniec en het beng-import-contract aansluit.

## Wat K.1.4 vastlegt

Bron: NTA 8800:2025+C1:2026, bijlage K.1.2 en tabel K.1 (referentiedetails).
Samengevat, niet geciteerd:

| plek | afsnijvlak |
|---|---|
| gevel bij de begane-grondvloer | vloerpeil (bovenzijde vloer) |
| begane-grondvloer bij de gevel | binnenzijde gevel |
| gevel bij een verdiepingsvloer | geen afsnijvlak: de gevel loopt door |
| gevel bij een binnenwand | geen afsnijvlak: de gevel loopt door |
| buitenhoek | binnenoppervlak van de aansluitende gevel |
| dak bij de gevel | snijpunt onderzijde dak met het doorgetrokken binnenoppervlak van de gevel |
| nok | snijpunt van de binnenoppervlakken van de dakvlakken |
| vloer op grond of boven een kruipruimte | binnenwerks, voor zover grenzend aan een buitenwand of AOR-wand (§6.9.3) |
| raam, deur, kozijn met paneel | het vlak binnen de kozijnranden (K.1.2) |

## Gevolgen

- Per vertrek gemeten binnenvlakken (ISSO 51) missen per bouwlaag de strook
  van de gevel ter hoogte van de verdiepingsvloer. Voor BENG moet die strook
  erbij.
- Een buitenmaat is te groot: die telt de gevel- en dakdikte mee.
- `A_ls` (§6.7.2) en `A_r` (§6.8) gebruiken dezelfde A_T.

## Afgestemd met

ifc-ruimtebalans (contract beng-import v1, 12-10-2026): zij leveren A_T op deze
manier.
