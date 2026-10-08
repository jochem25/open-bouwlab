# Constructiemodule: UNP-profielen (2026-10-08)

Ontwerpbesluiten bij het toevoegen van de reeks UNP aan de staalmodule. Geen normtekst;
artikelverwijzingen staan in de code bij elke toets.

## Profieldata

- Reeks UNP 80 t/m UNP 400 (16 maten), `crates/constructie-core/data/staal_profielen.json`.
- Bron: ArcelorMittal, *Sections and Merchant Bars - sales programme* FR/EN/RU (PDF van
  2018-05-18), p. 98 (afmetingen) en p. 99 (statische waarden); afmetingen volgens DIN 1026-1.
  Het veld `bron` geldt voor elke waarde in het record.
- Velden: h, b, t_w, t_f, r (= r1), r2, A, I_y, W_el_y, W_pl_y, I_z, A_vz, G.
- Kruiscontrole met een tweede bron (STAD, *Technical data of sections UPN*, update
  01-09-2026): 208 waarden vergeleken, 197 gelijk. De verschillen zijn afrondingen (r op hele
  mm, G), UPN 80 W_pl,y en W_pl,z staan bij STAD als 0,01 (fout in die bron), en UPN 380
  W_pl,y is 1014 tegen 1010 cm3.

## Onafhankelijke controle

Script `bouw_s2_unp_controle.py` (werkmap constructiemodule, buiten de repo) rekent A, I_y,
W_el,y en W_pl,y na uit de afmetingen, met hellende flens en beide afrondingsstralen:

- flenshelling 8 % bij h <= 300 en 5 % bij h > 300 (volgens de fabrikant, niet overal 8 %);
- t_f geldt op u = b/2 vanaf de rug (h <= 300) of u = (b - t_w)/2 vanaf de binnenkant van
  het lijf (h > 300); de andere lezing voor h > 300 wijkt 1,6 - 2,3 % af, deze 0,1 %.

Uitkomst: A, I_y en W_el,y overal binnen 0,35 %. Afwijkingen > 1 %, gemeld en niet
gecorrigeerd: W_pl,y UNP 80 (-1,24 %) en UNP 350 (-2,04 %). De rekenkern gebruikt W_pl,y
niet (zie hieronder).

## Toetsing, verschillen met I/H-profielen

| Onderdeel | UNP |
|---|---|
| Flens c/t | c = b - t_w - r, gemeten vanaf het lijf (een uitstekende flens) |
| Afschuifoppervlak | formule voor gewalste U-profielen (EC3 6.2.6(3)b), zonder eta-ondergrens. Geeft 1,3 - 3,5 % minder dan A_vz in de fabrikantentabel: veilige kant |
| Buigweerstand | **elastisch, W_el,y**. De fabrikant vermeldt bij W_pl,y dat die alleen geldt als wringing is uitgesloten (twee profielen gekoppeld). Een enkel U-profiel wordt niet in het dwarskrachtcentrum belast |
| M-V-interactie | bij V_Ed > 0,5 V_pl,Rd conservatief (1 - rho) M_el,Rd; de I-formule (6.30) geldt niet voor U |
| Wringing en kip | niet getoetst. Vaste waarschuwing in UI en rapport: "U-profiel: wringing en kip niet getoetst - alleen bij zijdelingse steun, anders constructeur". Blokkeert niet |

## Golden S2

UNP-ligger S235, l = 4,20 m, belastingbreedte 2,40 m, CC2, woningvloer (g_k 0,75 kN/m2):
advies UNP 140 (UC buiging 0,94); UNP 120 voldoet niet (UC 1,33). Test
`crates/constructie-core/tests/norm_s2_unp.rs`, waarden uit het controlescript.

## Voorstel, niet gebouwd: 2 x UNP rug aan rug

Twee UNP's met de ruggen tegen elkaar (gekoppeld met bouten of vulplaatjes) vormen een
dubbelsymmetrische doorsnede. Wat dat vraagt:

1. Doorsnedewaarden: 2 x A, 2 x I_y, 2 x W_pl,y (dan wel plastisch, want geen wringing);
   I_z hangt af van de tussenruimte (steinerterm met e en de zwaartepuntsafstand y_s).
2. Een invoerveld voor de tussenruimte (0 mm of vulplaatdikte) en de koppelafstand.
3. Toets op de koppelmiddelen (afschuiving tussen de profielen) en de maximale
   koppelafstand; buiten scope v1 is dat een signalering naar de constructeur.
4. Kip wordt dan wel relevant als de bovenflens niet gesteund is (dezelfde scope als I/H).
5. Data: y_s uit de fabrikantentabel toevoegen (staat op p. 99).
