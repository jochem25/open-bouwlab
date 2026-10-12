# BENG Uniec-crosscheck — certified replay-ankers (lokaal)

Onafhankelijke BENG-cross-check tegen **certified Uniec 3.3.x**-berekeningen, als
tweede validatielaag naast de RVO-eindwaarden. Deterministische invoer + sub-totalen
per dienst (verwarming/tapwater/koeling/ventilatoren/PV) → diagnostisch sterker dan
alleen eindwaarden.

## Waar de data staat

De ankers komen uit **echte projecten** en staan daarom niet in deze (publieke) repo.
Ze staan lokaal, gitignored, onder:

```
tests/fixtures.local/beng_uniec_crosscheck/<case>/
    input.oes.json            # project{}-blok (open-energy-studio-formaat)
    expected.json             # certified waarden + provenance
    beng_geometry.input.json  # gevel-georienteerde BENG-geometrie
    project.uniec3            # bron-export (voor de round-trip-tests)
```

Cases: `aalten-2522` en `gouda-2467` (projectnummer-vorm). Ontbreekt de map (CI,
verse clone), dan **slaan de tests over** met een `SKIP`-regel op stderr; ze falen
niet. Tests die deze map lezen:

- `crates/openaec-project-shared/tests/beng_golden.rs` (Uniec-goldens, F6-brug, grens)
- `crates/openaec-project-shared/tests/beng_geometry_fixture.rs`
- `crates/openaec-project-shared/tests/beng_ifc_fixes_test.rs` (`a_ls_aalten_fixture_gewogen`)
- `crates/uniec3-import/tests/round_trip.rs`
- `crates/isso51-api/src/handlers/uniec_import.rs` (e2e-test)

De metingen en gap-analyses per case staan in de README's in die lokale map. De
norm-analyses in `docs/2026-07-1*-*.md` verwijzen nog naar het oude pad
`tests/verification/beng_uniec_crosscheck/<case>/`; lees daar `fixtures.local`.

`woning-2176/` (multi-rekenzone) volgt hetzelfde principe: het `.uniec3` staat alleen
lokaal (gitignored `*.uniec3`), de test vindt het via een glob en slaat anders over.

## Kanttekening

De geometrie in de `project{}`-blokken is een benadering van de originele
certificaten (residueel 1-5 %). Dit zijn **regressie-ankers, geen
certificerings-referentie**. Anti-fudge: `expected.json` en de toleranties worden
niet aangepast om een test groen te krijgen.
