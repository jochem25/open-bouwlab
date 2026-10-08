//! Valideert de rapport-JSON voor S1 en B1 tegen het schema van openaec-reports
//! (submodule `libs/openaec-reports`). Zonder uitgecheckte submodule wordt de test overgeslagen.

use constructie_core::rapport::{rapport_json, RapportBerekening, RapportInvoer};
use constructie_core::{
    bereken_beton, bereken_hout, bereken_staal, BetonInvoer, HoutInvoer, StaalInvoer,
};
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

#[test]
fn h1_rapport_valideert() {
    let i: HoutInvoer = serde_json::from_str(
        r#"{"algemeen":{"overspanning_m":4.2,"permanent_kn_m2":0.75,"eigen_gewicht_automatisch":false,"gevolgklasse":"CC2"},
        "element":{"type":"balklaag","hoh_mm":400},"sterkteklasse":"C24",
        "vloerplaat":{"dikte_mm":18,"e_mean_n_mm2":4000},"vloerbreedte_m":4.0}"#,
    )
    .unwrap();
    let r = bereken_hout(&i).unwrap();
    valideer(&rapport_json(&invoer(RapportBerekening::Hout {
        invoer: &i,
        resultaat: &r,
    })));
}

#[test]
fn h1_rapport_met_trilling_uit_bevat_de_zin_en_valideert() {
    let i: HoutInvoer = serde_json::from_str(
        r#"{"algemeen":{"overspanning_m":4.2,"permanent_kn_m2":0.75,"eigen_gewicht_automatisch":false,"gevolgklasse":"CC2"},
        "element":{"type":"balklaag","hoh_mm":400},"sterkteklasse":"C24","trillingstoets":false,
        "vloerlagen":[{"soort":"vloerplaat","dikte_mm":18,"e_mean_n_mm2":4000},
                      {"soort":"plafond","dikte_mm":12.5,"e_mean_n_mm2":2000}]}"#,
    )
    .unwrap();
    let r = bereken_hout(&i).unwrap();
    let rapport = rapport_json(&invoer(RapportBerekening::Hout {
        invoer: &i,
        resultaat: &r,
    }));
    valideer(&rapport);
    let tekst = rapport.to_string();
    assert!(tekst.contains(constructie_core::hout::TRILLING_UIT_ZIN));
    assert!(tekst.contains("Trillingstoets (EC5 7.3) meenemen"));
    // Direct na de disclaimer, als aparte vette alinea.
    let inhoud = &rapport["sections"][0]["content"];
    assert_eq!(inhoud.as_array().unwrap().len(), 2);
    assert!(inhoud[1]["text"]
        .as_str()
        .unwrap()
        .starts_with("<b>Trillingen"));
}
