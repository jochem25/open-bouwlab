//! Invoer- en uitvoermodel van de constructie-rekenkern.
//!
//! Eenheden: m voor overspanning en belastingbreedte, kN/m2 voor vlaklasten,
//! mm voor doorsnedematen.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{ConstructieFout, Result};
use crate::toets::{Melding, Toets};

fn waar() -> bool {
    true
}
fn hellingshoek_default() -> f64 {
    0.0
}
fn afschot_default() -> f64 {
    1.6
}
fn element_ligger() -> Element {
    Element::Ligger
}
fn element_balk() -> Element {
    Element::Balk
}
fn phi_hoofd_default() -> f64 {
    20.0
}
fn phi_beugel_default() -> f64 {
    8.0
}
fn korrel_default() -> f64 {
    16.0
}

/// Materiaal van het element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Materiaal {
    /// Hout: nog niet beschikbaar (deel 2).
    Hout,
    /// Gewalste staalprofielen.
    Staal,
    /// Gewapend beton, rechthoekige balk.
    Beton,
}

/// Type element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    /// Houten balklaag.
    Balklaag,
    /// Stalen ligger.
    Ligger,
    /// Betonbalk.
    Balk,
}

/// Toepassing van het veld.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Toepassing {
    /// Vloer.
    #[default]
    Vloer,
    /// Dak.
    Dak,
}

/// Gebruiksfunctie van de vloer (bepaalt categorie en veranderlijke belasting).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Gebruiksfunctie {
    /// Categorie A, woonfunctie, vloer.
    #[default]
    WoonVloer,
    /// Categorie A, woonfunctie, trap.
    WoonTrap,
    /// Categorie A, woonfunctie, balkon.
    WoonBalkon,
    /// Categorie A, gemeenschappelijke ruimte (vloer, trap, balkon).
    Gemeenschappelijk,
    /// Categorie B, kantoor.
    Kantoor,
}

/// Gebouwtype (bepaalt de minimale gevolgklasse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Gebouwtype {
    /// Eengezinswoning, 1 tot 3 bouwlagen.
    #[default]
    Eengezinswoning1Tot3,
    /// Eengezinswoning, 4 of meer bouwlagen.
    Eengezinswoning4Plus,
    /// Woongebouw.
    Woongebouw,
    /// Kantoorgebouw.
    Kantoorgebouw,
}

/// Gevolgklasse.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum Gevolgklasse {
    /// Gevolgklasse 1.
    CC1,
    /// Gevolgklasse 2.
    CC2,
    /// Gevolgklasse 3 (buiten scope v1).
    CC3,
}

/// Klasse lichte scheidingswanden (gewicht per strekkende meter wand).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Scheidingswanden {
    /// Geen lichte scheidingswanden.
    #[default]
    Geen,
    /// Wandgewicht tot 1,0 kN/m.
    TotEen,
    /// Wandgewicht tot 2,0 kN/m.
    TotTwee,
    /// Wandgewicht tot 3,0 kN/m.
    TotDrie,
}

/// Invoer die voor alle materialen geldt.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Algemeen {
    /// Toepassing: vloer of dak.
    #[serde(default)]
    pub toepassing: Toepassing,
    /// Gebruiksfunctie (bij dak alleen van belang als het dak beloopbaar is).
    #[serde(default)]
    pub gebruiksfunctie: Gebruiksfunctie,
    /// Gebouwtype; bepaalt de minimale gevolgklasse.
    #[serde(default)]
    pub gebouwtype: Gebouwtype,
    /// Gevolgklasse; `None` volgt het gebouwtype. Lager dan de default is niet toegestaan.
    #[serde(default)]
    pub gevolgklasse: Option<Gevolgklasse>,
    /// Overspanning l in m (1,0 - 12,0).
    pub overspanning_m: f64,
    /// Permanente belasting g_k in kN/m2, zonder eigen gewicht van het element (0 - 15).
    pub permanent_kn_m2: f64,
    /// Eigen gewicht van het element automatisch meenemen.
    #[serde(default = "waar")]
    pub eigen_gewicht_automatisch: bool,
    /// Klasse lichte scheidingswanden.
    #[serde(default)]
    pub lichte_scheidingswanden: Scheidingswanden,
    /// Scheurgevoelige scheidingswanden aanwezig.
    #[serde(default)]
    pub scheurgevoelige_scheidingswanden: bool,
    /// Uiterlijk van belang (toets op w_max).
    #[serde(default = "waar")]
    pub uiterlijk_van_belang: bool,
    /// Dak: beloopbaar (belasting als vloer).
    #[serde(default)]
    pub dak_beloopbaar: bool,
    /// Dak: hellingshoek in graden (0 - 60).
    #[serde(default = "hellingshoek_default")]
    pub dak_hellingshoek_graden: f64,
    /// Dak: afschot in procent (0 - 100).
    #[serde(default = "afschot_default")]
    pub dak_afschot_procent: f64,
}

impl Algemeen {
    /// Controleer bereiken en veldcombinaties.
    pub fn valideer(&self) -> Result<()> {
        bereik("overspanning_m", self.overspanning_m, 1.0, 12.0)?;
        bereik("permanent_kn_m2", self.permanent_kn_m2, 0.0, 15.0)?;
        bereik(
            "dak_hellingshoek_graden",
            self.dak_hellingshoek_graden,
            0.0,
            60.0,
        )?;
        bereik("dak_afschot_procent", self.dak_afschot_procent, 0.0, 100.0)?;
        if let Some(gk) = self.gevolgklasse {
            let minimum = crate::belasting::minimale_gevolgklasse(self.gebouwtype);
            if gk < minimum {
                return Err(ConstructieFout::Invoer(format!(
                    "gevolgklasse {gk:?} is lager dan de minimale gevolgklasse {minimum:?} voor dit gebouwtype"
                )));
            }
        }
        Ok(())
    }
}

/// Controleer dat `waarde` eindig is en tussen `min` en `max` ligt.
pub(crate) fn bereik(naam: &str, waarde: f64, min: f64, max: f64) -> Result<()> {
    if !waarde.is_finite() || waarde < min || waarde > max {
        return Err(ConstructieFout::Invoer(format!(
            "{naam} = {waarde} valt buiten het bereik {min} - {max}"
        )));
    }
    Ok(())
}

/// Profielreeks staal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Reeks {
    /// IPE-profielen.
    #[serde(rename = "IPE")]
    Ipe,
    /// HEA-profielen.
    #[serde(rename = "HEA")]
    Hea,
    /// HEB-profielen.
    #[serde(rename = "HEB")]
    Heb,
}

impl Reeks {
    /// Naam van de reeks.
    pub fn naam(self) -> &'static str {
        match self {
            Reeks::Ipe => "IPE",
            Reeks::Hea => "HEA",
            Reeks::Heb => "HEB",
        }
    }
}

/// Staalsoort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Staalsoort {
    /// S235.
    S235,
    /// S275.
    S275,
    /// S355.
    S355,
}

/// Invoer voor een stalen ligger.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StaalInvoer {
    /// Element (staal: alleen ligger).
    #[serde(default = "element_ligger")]
    pub element: Element,
    /// Gemeenschappelijke invoer.
    pub algemeen: Algemeen,
    /// Belastingbreedte in m.
    pub belastingbreedte_m: f64,
    /// Staalsoort.
    pub staalsoort: Staalsoort,
    /// Te beschouwen profielreeksen.
    pub reeksen: Vec<Reeks>,
    /// Bovenflens doorgaand zijdelings gesteund; `false` is buiten scope v1.
    #[serde(default = "waar")]
    pub bovenflens_gesteund: bool,
}

impl StaalInvoer {
    /// Controleer de invoer.
    pub fn valideer(&self) -> Result<()> {
        self.algemeen.valideer()?;
        bereik("belastingbreedte_m", self.belastingbreedte_m, 0.1, 30.0)?;
        if self.element != Element::Ligger {
            return Err(ConstructieFout::Invoer(
                "element voor staal moet 'ligger' zijn".into(),
            ));
        }
        if self.reeksen.is_empty() {
            return Err(ConstructieFout::Invoer(
                "kies minimaal een profielreeks".into(),
            ));
        }
        Ok(())
    }
}

/// Sterkteklasse beton.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Sterkteklasse {
    /// C20/25.
    #[serde(rename = "C20/25")]
    C20_25,
    /// C25/30.
    #[serde(rename = "C25/30")]
    C25_30,
    /// C30/37.
    #[serde(rename = "C30/37")]
    C30_37,
    /// C35/45.
    #[serde(rename = "C35/45")]
    C35_45,
}

/// Milieuklasse (carbonatatie).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub enum Milieuklasse {
    /// XC1 (droog of permanent nat).
    #[default]
    #[serde(rename = "XC1")]
    Xc1,
    /// XC3 (matige vochtigheid).
    #[serde(rename = "XC3")]
    Xc3,
}

/// Invoer voor een betonbalk.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BetonInvoer {
    /// Element (beton: alleen balk).
    #[serde(default = "element_balk")]
    pub element: Element,
    /// Gemeenschappelijke invoer.
    pub algemeen: Algemeen,
    /// Belastingbreedte in m.
    pub belastingbreedte_m: f64,
    /// Sterkteklasse.
    pub sterkteklasse: Sterkteklasse,
    /// Balkbreedte in mm.
    pub balkbreedte_mm: f64,
    /// Hoogte automatisch bepalen (stappen van 50 mm).
    #[serde(default = "waar")]
    pub hoogte_automatisch: bool,
    /// Vaste hoogte in mm, verplicht als `hoogte_automatisch` onwaar is.
    #[serde(default)]
    pub hoogte_mm: Option<f64>,
    /// Milieuklasse.
    #[serde(default)]
    pub milieuklasse: Milieuklasse,
    /// Aangenomen hoofdstaafdiameter in mm.
    #[serde(default = "phi_hoofd_default")]
    pub phi_hoofd_mm: f64,
    /// Beugeldiameter in mm.
    #[serde(default = "phi_beugel_default")]
    pub phi_beugel_mm: f64,
    /// Grootste korrelafmeting in mm.
    #[serde(default = "korrel_default")]
    pub d_g_mm: f64,
}

impl BetonInvoer {
    /// Controleer de invoer.
    pub fn valideer(&self) -> Result<()> {
        self.algemeen.valideer()?;
        bereik("belastingbreedte_m", self.belastingbreedte_m, 0.1, 30.0)?;
        bereik("balkbreedte_mm", self.balkbreedte_mm, 100.0, 2000.0)?;
        bereik("phi_hoofd_mm", self.phi_hoofd_mm, 6.0, 40.0)?;
        bereik("phi_beugel_mm", self.phi_beugel_mm, 6.0, 20.0)?;
        bereik("d_g_mm", self.d_g_mm, 4.0, 40.0)?;
        if self.element != Element::Balk {
            return Err(ConstructieFout::Invoer(
                "element voor beton moet 'balk' zijn".into(),
            ));
        }
        if !self.hoogte_automatisch {
            match self.hoogte_mm {
                Some(h) => bereik("hoogte_mm", h, 100.0, 3000.0)?,
                None => {
                    return Err(ConstructieFout::Invoer(
                        "hoogte_mm is verplicht als hoogte_automatisch onwaar is".into(),
                    ))
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Uitvoer
// ---------------------------------------------------------------------------

/// Gekozen wapening en beugels (beton).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BetonKeuze {
    /// Aantal hoofdstaven in een laag.
    pub aantal: u32,
    /// Staafdiameter in mm.
    pub diameter_mm: f64,
    /// Aanwezig staaloppervlak in mm2.
    pub a_s_prov_mm2: f64,
    /// Beugeldiameter in mm.
    pub beugel_diameter_mm: f64,
    /// Beugelafstand in mm (naar beneden afgerond op 10 mm).
    pub beugel_s_mm: f64,
    /// Gekozen cot(theta) voor de dwarskrachtwapening.
    pub cot_theta: f64,
}

/// Een doorgerekende kandidaat (profiel of betonhoogte) met al zijn toetsen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Kandidaat {
    /// Naam (profielnaam of "300 x 450").
    pub naam: String,
    /// Constructiehoogte in mm.
    pub hoogte_mm: f64,
    /// Eigen gewicht in kN/m.
    pub eigen_gewicht_kn_m: f64,
    /// Massa in kg/m (alleen staal).
    pub gewicht_kg_m: Option<f64>,
    /// Alle toetsen.
    pub toetsen: Vec<Toets>,
    /// Meldingen die bij deze kandidaat horen.
    pub meldingen: Vec<Melding>,
    /// Reden waarom de kandidaat is afgewezen (bijv. wapening past niet).
    pub afgewezen: Option<String>,
    /// Tussenwaarden (q_d, M_Ed, ...) op de onafgeronde waarde.
    pub tussenwaarden: BTreeMap<String, f64>,
    /// Gekozen wapening (alleen beton).
    pub beton: Option<BetonKeuze>,
}

impl Kandidaat {
    /// Hoogste UC over alle toetsen met een UC.
    pub fn max_uc(&self) -> Option<f64> {
        self.toetsen
            .iter()
            .filter_map(|t| t.uc)
            .fold(None, |m, u| Some(m.map_or(u, |m: f64| m.max(u))))
    }

    /// `true` als alle UC <= 1,00 en geen toets het advies blokkeert.
    pub fn voldoet(&self) -> bool {
        self.afgewezen.is_none() && !self.toetsen.iter().any(Toets::blokkeert)
    }
}

/// Toets met de hoogste UC en een korte uitleg.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Maatgevend {
    /// Sleutel van de toets.
    pub toets_id: String,
    /// Naam van de toets.
    pub naam: String,
    /// Unity check (onafgerond).
    pub uc: f64,
    /// Korte uitleg.
    pub uitleg: String,
}

/// Kengetallen van een gekozen kandidaat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Kengetallen {
    /// Constructiehoogte in mm.
    pub constructiehoogte_mm: f64,
    /// Verhouding overspanning / hoogte.
    pub l_h: f64,
    /// Eigen gewicht in kN/m.
    pub eigen_gewicht_kn_m: f64,
    /// Massa in kg/m (alleen staal).
    pub gewicht_kg_m: Option<f64>,
    /// Eigenfrequentie in Hz, alleen indien berekend (in v1 niet).
    pub eigenfrequentie_hz: Option<f64>,
}

/// Een gekozen kandidaat (advies of laagste bouwhoogte).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Advies {
    /// Naam van de kandidaat.
    pub naam: String,
    /// Constructiehoogte in mm.
    pub hoogte_mm: f64,
    /// Maatgevende toets.
    pub maatgevend: Option<Maatgevend>,
    /// Kengetallen.
    pub kengetallen: Kengetallen,
    /// Alle toetsen van deze kandidaat.
    pub toetsen: Vec<Toets>,
    /// Gekozen wapening (alleen beton).
    pub beton: Option<BetonKeuze>,
}

/// Positie van een alternatief ten opzichte van het advies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlternatiefPositie {
    /// Direct onder het advies (laat zien waarom het niet kan).
    Onder,
    /// Boven het advies.
    Boven,
}

/// Alternatief naast het advies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Alternatief {
    /// Naam van de kandidaat.
    pub naam: String,
    /// Positie ten opzichte van het advies.
    pub positie: AlternatiefPositie,
    /// Voldoet de kandidaat?
    pub voldoet: bool,
    /// Hoogste UC.
    pub max_uc: Option<f64>,
    /// Naam van de maatgevende toets.
    pub maatgevend: Option<String>,
    /// Toelichting.
    pub toelichting: String,
}

/// Resultaat van een voordimensionering (staal of beton).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Resultaat {
    /// Vaste tekst E1; altijd aanwezig.
    pub disclaimer: String,
    /// Versie van de rekenkern.
    pub kern_versie: String,
    /// Gebruikte normedities (sleutel, editie).
    pub normedities: Vec<(String, String)>,
    /// Toegepaste gevolgklasse.
    pub gevolgklasse: Gevolgklasse,
    /// Alle doorgerekende kandidaten, oplopend.
    pub kandidaten: Vec<Kandidaat>,
    /// Advies; `None` als er binnen bereik geen kandidaat voldoet.
    pub advies: Option<Advies>,
    /// Laagste bouwhoogte (alleen staal).
    pub laagste_bouwhoogte: Option<Advies>,
    /// Alternatieven rond het advies.
    pub alternatieven: Vec<Alternatief>,
    /// Meldingen (waarschuwingen, signaleringen, info).
    pub meldingen: Vec<Melding>,
}

/// Resultaat voor staal.
pub type StaalResultaat = Resultaat;
/// Resultaat voor beton.
pub type BetonResultaat = Resultaat;
