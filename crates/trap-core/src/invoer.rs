//! Invoermodel van de trapontwerper (fase 2). JSON in, met JSON-schema
//! (`schema/trap-invoer.schema.json`, gegenereerd uit deze types).
//!
//! Maten in mm, niveaus absoluut (mm), plattegrond in projectcoordinaten.
//! Alles met een standaardwaarde mag weg; de standaard staat bij het veld.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model::P2;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Invoer {
    /// Naam van de trap (ook sleutel voor de GUID's in het IFC).
    pub naam: String,
    #[serde(default)]
    pub project: Project,
    pub niveaus: Niveaus,
    pub plattegrond: Plattegrond,
    /// Aantal optreden. Weg = automatisch: kleinste aantal waarbij de optrede
    /// aan het Bbl voldoet, met de stapmaat het dichtst bij 630 mm.
    #[serde(default)]
    pub optreden: Option<u32>,
    #[serde(default)]
    pub boom: BoomInvoer,
    #[serde(default)]
    pub trede: TredeInvoer,
    #[serde(default)]
    pub stootbord: StootbordInvoer,
    /// Trappaal in elke binnenhoek (verplicht bij een draai).
    #[serde(default)]
    pub trappaal: Option<TrappaalInvoer>,
    #[serde(default)]
    pub verdrijving: VerdrijvingInvoer,
    #[serde(default)]
    pub looplijn: LooplijnInvoer,
    #[serde(default)]
    pub hoekverbinding: HoekverbindingInvoer,
    #[serde(default)]
    pub clearance: ClearanceInvoer,
    #[serde(default)]
    pub gebruiksfunctie: Gebruiksfunctie,
    #[serde(default)]
    pub bouwsoort: Bouwsoort,
    /// Optioneel bestaand trapgat (veelhoek in plattegrond): dan wordt
    /// gecontroleerd of de clearance erbinnen valt. Zonder trapgat volgt het
    /// minimale trapgat uit de trap (de trap is leidend).
    #[serde(default)]
    pub trapgat: Option<Vec<P2>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Project {
    #[serde(default = "standaard_projectnaam")]
    pub naam: String,
    #[serde(default = "standaard_bouwlaag")]
    pub bouwlaag: String,
    /// Peil van de bouwlaag in het IFC; weg = niveau onder.
    #[serde(default)]
    pub z_bouwlaag: Option<f64>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            naam: standaard_projectnaam(),
            bouwlaag: standaard_bouwlaag(),
            z_bouwlaag: None,
        }
    }
}

fn standaard_projectnaam() -> String {
    "trapontwerp".into()
}
fn standaard_bouwlaag() -> String {
    "bouwlaag".into()
}

/// Afgewerkte vloerniveaus onder en boven de trap.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Niveaus {
    pub onder: f64,
    pub boven: f64,
    /// Dikte van de bovenvloer (voor het minimale trapgat); standaard 300.
    #[serde(default = "standaard_vloerdikte")]
    pub vloerdikte_boven: f64,
}

fn standaard_vloerdikte() -> f64 {
    300.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Kant {
    Links,
    Rechts,
}

impl Kant {
    pub fn andere(self) -> Self {
        match self {
            Self::Links => Self::Rechts,
            Self::Rechts => Self::Links,
        }
    }
}

/// De trap in plattegrond.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "soort", rename_all = "lowercase")]
pub enum Plattegrond {
    /// Parametrisch: rechte armen met kwartdraaien in dezelfde draaizin.
    /// Eén arm = rechte trap, twee = kwartdraai, drie = halfdraai.
    Armen(Armen),
    /// Vrij: binnen- en buitenzijde van de loopbreedte als polylijnen in de
    /// klimrichting, van de voorkant van de eerste trede tot de voorkant van
    /// het welstuk; het welstuk loopt `welstuk` door langs het laatste segment.
    Contour(ContourInvoer),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Armen {
    /// Punt op de buitenzijde van de loopbreedte, op de voorkant van trede 1.
    pub start: P2,
    /// Klimrichting van de eerste arm (wordt genormaliseerd).
    pub richting: P2,
    /// Kant van de binnenzijde (= draaizin: 'links' draait linksom).
    pub binnenzijde: Kant,
    /// Lengte per arm langs de buitenzijde van de loopbreedte: eerste arm van
    /// de voorkant van trede 1 tot de hoek, middelste armen hoek tot hoek,
    /// laatste arm van de hoek tot de voorkant van het welstuk.
    pub armlengtes: Vec<f64>,
    /// Breedte, zie `breedte_is`.
    pub breedte: f64,
    /// 'loopbreedte' (tussen de bomen) of 'buitenwerks' (over de bomen); bij
    /// buitenwerks gelden ook de armlengtes buitenwerks (buitenkant buitenboom).
    #[serde(default)]
    pub breedte_is: Breedtemaat,
    /// Diepte van het welstuk (voorkant tot trapgatrand); standaard 80.
    #[serde(default = "standaard_welstuk")]
    pub welstuk: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Breedtemaat {
    #[default]
    Loopbreedte,
    Buitenwerks,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContourInvoer {
    pub binnenzijde: Vec<P2>,
    pub buitenzijde: Vec<P2>,
    pub kant_binnen: Kant,
    #[serde(default = "standaard_welstuk")]
    pub welstuk: f64,
}

fn standaard_welstuk() -> f64 {
    80.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoomInvoer {
    /// Dikte; standaard 40.
    #[serde(default = "standaard_40")]
    pub dikte: f64,
    /// Plankbreedte loodrecht op de bovenkant; standaard 320.
    #[serde(default = "standaard_plank")]
    pub plankbreedte: f64,
    /// Hout boven de tredevoorkanten, loodrecht; standaard 50.
    #[serde(default = "standaard_voorhout")]
    pub voorhout: f64,
    /// Minimum hout onder de nesten; standaard 20.
    #[serde(default = "standaard_achterhout")]
    pub achterhout_min: f64,
    /// Nestdiepte (treden en stootborden in de boom); standaard 10.
    #[serde(default = "standaard_nest")]
    pub nestdiepte: f64,
    #[serde(default = "standaard_materiaal")]
    pub materiaal: String,
}

impl Default for BoomInvoer {
    fn default() -> Self {
        Self {
            dikte: 40.0,
            plankbreedte: standaard_plank(),
            voorhout: standaard_voorhout(),
            achterhout_min: standaard_achterhout(),
            nestdiepte: standaard_nest(),
            materiaal: standaard_materiaal(),
        }
    }
}

fn standaard_40() -> f64 {
    40.0
}
fn standaard_plank() -> f64 {
    320.0
}
fn standaard_voorhout() -> f64 {
    50.0
}
fn standaard_achterhout() -> f64 {
    20.0
}
fn standaard_nest() -> f64 {
    10.0
}
fn standaard_materiaal() -> String {
    "vurenhout".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TredeInvoer {
    /// Standaard 40.
    #[serde(default = "standaard_40")]
    pub dikte: f64,
    /// Neus voorbij het voorvlak van het stootbord; standaard 42.
    #[serde(default = "standaard_neus")]
    pub neus: f64,
}

impl Default for TredeInvoer {
    fn default() -> Self {
        Self {
            dikte: 40.0,
            neus: standaard_neus(),
        }
    }
}

fn standaard_neus() -> f64 {
    42.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StootbordInvoer {
    /// 'dicht' of 'open' (open trap: geen stootborden).
    #[serde(default)]
    pub soort: Stootbordsoort,
    /// Standaard 18.
    #[serde(default = "standaard_stootbord")]
    pub dikte: f64,
}

impl Default for StootbordInvoer {
    fn default() -> Self {
        Self {
            soort: Stootbordsoort::Dicht,
            dikte: standaard_stootbord(),
        }
    }
}

fn standaard_stootbord() -> f64 {
    18.0
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Stootbordsoort {
    #[default]
    Dicht,
    Open,
}

/// Trappaal in een binnenhoek: rechthoek in de hoek aan de boomkant; de
/// binnenbomen komen er stomp tegen.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrappaalInvoer {
    /// Maat langs de inkomende arm, vanaf de binnenhoek terug; standaard 90.
    #[serde(default = "standaard_paal")]
    pub langs_in: f64,
    /// Maat langs de uitgaande arm, vanaf de binnenhoek; standaard 90.
    #[serde(default = "standaard_paal")]
    pub langs_uit: f64,
    /// Hoe ver de paal de loopbreedte van de inkomende arm insteekt; standaard 0.
    #[serde(default)]
    pub insteek_in: f64,
    /// Hoe ver de paal de loopbreedte van de uitgaande arm insteekt; standaard 0.
    #[serde(default)]
    pub insteek_uit: f64,
    /// Paal tot zoveel boven het hoogste punt van de binnenboom; standaard 50.
    #[serde(default = "standaard_paal_boven")]
    pub boven_boom: f64,
    #[serde(default = "standaard_materiaal")]
    pub materiaal: String,
}

impl Default for TrappaalInvoer {
    fn default() -> Self {
        Self {
            langs_in: standaard_paal(),
            langs_uit: standaard_paal(),
            insteek_in: 0.0,
            insteek_uit: 0.0,
            boven_boom: standaard_paal_boven(),
            materiaal: standaard_materiaal(),
        }
    }
}

fn standaard_paal() -> f64 {
    90.0
}
fn standaard_paal_boven() -> f64 {
    50.0
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerdrijvingInvoer {
    #[serde(default)]
    pub methode: Verdrijfmethode,
    /// Verdrijfvakken (voorkant `van` en `tot` blijven onverdreven). Weg =
    /// automatisch: per binnenhoek de tusschenkwart met de grootste kleinste
    /// aantrede aan de binnenboom.
    #[serde(default)]
    pub vakken: Option<Vec<Vak>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Verdrijfmethode {
    /// Rekenkundige reeks langs de binnenboom (Wattjes, fuikmethode).
    #[default]
    Fuik,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Vak {
    pub van: usize,
    pub tot: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LooplijnInvoer {
    /// Afstand tot de buitenzijde van de loopbreedte; standaard 300.
    #[serde(default = "standaard_looplijn")]
    pub afstand_buitenzijde: f64,
    /// 'boog' (standaard) of 'scherp' in de hoeken.
    #[serde(default)]
    pub hoek: Looplijnhoek,
    /// Afrondingsstraal rho van de hoek van de offsetlijn (de boog raakt beide
    /// armen); weg = afstand van de binnenhoek tot de looplijn (boog uit de
    /// binnenhoek, rho = breedte - afstand bij een kwart); 0 = scherp.
    #[serde(default)]
    pub straal: Option<f64>,
}

impl Default for LooplijnInvoer {
    fn default() -> Self {
        Self {
            afstand_buitenzijde: standaard_looplijn(),
            hoek: Looplijnhoek::Boog,
            straal: None,
        }
    }
}

fn standaard_looplijn() -> f64 {
    300.0
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Looplijnhoek {
    #[default]
    Boog,
    Scherp,
}

/// Koude (stompe) hoekverbinding van de buitenboom.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HoekverbindingInvoer {
    /// Welk boomdeel doorloopt door het hoekvierkant: 'hoger' (standaard,
    /// Wattjes: uitkeping in de hogere boom, de lagere sluit er met een vlakke
    /// borst op aan) of 'lager'.
    #[serde(default)]
    pub doorlopend: Doorlopend,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Doorlopend {
    #[default]
    Hoger,
    Lager,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClearanceInvoer {
    /// Vrije hoogte boven het neuslijnvlak; weg = Bbl-minimum voor de
    /// gebruiksfunctie (tabel 4.26).
    #[serde(default)]
    pub vrije_hoogte: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Gebruiksfunctie {
    #[default]
    Woonfunctie,
    Bijeenkomstfunctie,
    Celfunctie,
    Gezondheidszorgfunctie,
    Industriefunctie,
    Kantoorfunctie,
    Logiesfunctie,
    Onderwijsfunctie,
    Sportfunctie,
    Winkelfunctie,
    Overig,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Bouwsoort {
    #[default]
    Nieuwbouw,
    /// Verbouw: Bbl art. 5.4/5.5, rechtens verkregen niveau. Een
    /// overschrijding van het nieuwbouwniveau wordt gemeld, niet afgekeurd.
    Verbouw,
}

/// JSON-schema van de invoer.
pub fn schema() -> String {
    let s = schemars::schema_for!(Invoer);
    serde_json::to_string_pretty(&s).expect("schema serialiseren") + "\n"
}
