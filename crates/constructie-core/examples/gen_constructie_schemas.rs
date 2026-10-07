//! Genereert de JSON-schema's naar `schemas/constructie/v1/`.
//! Draai vanuit de workspace-root: `cargo run -p constructie-core --example gen_constructie_schemas`
use constructie_core::{beton_invoer_schema, resultaat_schema, staal_invoer_schema};

fn main() {
    let uit = [
        ("staal-invoer.schema.json", staal_invoer_schema()),
        ("beton-invoer.schema.json", beton_invoer_schema()),
        ("resultaat.schema.json", resultaat_schema()),
    ];
    for (naam, inhoud) in uit {
        let pad = format!("schemas/constructie/v1/{naam}");
        std::fs::write(&pad, &inhoud).expect("schema schrijven");
        println!("Geschreven {pad} ({} bytes)", inhoud.len());
    }
}
