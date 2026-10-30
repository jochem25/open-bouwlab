# Klimlijn met afgeronde hoek (O-2026-10-30-07, 2026-10-30)

Vraag: mag de looplijn in de bocht de lijn 300 uit de buitenzijde blijven, met alleen de hoek
afgerond (straal rho)? Bbl bijlage I: klimlijn = 'denkbeeldige, vloeiend verlopende lijn';
tabel 4.26: minimaal 0,3 m van de zijkanten.

## Kern

| onderdeel | gedrag |
|---|---|
| `looplijn.straal` (invoer) | = rho, afrondingsstraal van de hoek van de offsetlijn; de boog raakt beide armen |
| weg | boog uit de binnenhoek: rho = breedte - afstand (standaard, ongewijzigd) |
| 0 | scherpe knik (niet vloeiend) |
| negatief of te groot | invoerfout (raakpunt buiten de arm) |
| `Contour::afronding` | gebruikte rho per binnenhoek |
| `Polylijn::grootste_knik` | grootste richtingsverandering (graden); boog 0,25 per koorde |
| toets-JSON | `meting.looplijn_afronding`, `meting.looplijn_grootste_knik`; toelichting `bbl-4.26-aantrede` noemt rho en 'vloeiend' / 'NIET vloeiend' (geen afkeur) |
| `verdrijving::meet_op_lijn` | aantreden B per trede op een willekeurige contour bij vaste voorkanten |

Waarom >= 300 van beide zijkanten: de afronding ligt binnen de offsetlijn (verder van de
buitenzijde); in een kwart (breedte b, afstand a) ligt het midden van de boog
(b - a) x wortel2 - rho x (wortel2 - 1) van de binnenhoek, en dat is >= b - a voor rho <= b - a.
Gemeten in de test voor alle rho.

## Meting (test `tests/klimlijn_afronding.rs`)

Leverancierstrap (vaste neusranden uit het coordinatie-IFC, fixture tegen het IFC gecontroleerd
op 0,05 mm): B op de lijn 300 uit de buitenzijde.

| rho | 0 (info) | 25-300 | 572 (standaard) |
|---|---|---|---|
| min B | 179,8 (trede 10) | 179,8 (trede 10) | 162,8 (trede 11) |

De maatgevende winders (10: 179,8, 14: 183,2) kruisen de lijn op de rechte armen, waar de afronding
niets verandert: geen rho haalt 185 (tabel 4.26 overig). Kern-nabouw (herverdeeld op de lijn):
B = 199,6 (rho 0) tot 185,2 (rho 572); tredevlak 230 vanaf rho <= 379,9. referentietrap (17 x 187,06): B 201,7
tot 188,3; 220 (woon) bij geen rho.
