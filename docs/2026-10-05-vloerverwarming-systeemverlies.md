# Vloerverwarming: fk = 0 en systeemverlies (ISSO 51:2023)

**Datum:** 2026-10-05 · **Code:** `crates/isso51-core/src/calc/{transmission,room_load,system_losses}.rs`
**Tests:** `crates/isso51-core/tests/norm_vloerverwarming_test.rs`
**Aanleiding:** controle van een warmteverliesrapport (projectnr. 3076, 02-10-2026), bevindingen 1 en 2.

Dit document legt de rekenregels en de gekozen norminterpretaties vast. Normtekst wordt
niet overgenomen; de verwijzingen gaan naar ISSO 51:2023 (met erratum 01-09-2023).

## Rekenregels

| Regel | Implementatie | Bron (ISSO 51:2023) |
|---|---|---|
| Het verwarmde deel van wand/vloer/plafond telt niet in de transmissie van het eigen vertrek | Element met `has_embedded_heating = true` draagt niet bij aan H_T (alle begrenzingstypen) | §2.5.1 (form. 2.6), §2.5.3 (fia,k), §2.5.4 (form. 2.22), §2.5.5 (fig,k); onderburen §2.8.1 |
| Afgifte naar de onderzijde is systeemverlies | Φverlies1 = fvlw · ΦHL,i, n = 1 bij vertrekberekening; idem Φverlies2 (fwvw), Φverlies3 | §2.9.1, form. 2.61 / 2.62, tabel 2.17 / 2.18 |
| Systeemverlies hoort niet in het vertrekvermogen | `total_heat_loss` = Φbasis + Φextra; `basis_heat_loss` zonder Φsys; Φsys apart in `system_losses` | §4.1 Opmerking, form. 4.23 |
| Systeemverlies telt wel voor verdeler/opwekker | `phi_hl_verdeler` = `phi_hl_build` + ΣΦsys | form. 4.24 (vertrek), 3.13 (gebouw) |
| Verdiepingsvloer binnen de woning | Φverlies1 = 0 | §2.9.1 |
| Negatief vertrekvermogen | Φsys ≥ 0 (berekend op max(ΦHL,i, 0)) | volgt uit 2.61 (fractie van een vermogen) |

Tabel 2.17 heeft twee kolommen; de code kiest per vertrek:

| Begrenzing van de verwarmde vloer | Kolom |
|---|---|
| grond, kruipruimte/onverwarmde ruimte, buitenlucht, water | begane grondvloer |
| aangrenzend gebouw (onderburen) | verdiepingsvloer |
| vertrek binnen de woning | geen systeemverlies |

## Norminterpretaties (akkoord Jochem, 05-10-2026)

1. **Kolom "begane grondvloer" ook boven buitenlucht en boven water.** De kop van §2.9.1 noemt
   bodem/kruipruimte/buiten; water is geen ISSO 51-categorie en volgt naar analogie de grond.
2. **Kruipruimte krijgt Φverlies1.** Een vloer boven een onverwarmde ruimte (`unheated_space`) valt
   onder "bodem/kruipruimte" van §2.9.1. Tot 05-10 was dit 0.
3. **ΦHL,i in formule 2.61 is het vertrekvermogen van formule 4.23, zonder Φadd.** Er is dus geen
   kringverwijzing; de eerdere oplossing ΦHL,i = (Φbasis + Φextra)/(1 − f) gaf effectief
   f/(1 − f) = 0,111 bij f = 0,10 en telde Φsys in het vertrekvermogen.
4. **De vlag geldt per element.** De norm spreekt van "het verwarmde deel"; een deels verwarmde vloer
   of wand moet als twee elementen worden ingevoerd.
5. **Rc voor tabel 2.17/2.18 wordt geschat uit U** (Rc = 1/U − Rsi − Rse). Ongewijzigd; de
   tabelvoetnoot gaat uit van Rc inclusief een afwerking van 0,15 m²K/W.

## Bekende randgevallen (review 05-10)

- **Wand- of plafondverwarming naar een onverwarmde ruimte of een vertrek binnen de woning** telt
  0 W: fk / fia,k = 0 voor het verwarmde deel (§2.5.3, §2.5.4) en Φverlies2/3 = 0 voor wanden op
  binnenwanden resp. plafonds op tussenvloeren (§2.9.1). Normconform, maar let op bij invoer.
- **Gemengde verwarmde vloeren in één vertrek** (bv. deels op grond, deels boven onderburen):
  één fvlw per vertrek met de kleinste Rc en de kolom begane grondvloer zodra één vloer daaronder
  valt. Conservatief; per-vloer weging is niet geïmplementeerd (open punt in `TODO.md`).

## Gevolgen

- Vertrekken met vloerverwarming naar grond/kruipruimte/water krijgen een lager vertrekvermogen
  (projectnr. 3076, lokaal herberekend: −12 tot −31 % in de verdiepingsvertrekken met vloerverwarming
  naar water); het gebouwtotaal ΦHL,build daalt met het weggevallen vloerverlies.
- In UI en rapport staat Φ_sys als kolom naast Φ_totaal en wordt hij op de vertrekbalk gestapeld;
  sinds deze wijziging telt hij niet meer in Φ_totaal. Open punt in `TODO.md`.
