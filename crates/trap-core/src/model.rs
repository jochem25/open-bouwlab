//! Trapmodel: de geometrische primitieven die de rekenkern oplevert en de
//! IFC-schrijver vertaalt. Maten in mm, projectcoordinaten; z absoluut.
//!
//! Sinds fase 2 geen verstek meer: de hoekverbinding van de bomen is koud
//! (stomp), dus een boomdeel is een zuivere extrusie min nesten.

use serde::{Deserialize, Serialize};

pub type P2 = [f64; 2];
pub type P3 = [f64; 3];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trapmodel {
    #[serde(default)]
    pub bron: String,
    pub projectnaam: String,
    pub bouwlaag: String,
    pub z_bouwlaag: f64,
    pub trap: Trap,
    /// Pset_StairCommon (en, gefilterd, Pset_StairFlightCommon).
    pub pset_trap: Vec<(String, Waarde)>,
    /// KBA_Trapverdrijving.
    pub pset_extra: Vec<(String, Waarde)>,
    pub treden: Vec<Prisma>,
    pub borden: Vec<Prisma>,
    pub bomen: Vec<Boom>,
    #[serde(default)]
    pub palen: Vec<Paal>,
    pub clearance: Option<Clearance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trap {
    pub naam: String,
    /// IfcStair PredefinedType, bv. QUARTER_WINDING_STAIR.
    #[serde(default = "standaard_vorm")]
    pub vorm: String,
    /// IfcStairFlight PredefinedType: WINDER (verdreven) of STRAIGHT.
    #[serde(default = "standaard_steek")]
    pub steek: String,
    pub optreden: u32,
    pub aantreden: u32,
    pub optrede: f64,
    pub aantrede: f64,
}

fn standaard_vorm() -> String {
    "QUARTER_WINDING_STAIR".into()
}

fn standaard_steek() -> String {
    "WINDER".into()
}

/// Waarde van een eigenschap; het IFC-meettype volgt uit de pset (zie `ifc`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Waarde {
    Bool(bool),
    Int(i64),
    Real(f64),
    Label(String),
}

/// Verticaal prisma: veelhoek in plattegrond tussen z0 en z1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prisma {
    pub naam: String,
    /// TREDE, WELSTUK, STOOTBORD, PAAL of NEST.
    pub soort: String,
    pub veelhoek: Vec<P2>,
    pub z0: f64,
    pub z1: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Boom {
    pub naam: String,
    pub delen: Vec<BoomdeelMetNesten>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoomdeelMetNesten {
    pub deel: Boomdeel,
    pub nesten: Vec<Prisma>,
}

/// Recht deel van een boom: uitslag (u langs het deel, z omhoog) over de dikte.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Boomdeel {
    pub naam: String,
    /// Plattegrondpunt van u = 0 aan de binnenkant van het deel.
    pub oorsprong: P2,
    /// Eenheidsvector langs het deel.
    pub richting: P2,
    /// Eenheidsvector loodrecht op het deel, van de trap af.
    pub naar_buiten: P2,
    pub dikte: f64,
    /// Gesloten keten; elk element begint waar het vorige eindigt.
    pub profiel: Vec<Profielelement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "soort", rename_all = "lowercase")]
pub enum Profielelement {
    Lijn {
        p0: P2,
        p1: P2,
    },
    /// Cirkelboog van p0 naar p1 rond `centrum`; `boven` = bovenste helft (z > centrum).
    Boog {
        p0: P2,
        p1: P2,
        centrum: P2,
        r: f64,
        boven: bool,
    },
}

impl Profielelement {
    pub fn p0(&self) -> P2 {
        match self {
            Self::Lijn { p0, .. } | Self::Boog { p0, .. } => *p0,
        }
    }
    pub fn p1(&self) -> P2 {
        match self {
            Self::Lijn { p1, .. } | Self::Boog { p1, .. } => *p1,
        }
    }
}

/// Punt op de boog halverwege in u (zoals `Boog.midden` in de Python-referentie).
pub fn boog_midden(p0: P2, p1: P2, centrum: P2, r: f64, boven: bool) -> P2 {
    let um = (p0[0] + p1[0]) / 2.0;
    let du = um - centrum[0];
    let h = (r * r - du * du).max(0.0).sqrt();
    [um, centrum[1] + if boven { h } else { -h }]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paal {
    pub paal: Prisma,
    pub nesten: Vec<Prisma>,
    /// KBA_Trappaal; 'Materiaal' wordt ook een IfcMaterial.
    pub eigenschappen: Vec<(String, Waarde)>,
}

/// Gesloten schil van vlakke veelhoeken (tegen de klok in van buiten gezien).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clearance {
    pub naam: String,
    pub hoekpunten: Vec<P3>,
    pub vlakken: Vec<Vec<usize>>,
    /// KBA_Clearance.
    pub eigenschappen: Vec<(String, Waarde)>,
    /// Hulpkopie als IfcSpatialZone; in v5 uit (Zoom toont de representatie zelf).
    #[serde(default)]
    pub met_zone: bool,
}
