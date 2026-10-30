//! Het JSON-schema van de invoer is gegenereerd uit de Rust-types en moet
//! actueel zijn. Bijwerken: `cargo run -p trap-core --bin trap -- schema >
//! crates/trap-core/schema/trap-invoer.schema.json`.

use trap_core::invoer::{schema, Invoer};

#[test]
fn schema_is_actueel() {
    let opgeslagen = include_str!("../schema/trap-invoer.schema.json").replace("\r\n", "\n");
    assert_eq!(opgeslagen, schema(), "schema verouderd: opnieuw genereren");
}

#[test]
fn fixtures_voldoen_aan_types() {
    for f in [
        include_str!("fixtures/invoer_kwartdraai_referentie.json"),
        include_str!("fixtures/invoer_kwartdraai_leverancier.json"),
    ] {
        serde_json::from_str::<Invoer>(f).expect("fixture");
    }
}

#[test]
fn onbekend_veld_faalt() {
    let j = r#"{"naam":"x","niveaus":{"onder":0,"boven":2800},"plattegrond":{"soort":"armen","start":[0,0],"richting":[1,0],"binnenzijde":"links","armlengtes":[3000],"breedte":850},"optrede":17}"#;
    assert!(serde_json::from_str::<Invoer>(j).is_err(), "deny_unknown_fields");
}
