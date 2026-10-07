//! Foutafhandeling van de constructie-rekenkern.

use thiserror::Error;

/// Fouten die de rekenkern kan teruggeven.
#[derive(Debug, Error)]
pub enum ConstructieFout {
    /// De JSON-invoer of -uitvoer kon niet worden verwerkt.
    #[error("ongeldige JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// De invoer is inhoudelijk ongeldig (bereik, combinatie van velden).
    #[error("ongeldige invoer: {0}")]
    Invoer(String),
    /// De gevraagde functie bestaat in deze versie nog niet.
    #[error("niet beschikbaar in deze versie: {0}")]
    NietBeschikbaar(String),
    /// De meegeleverde profieldata is ongeldig.
    #[error("ongeldige profieldata: {0}")]
    Data(String),
}

/// Resultaattype van de rekenkern.
pub type Result<T> = std::result::Result<T, ConstructieFout>;
