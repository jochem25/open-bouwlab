//! Trapontwerper - rekenkern (fase 2, O-2026-10-30-03).
//!
//! Invoer-JSON (trapgat/plattegrond, niveaus, onderdelen, verdrijving) naar een
//! trapmodel (treden, stootborden, bomen met uitslag en koude hoekverbinding,
//! trappaal, clearance), een toetsing met bron per regel (Bbl uit het
//! normcorpus, Wattjes als advies) en IFC4 (STEP P21).
//!
//! De meetkunde is een port van de Python-referentie (ai-tekenaar,
//! `trappen_verdrijving`, IFC v5); de golden tests vergelijken ermee.
//!
//! Puur: geen I/O, geen klok, geen randomness. Dezelfde invoer geeft hetzelfde
//! bestand (GUID's zijn afgeleid van de elementnamen).

pub mod boom;
pub mod bouw;
pub mod clearance;
pub mod ifc;
pub mod invoer;
pub mod model;
pub mod onderdelen;
pub mod p21;
pub mod toets;
pub mod verdrijving;
pub mod vlak;

pub use bouw::{ontwerp, Ontwerp, Rapport, DISCLAIMER};
pub use ifc::{schrijf_ifc, IfcOpties};
pub use invoer::Invoer;
pub use model::Trapmodel;

#[derive(Debug, thiserror::Error)]
pub enum Fout {
    #[error("ongeldige invoer-JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ongeldige invoer: {0}")]
    Invoer(String),
    /// De opgave laat geen oplossing toe (bv. verdrijving onmogelijk).
    #[error("ontwerp niet mogelijk: {0}")]
    Ontwerp(String),
    #[error("ongeldig trapmodel: {0}")]
    Model(String),
}

/// Trapmodel-JSON in, IFC4 STEP-tekst uit.
pub fn ifc_from_json(json: &str, opties: &IfcOpties) -> Result<String, Fout> {
    let model: Trapmodel = serde_json::from_str(json)?;
    schrijf_ifc(&model, opties)
}

/// Invoer-JSON in: (IFC4 STEP-tekst, toets-JSON).
pub fn ontwerp_from_json(json: &str, opties: &IfcOpties) -> Result<(String, String), Fout> {
    let invoer: Invoer = serde_json::from_str(json)?;
    let o = ontwerp(&invoer)?;
    let ifc = schrijf_ifc(&o.model, opties)?;
    let toets = serde_json::to_string_pretty(&o.rapport)? + "\n";
    Ok((ifc, toets))
}
