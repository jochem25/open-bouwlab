//! Valideert de rapport-JSON voor S1 en B1 tegen het schema van openaec-reports
//! (submodule `libs/openaec-reports`). Zonder uitgecheckte submodule wordt de test overgeslagen.

use constructie_core::rapport::{rapport_json, RapportBerekening, RapportInvoer};
use constructie_core::{bereken_beton, bereken_staal, BetonInvoer, StaalInvoer};
use serde_json::Value;

const SCHEMA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../libs/openaec-reports/schemas/report.schema.json"
);

fn valideer(rapport: &Value) {
    let Ok(tekst) = std::fs::read_to_string(SCHEMA) else {
        eprintln!("schema ontbreekt (submodule niet uitgecheckt): test overgeslagen");
        return;
    };
    let schema: Value = serde_json::from_str(&tekst).expect("schema is JSON");
    let validator = jsonschema::validator_for(&schema).expect("schema compileert");
    let fouten: Vec<String> = validator
        .iter_errors(rapport)
        .map(|e| format!("{} @ {}", e, e.instance_path))
        .collect();
    assert!(fouten.is_empty(), "schemafouten: {fouten:#?}");
    assert!(rapport.to_string().contains(constructie_core::DISCLAIMER));
}

fn invoer<'a>(berekening: RapportBerekening<'a>) -> RapportInvoer<'a> {
    RapportInvoer {
        project: "Proef",
        projectnummer: "",
        opdrachtgever: "",
        auteur: "Tester",
        datum: "2026-10-07",
        berekening,
    }
}

#[test]
fn s1_rapport_valideert() {
    let i: StaalInvoer = serde_json::from_str(
        r#"{"algemeen":{"overspanning_m":5.4,"permanent_kn_m2":0.75,"gevolgklasse":"CC2"},
        "belastingbreedte_m":3.6,"staalsoort":"S235","reeksen":["IPE","HEA"]}"#,
    )
    .unwrap();
    let r = bereken_staal(&i).unwrap();
    valideer(&rapport_json(&invoer(RapportBerekening::Staal {
        invoer: &i,
        resultaat: &r,
    })));
}

#[test]
fn b1_rapport_valideert() {
    let i: BetonInvoer = serde_json::from_str(
        r#"{"algemeen":{"overspanning_m":6.0,"permanent_kn_m2":6.0,"gevolgklasse":"CC2"},
        "belastingbreedte_m":3.0,"sterkteklasse":"C30/37","balkbreedte_mm":300,"milieuklasse":"XC1"}"#,
    )
    .unwrap();
    let r = bereken_beton(&i).unwrap();
    valideer(&rapport_json(&invoer(RapportBerekening::Beton {
        invoer: &i,
        resultaat: &r,
    })));
}
