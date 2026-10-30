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

pub use bouw::{ontwerp, Ontwerp, Rapport, Tekening, DISCLAIMER};
pub use ifc::{schrijf_ifc, ClearanceIfc, IfcOpties};
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

/// Alles wat het scherm nodig heeft: toetsing, tekening en twee IFC's.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Uitvoer {
    pub rapport: Rapport,
    pub tekening: Tekening,
    /// IFC4 met de clearance als representatie en als proxy (coordinatie).
    pub ifc: String,
    /// IFC4 zonder clearance-geometrie (voor de leverancier).
    pub ifc_leverancier: String,
    /// Voorgestelde bestandsnaam zonder extensie.
    pub bestandsnaam: String,
}

/// Bestandsnaam uit de trapnaam: kleine letters, cijfers en '-'.
pub fn bestandsnaam(naam: &str) -> String {
    let mut s = String::new();
    for c in naam.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-');
    if s.is_empty() { "trap".into() } else { s.into() }
}

/// Invoer in, scherm-uitvoer uit. `tijd` = FILE_NAME-tijdstempel (ISO 8601).
pub fn ontwerp_uitvoer(invoer: &Invoer, tijd: &str) -> Result<Uitvoer, Fout> {
    let o = ontwerp(invoer)?;
    let naam = bestandsnaam(&invoer.naam);
    let opties = |achtervoegsel: &str, clearance| IfcOpties {
        tijd: tijd.into(),
        bestandsnaam: format!("{naam}{achtervoegsel}.ifc"),
        disclaimer: Some(DISCLAIMER.into()),
        clearance,
    };
    Ok(Uitvoer {
        ifc: schrijf_ifc(&o.model, &opties("", ClearanceIfc::RepresentatieEnProxy))?,
        ifc_leverancier: schrijf_ifc(&o.model, &opties("_leverancier", ClearanceIfc::Geen))?,
        rapport: o.rapport,
        tekening: o.tekening,
        bestandsnaam: naam,
    })
}
