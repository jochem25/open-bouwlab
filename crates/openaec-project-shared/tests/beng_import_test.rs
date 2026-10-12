//! Contract beng-import v1 (12-10-2026): ontvangst, schema en e2e tot en met
//! `compute_beng`. Alleen synthetische invoer (`tests/fixtures/beng-import-v1-synthetisch.json`).

use openaec_project_shared::beng_import::{
    beng_import_schema_json, import_beng, BengImportError, BengImportStatus,
};
use openaec_project_shared::energy::{
    EnergyInput, HeatEmissionType, HeatGeneratorType, HeatingInput, VentilationInput,
    VentilationSystemType,
};
use openaec_project_shared::shared::{BuildingTypeShared, ResidentialType};
use openaec_project_shared::{compute_beng, Obstruction, ProjectV2};
use serde_json::Value;

const FIXTURE: &str = "../../tests/fixtures/beng-import-v1-synthetisch.json";
const SCHEMA: &str = "../../schemas/v1/beng-import.schema.json";

fn fixture() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap()
}

#[test]
fn compleet_document_levert_geometrie_en_meldingen() {
    let r = import_beng(&fixture()).expect("fixture importeert");
    assert_eq!(r.status, BengImportStatus::Compleet);
    assert!(r.ontbrekend.is_empty());
    assert_eq!(
        r.genegeerd,
        vec!["herkomst".to_string(), "aansluitingen".to_string()]
    );
    assert!(r
        .meldingen
        .iter()
        .any(|m| m == "N-TEST: synthetische fixture"));
    // Eén raamplaatsing (noord, wd-raam) zonder belemmering → melding.
    assert!(r
        .meldingen
        .iter()
        .any(|m| m.starts_with("1 raamplaatsing(en) zonder belemmering")));

    let geo = r.beng_geometry.expect("geometrie bij compleet");
    let zuid = &geo.zones[0].gevels[1].ramen;
    assert_eq!(zuid[0].belemmering, Obstruction::Overhang { h_o: 0.7 });
    assert_eq!(zuid[1].belemmering, Obstruction::Full);
    assert_eq!(geo.window_defs[0].frame_fraction, Some(0.18));
}

#[test]
fn onvolledig_document_geeft_paden_en_geen_geometrie() {
    let mut doc = fixture();
    doc["compleet"] = Value::Bool(false);
    doc["beng_geometry"]["opaque_defs"][2]
        .as_object_mut()
        .unwrap()
        .remove("thermal");
    doc["beng_geometry"]["window_defs"][1]
        .as_object_mut()
        .unwrap()
        .remove("u_w_per_m2k");
    let r = import_beng(&doc).expect("onvolledig is geen fout");
    assert_eq!(r.status, BengImportStatus::Onvolledig);
    assert!(
        r.beng_geometry.is_none(),
        "niets invullen, niets teruggeven"
    );
    assert_eq!(
        r.ontbrekend,
        vec![
            "/beng_geometry/opaque_defs/2/thermal".to_string(),
            "/beng_geometry/window_defs/1/u_w_per_m2k".to_string()
        ]
    );
}

#[test]
fn onvolledig_maar_compleet_true_is_strijdig() {
    let mut doc = fixture();
    doc["beng_geometry"]["opaque_defs"][0]
        .as_object_mut()
        .unwrap()
        .remove("thermal");
    assert!(matches!(
        import_beng(&doc),
        Err(BengImportError::StrijdigCompleet(_))
    ));
}

#[test]
fn onvolledig_document_met_schemafout_elders_blijft_een_fout() {
    let mut doc = fixture();
    doc["compleet"] = Value::Bool(false);
    doc["beng_geometry"]["opaque_defs"][0]
        .as_object_mut()
        .unwrap()
        .remove("thermal");
    doc["beng_geometry"]["zones"][0]["gevels"][0]["vlak_type"] = Value::String("muur".into());
    match import_beng(&doc) {
        Err(BengImportError::Schema { pad, .. }) => {
            assert_eq!(pad, "beng_geometry.zones[0].gevels[0].vlak_type");
        }
        other => panic!("verwacht schemafout, kreeg {other:?}"),
    }
}

#[test]
fn onbekende_topsleutel_wordt_geweigerd() {
    let mut doc = fixture();
    doc["zonez"] = serde_json::json!([]);
    assert!(matches!(
        import_beng(&doc),
        Err(BengImportError::Schema { .. })
    ));
}

#[test]
fn verkeerd_format_of_versie_wordt_geweigerd() {
    let mut doc = fixture();
    doc["version"] = Value::String("1.1".into());
    assert!(matches!(
        import_beng(&doc),
        Err(BengImportError::Envelope {
            veld: "version",
            ..
        })
    ));
    let mut doc = fixture();
    doc["format"] = Value::String("thermal".into());
    assert!(matches!(
        import_beng(&doc),
        Err(BengImportError::Envelope { veld: "format", .. })
    ));
    assert!(matches!(
        import_beng(&serde_json::json!([])),
        Err(BengImportError::GeenObject(_))
    ));
}

#[test]
fn inhoudelijk_ongeldige_geometrie_is_een_validatiefout() {
    let mut doc = fixture();
    doc["beng_geometry"]["zones"][0]["gevels"][1]["constructie_ref"] =
        Value::String("bestaat-niet".into());
    assert!(matches!(
        import_beng(&doc),
        Err(BengImportError::Validatie(_))
    ));
    let mut doc = fixture();
    doc["beng_geometry"]["zones"][0]["gevels"][1]["ramen"][0]["belemmering"] =
        serde_json::json!({"overhang": {"h_o": 0.0}});
    assert!(matches!(
        import_beng(&doc),
        Err(BengImportError::Validatie(_))
    ));
}

/// e2e: import → `project.beng_geometry` → `compute_beng`.
///
/// Handberekening (NTA 8800 §6.7.2/§6.7.3, Bbl tabel 4.148A rij 1e):
/// - A_ls = 0,7·40 (vloer op grond) + 30 + 30 + 45 + 0·50 (woningscheidend, AVR)
///   = 28 + 105 = 133 m²; A_ls/A_g = 133/80 = 1,6625.
/// - BENG1-grens = 55 + 30·(1,6625 − 1,5) = 59,875; BENG2 30; BENG3 50.
#[test]
fn e2e_import_naar_project_en_beng() {
    let r = import_beng(&fixture()).unwrap();
    let mut p = ProjectV2::new("synthetisch");
    p.shared.building_type = BuildingTypeShared::Woning {
        subtype: ResidentialType::Terraced,
    };
    p.beng_geometry = r.beng_geometry;
    p.energy = Some(EnergyInput {
        heating: Some(HeatingInput {
            generator: HeatGeneratorType::HeatPumpGround,
            cop: Some(4.5),
            hr_class: None,
            district_factor: None,
            emission: Some(HeatEmissionType::FloorHeating),
            distribution_efficiency: None,
            control_factor: None,
            coverage_fraction: 1.0,
            source: None,
        }),
        ventilation: Some(VentilationInput {
            system: VentilationSystemType::D,
            wtw_efficiency: Some(0.85),
            sfp_w_per_m3h: None,
            bypass_enabled: true,
            mechanical_supply_m3_per_h: Some(120.0),
            mechanical_exhaust_m3_per_h: Some(120.0),
            infiltration_m3_per_h: None,
            q_v10_spec_dm3_s_m2: None,
            source: None,
        }),
        ..Default::default()
    });

    let res = compute_beng(&p).expect("compute_beng op geïmporteerde geometrie");
    assert!((res.a_g_m2 - 80.0).abs() < 1e-9);
    assert!((res.a_ls_m2 - 133.0).abs() < 1e-9, "A_ls = {}", res.a_ls_m2);
    assert!((res.beng1.limit.unwrap() - 59.875).abs() < 1e-9);
    assert!((res.beng2.limit.unwrap() - 30.0).abs() < 1e-9);
    assert!((res.beng3.limit.unwrap() - 50.0).abs() < 1e-9);
    assert!(res.beng1.value.is_finite() && res.beng1.value > 0.0);
}

/// Het gecommitte schema is exact wat de Rust-types genereren. Bij een
/// bewuste wijziging: `UPDATE_SCHEMA=1 cargo test -p openaec-project-shared
/// --test beng_import_test` en de nieuwe sha melden aan ifc-ruimtebalans.
#[test]
fn schema_bestand_is_actueel() {
    let gegenereerd = beng_import_schema_json();
    if std::env::var_os("UPDATE_SCHEMA").is_some() {
        std::fs::write(SCHEMA, &gegenereerd).unwrap();
    }
    let op_schijf = std::fs::read_to_string(SCHEMA)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(
        op_schijf, gegenereerd,
        "schemas/v1/beng-import.schema.json is verouderd"
    );
}

#[test]
fn schema_laat_thermal_en_u_w_optioneel() {
    let s: Value = serde_json::from_str(&beng_import_schema_json()).unwrap();
    let req = |def: &str| -> Vec<String> {
        s["definitions"][def]["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    };
    assert!(!req("OpaqueConstructionDef").contains(&"thermal".to_string()));
    assert!(!req("WindowDef").contains(&"u_w_per_m2k".to_string()));
    assert!(req("WindowDef").contains(&"area_m2".to_string()));
    assert_eq!(s["additionalProperties"], Value::Bool(false));
}

#[test]
fn onvolledig_document_met_ongeldige_inhoud_is_een_validatiefout() {
    let mut doc = fixture();
    doc["compleet"] = Value::Bool(false);
    doc["beng_geometry"]["opaque_defs"][0].as_object_mut().unwrap().remove("thermal");
    doc["beng_geometry"]["zones"][0]["gevels"][1]["constructie_ref"] = Value::String("weg".into());
    assert!(matches!(import_beng(&doc), Err(BengImportError::Validatie(_))));
}
