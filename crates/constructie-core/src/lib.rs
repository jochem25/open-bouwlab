//! # constructie-core
//!
//! Rekenkern voor indicatieve voordimensionering van vrij opgelegde liggers
//! (gewalst staal, gewapend beton en massief hout) in het ontwerpstadium.
//!
//! De kern is puur: geen I/O, geen async, geen unsafe. Invoer en uitvoer zijn
//! serde-structs of JSON. Elke toets draagt een bronverwijzing (norm en artikel,
//! nationale bijlage apart). De crate bevat geen normtekst.
//!
//! Sterkteklassewaarden van hout komen uit een secundaire bron en zijn te verifieren.
//!
//! ```rust,no_run
//! use constructie_core::bereken_staal_from_json;
//!
//! let invoer = r#"{ "algemeen": { "overspanning_m": 5.4, "permanent_kn_m2": 0.75 },
//!                   "belastingbreedte_m": 3.6, "staalsoort": "S235", "reeksen": ["IPE"] }"#;
//! let uitvoer = bereken_staal_from_json(invoer).unwrap();
//! ```

pub mod advies;
pub mod belasting;
pub mod beton;
pub mod error;
pub mod hout;
pub mod model;
pub mod rapport;
pub mod staal;
pub mod toets;

pub use error::{ConstructieFout, Result};
pub use model::{
    BetonInvoer, BetonResultaat, HoutInvoer, HoutResultaat, Materiaal, Resultaat, StaalInvoer,
    StaalResultaat,
};
pub use toets::fmt_uc;

/// Versie van de rekenkern.
pub const KERN_VERSIE: &str = env!("CARGO_PKG_VERSION");

/// Vaste tekst E1: wordt in elk resultaat en elk rapport getoond en kan niet worden uitgeschakeld.
pub const DISCLAIMER: &str = "Indicatieve voordimensionering - vervangt geen constructieve berekening. Definitieve afmetingen, verbindingen en opleggingen worden door de constructeur bepaald.";

/// Gebruikte normedities: (sleutel, editie).
pub const NORMEDITIES: &[(&str, &str)] = &[
    ("EC0", "NEN-EN 1990+A1+NB:2019"),
    ("EC1-1", "NEN-EN 1991-1-1+C11+NB:2019"),
    ("EC1-3", "NEN-EN 1991-1-3+A1+NB:2019"),
    ("EC2", "NEN-EN 1992-1-1+A1+NB:2016+A1:2020"),
    ("EC3", "NEN-EN 1993-1-1+A1+NB:2016"),
    ("EC5", "NEN-EN 1995-1-1+A2+NB:2013"),
];

/// Voordimensionering van een stalen ligger.
pub fn bereken_staal(invoer: &StaalInvoer) -> Result<StaalResultaat> {
    staal::bereken_staal(invoer)
}

/// Voordimensionering van een betonbalk.
pub fn bereken_beton(invoer: &BetonInvoer) -> Result<BetonResultaat> {
    beton::bereken_beton(invoer)
}

/// Voordimensionering van een houten balk of balklaag.
pub fn bereken_hout(invoer: &HoutInvoer) -> Result<HoutResultaat> {
    hout::bereken_hout(invoer)
}

/// Als [`bereken_staal`], met JSON in en uit.
pub fn bereken_staal_from_json(invoer_json: &str) -> Result<String> {
    let invoer: StaalInvoer = serde_json::from_str(invoer_json)?;
    Ok(serde_json::to_string_pretty(&bereken_staal(&invoer)?)?)
}

/// Als [`bereken_beton`], met JSON in en uit.
pub fn bereken_beton_from_json(invoer_json: &str) -> Result<String> {
    let invoer: BetonInvoer = serde_json::from_str(invoer_json)?;
    Ok(serde_json::to_string_pretty(&bereken_beton(&invoer)?)?)
}

/// Als [`bereken_hout`], met JSON in en uit.
pub fn bereken_hout_from_json(invoer_json: &str) -> Result<String> {
    let invoer: HoutInvoer = serde_json::from_str(invoer_json)?;
    Ok(serde_json::to_string_pretty(&bereken_hout(&invoer)?)?)
}

/// Rekent met JSON voor het gekozen materiaal.
pub fn bereken_van_materiaal_json(materiaal: Materiaal, invoer_json: &str) -> Result<String> {
    match materiaal {
        Materiaal::Staal => bereken_staal_from_json(invoer_json),
        Materiaal::Beton => bereken_beton_from_json(invoer_json),
        Materiaal::Hout => bereken_hout_from_json(invoer_json),
    }
}

/// JSON-schema van [`StaalInvoer`].
pub fn staal_invoer_schema() -> String {
    schema_json(schemars::schema_for!(StaalInvoer))
}

/// JSON-schema van [`BetonInvoer`].
pub fn beton_invoer_schema() -> String {
    schema_json(schemars::schema_for!(BetonInvoer))
}

/// JSON-schema van [`HoutInvoer`].
pub fn hout_invoer_schema() -> String {
    schema_json(schemars::schema_for!(HoutInvoer))
}

/// JSON-schema van [`Resultaat`] (staal, beton en hout).
pub fn resultaat_schema() -> String {
    schema_json(schemars::schema_for!(Resultaat))
}

fn schema_json(schema: schemars::schema::RootSchema) -> String {
    serde_json::to_string_pretty(&schema).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hout_via_materiaal_geeft_resultaat() {
        let json = r#"{ "algemeen": { "overspanning_m": 4.2, "permanent_kn_m2": 0.75 },
            "element": { "type": "balklaag", "hoh_mm": 400 }, "sterkteklasse": "C24" }"#;
        let uit = bereken_van_materiaal_json(Materiaal::Hout, json).unwrap();
        let r: Resultaat = serde_json::from_str(&uit).unwrap();
        assert_eq!(r.disclaimer, DISCLAIMER);
        assert!(!r.kandidaten.is_empty());
        // Ongeldige invoer blijft een nette fout.
        assert!(bereken_van_materiaal_json(Materiaal::Hout, "{}").is_err());
    }

    #[test]
    fn disclaimer_altijd_in_resultaat() {
        let json = r#"{ "algemeen": { "overspanning_m": 5.4, "permanent_kn_m2": 0.75 },
            "belastingbreedte_m": 3.6, "staalsoort": "S235", "reeksen": ["IPE"] }"#;
        let uit = bereken_staal_from_json(json).unwrap();
        let r: Resultaat = serde_json::from_str(&uit).unwrap();
        assert_eq!(r.disclaimer, DISCLAIMER);
        assert_eq!(r.kern_versie, KERN_VERSIE);
        // Ook zonder advies (geen profielen in reeks HEB) blijft de disclaimer staan.
        let json = r#"{ "algemeen": { "overspanning_m": 5.4, "permanent_kn_m2": 0.75 },
            "belastingbreedte_m": 3.6, "staalsoort": "S235", "reeksen": ["HEB"] }"#;
        let r: Resultaat = serde_json::from_str(&bereken_staal_from_json(json).unwrap()).unwrap();
        assert!(r.advies.is_none());
        assert_eq!(r.disclaimer, DISCLAIMER);
    }

    #[test]
    fn onbekend_veld_in_invoer_wordt_geweigerd() {
        let json = r#"{ "algemeen": { "overspanning_m": 5.4, "permanent_kn_m2": 0.75 },
            "belastingbreedte_m": 3.6, "staalsoort": "S235", "reeksen": ["IPE"], "onzin": 1 }"#;
        assert!(matches!(
            bereken_staal_from_json(json),
            Err(ConstructieFout::Json(_))
        ));
        let json = r#"{ "algemeen": { "overspanning_m": 5.4, "permanent_kn_m2": 0.75, "x": 1 },
            "belastingbreedte_m": 3.6, "sterkteklasse": "C30/37", "balkbreedte_mm": 300 }"#;
        assert!(bereken_beton_from_json(json).is_err());
    }
}
