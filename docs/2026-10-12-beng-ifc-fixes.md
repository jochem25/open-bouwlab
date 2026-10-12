# BENG-keten: vijf fixes voor de IFC-route

Datum: 2026-10-12. Branch `feat/beng-ifc`. Aanleiding: inventarisatie
IFC naar BENG (ifc-ruimtebalans, fase B0, sectie 5). Normbron: het normcorpus
(NTA 8800:2025+C1:2026, Bbl toestand 2026-09-24). Er staat geen normtekst in
deze repo; alleen artikel-, tabel- en formulenummers.

## Overzicht

| # | fix | norm | waar |
|---|---|---|---|
| 1 | A_ls gewogen met f_ls | NTA 8800 §6.7.2 formule (6.3), §6.7.3 | `BengAdjacency::f_ls`, `BengBoundary::f_ls`, `BengGeometry::loss_surface_area_m2`, `beng/mod.rs` |
| 2 | kozijnfractie per kozijnmerk | NTA 8800 §7.6.6.2 (formule 7.44; forfait 0,25) | `WindowDef::frame_fraction`, brug |
| 3 | belemmering: overstek, volledig, overig | NTA 8800 §17.3.2 c/e/g, §17.3.5, §17.3.7, tabellen 17.8, 17.9, 17.13, 17.14 | `Obstruction`, `nta8800-demand/src/calc/shading.rs`, Uniec-import |
| 4 | utiliteit in de Uniec-import + grenswaarden | Bbl tabel 4.148A, art. 4.149 lid 1-2 | `nta8800-ep/src/beng.rs`, `BengZone::gebruiksfuncties`, `uniec3-import` |
| 5 | meetconventie A_T | NTA 8800 K.1.2, K.1.4, §6.9 | `docs/2026-10-12-beng-meetconventie-a-t.md` |

## 1. f_ls

| begrenzing | f_ls | §6.7.3 |
|---|---|---|
| aangrenzende verwarmde ruimte, andere rekenzone, perceelsgrens | 0 | a |
| buitenlucht, water, AOR, AOS, sterk geventileerd, onverwarmde kelder (AOR) | 1 | b |
| grond, kruipruimte, kelderwand | 0,7 | c |

De BENG-geometrie berekent A_ls zelf: de brug naar `SharedGeometry` verliest
het onderscheid kruipruimte/AOR. Op de ruimteroute (`SharedGeometry`) weegt
`Ground` 0,7 en telt `UnheatedSpace` als AOR (1); daar is geen kruipruimte te
herkennen.

## 2. Kozijnfractie

`WindowDef.frame_fraction` (0..1, optioneel). Afwezig: forfait 0,25. De brug
zet hem op de opening. De zontoetreding schaalt met (1 − F_fr) (formule 7.32).

## 3. Belemmering

`Obstruction` heeft nu `none`, `minimal`, `overhang{h_o}`, `full` en
`other{overhang_h_o?}`.

| variant | warmte | koeling |
|---|---|---|
| overhang (verticaal vlak) | tabel 17.8 | tabel 17.9 |
| full | tabel 17.13 | tabel 17.14 |
| other | tabel 17.13 | tabel 17.9 bij een overstek, anders 1,0 |
| overhang op een niet-verticaal vlak | als other | als other |

Helling bij 17.13/17.14: dichtstbijzijnde kolom, bij gelijke afstand de
grootste factor. Helling onder 15°: zichtveld zuid (§17.3.1).

Uniec-codes: `BELEMTYPE_VOLLEDIG` → full; `BELEMTYPE_CO_EN_ZIJ` → overhang met
h_o = hoogte / horizontale afstand uit de `BELEMMERING`-entiteit, met
zijbelemmering → other; `BELEMTYPE_OVERIG` → other. Zijbelemmering (d,
`BELEMTYPE_ZIJ*`) en belemmering met constante hoogte (b) zijn niet
gemodelleerd; ZIJ blijft benaderd als minimal (met waarschuwing).

De Gouda-fixture had overstek en volledig als minimal; die is gecorrigeerd
naar de bron. Merk I stond in de oude notitie als VOLLEDIG; de bron zegt
overstek.

## 4b. Woongebouw en lichte bouwwijze

- Woningtypen portiek, galerij en gestapeld krijgen zonder gebruiksfuncties de
  grens van Bbl tabel 4.148A rij 1a (woongebouw) in plaats van 1e.
- Lichte-bouwwijze-toeslag (art. 4.149 lid 4) op de BENG1-grens als de
  gewogen D_m (tabel 7.10, uit de bouwwijze-codes) ≤ 180 is. Alleen voor de
  rijen waar lid 4 geldt (1a, 1e, 7b); niet bij onbekende bouwwijze.

## 4. Utiliteit

- `TGEB_UTILIT` wordt ingelezen. A_g per rekenzone is de som van de
  gebruiksfuncties (`UNIT-RZ-GFAG`) als `UNIT-RZAG` ontbreekt.
- Gebruiksfuncties per rekenzone in `BengZone.gebruiksfuncties`. Bekend uit een
  echte export: `GF_BIJEENOVER`. Andere codes worden uit de naam afgeleid, met
  een waarschuwing.
- Grenswaarden uit Bbl tabel 4.148A voor alle functies, gewogen naar
  gebruiksoppervlak bij meer functies (art. 4.149 lid 2). Industrie en overige
  gebruiksfunctie kennen geen eis.
- `vloer boven buitenlucht` (`VLAK_VLOER_BOVBUI`, `LIBVLAK_VLOER_BOVBUI`) viel op
  de grond-terugval en wordt nu buitenlucht, horizontaal.

Getoetst op een lokaal utiliteitsproject (niet in de repo): A_g, A_ls en de
drie grenswaarden zijn gelijk aan de gecertificeerde Uniec-uitvoer, voor drie
exportversies. Het BENG1-indicatorgetal wijkt nog fors af (rekenkern
utiliteit, niet gekalibreerd).

## Effect op de ankers

Gemeten met `uniec_measure_bridged` en `gouda_measure_bridged`
(`cargo test -p openaec-project-shared --test beng_golden -- --ignored --nocapture`),
voor (main 806513a) en na (deze branch):

| anker | grootheid | voor | na | certified |
|---|---|---|---|---|
| Aalten | BENG1 | 97,58 (−5,9 %) | 97,58 (−5,9 %) | 103,69 |
| Aalten | A_ls | 245,7 | 225,6 | 232,3 (terugrekening) |
| Aalten | BENG1-grens | 133,39 | 123,39 | 123,39 |
| Gouda | BENG1 | 82,98 (−13,4 %) | 81,69 (−14,8 %) | 95,86 |
| Gouda | BENG1-grens | 99,37 | 96,40 | 96,40 |

- De Aalten-afwijking op BENG1 verandert niet: f_ls raakt alleen de grens, de
  Aalten-fixture heeft geen overstek of volledige belemmering.
- De grens is nu op beide ankers gelijk aan certified. Daarvoor waren twee
  dingen nodig: f_ls 0,7 en de lichte-bouwwijze-toeslag (Bbl art. 4.149 lid 4,
  +5 bij een gewogen D_m ≤ 180 kJ/(m²·K); beide ankers D_m = 180). De toeslag
  werd nergens toegepast. Gevonden in de review: beide ankers weken na de
  f_ls-fix exact 5,00 af.
- Gouda BENG1 zakt 1,3 procentpunt; alleen de belemmering in de fixture is
  veranderd (nu gelijk aan de bron). De afwijking zelf zit in
  de rekenkern (bekend, zie TODO).
- Pass/fail van de ignored ankers is ongewijzigd (4 groen, 9 rood, voor en na).

## Open

- Zijbelemmering (§17.3.2 d, tabel 17.10/17.11) en belemmering met constante
  hoogte (b).
- Gebruiksfunctiecodes buiten `GF_BIJEENOVER` bevestigen met echte exports.
