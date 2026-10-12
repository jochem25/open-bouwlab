# Contract beng-import v1

Afgesproken 2026-10-12 tussen open-bouwlab en ifc-ruimtebalans (optie B uit
de B0-inventarisatie). Ontvanger: `crates/openaec-project-shared/src/beng_import.rs`.
Endpoint: `POST /api/v1/import/beng`. Schema: `schemas/v1/beng-import.schema.json`
(gegenereerd uit de Rust-types; de test `schema_bestand_is_actueel` bewaakt
drift).

## Document

| veld | type | verplicht | betekenis |
|---|---|---|---|
| `format` | `"beng-import"` | ja | vast |
| `version` | `"1.0"` | ja | contractversie |
| `source` | string | ja | bron, bv. tool en versie |
| `exported_at` | string | ja | ISO 8601 |
| `project_name` | string | nee | informatief |
| `true_north_deg` | number | nee | informatief |
| `compleet` | bool | ja | `true` alleen als `beng_geometry` zonder aanvulling deserialiseert en valideert |
| `beng_geometry` | BengGeometry | ja | de kern, exact de serde-vorm van `beng_geometry.rs` |
| `herkomst`, `zones_extra`, `gevels_extra`, `opaque_defs_extra`, `window_defs_extra`, `aansluitingen`, `belemmering_geometrie`, `vloeren_per_bouwlaag`, `pv_vlakken` | array van objecten | nee | v1 leest ze niet; teruggemeld als `genegeerd` |
| `meldingen` | array van objecten | nee | doorgegeven aan de gebruiker (`code: tekst`) |

Onbekende sleutels op het hoogste niveau: 422.

## Kern

Veldnamen en enums: zie `beng_geometry.rs`. Nieuw op 12-10-2026:
`WindowDef.frame_fraction`, `BengZone.gebruiksfuncties`
(`{functie, a_g_m2}`, functie volgens Bbl tabel 4.148A) en de uitgebreide
`belemmering` (`none`, `minimal`, `full`, `{overhang: {h_o}}`,
`{other: {overhang_h_o?}}`).

`bruto_buiten_opp_m2` is A_T volgens NTA 8800 K.1.2/K.1.4, zie
`docs/2026-10-12-beng-meetconventie-a-t.md`.

## Onvolledig

Rc/U en U_w komen niet uit IFC. Bij `compleet = false` mogen
`opaque_defs[].thermal` en `window_defs[].u_w_per_m2k` ontbreken (weglaten,
geen null en geen placeholder). De import vult niets in en geeft:

```json
{ "status": "onvolledig", "ontbrekend": ["/beng_geometry/opaque_defs/2/thermal"], "meldingen": [], "genegeerd": [], "source": "..." }
```

De rest van de structuur wordt wel gecontroleerd. `compleet = true` met
ontbrekende waarden is een 422.

## Antwoorden

| status | wanneer | body |
|---|---|---|
| 200 | compleet | `status: "compleet"`, `beng_geometry`, `meldingen`, `genegeerd` |
| 200 | onvolledig | `status: "onvolledig"`, `ontbrekend` |
| 400 | geen JSON | `error: "json_parse_error"` |
| 422 | contractfout | `error: "beng_import_error"`, `detail` met JSON-pad |

Een raam zonder belemmering rekent als "geen belemmering" (factor 1,0); de
import meldt het aantal zodat de gebruiker kiest.

## Versies

Minor-versies voegen alleen optionele velden toe. Een nieuwe sha van het
schema wordt aan ifc-ruimtebalans gemeld; zij pinnen het bestand.
