//! Toetsen, bronverwijzingen en meldingen.
//!
//! Elke toets draagt zijn bronverwijzing (norm, artikel, NB apart) en een status.
//! Een toets die niet kan worden uitgevoerd verschijnt als `NietGetoetst`, nooit
//! als `Voldoet`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Verwijzing naar een norm: alleen sleutel en artikel, geen normtekst.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Bron {
    /// Normsleutel, zoals in [`crate::NORMEDITIES`] (bijv. "EC3").
    pub norm: String,
    /// Artikel of tabel (bijv. "6.2.5 (6.13)").
    pub artikel: String,
    /// `true` als de waarde of regel uit de nationale bijlage komt.
    pub nb: bool,
}

impl Bron {
    /// Verwijzing naar de basistekst van een norm.
    pub fn basis(norm: &str, artikel: &str) -> Self {
        Self {
            norm: norm.to_string(),
            artikel: artikel.to_string(),
            nb: false,
        }
    }

    /// Verwijzing naar de nationale bijlage van een norm.
    pub fn nb(norm: &str, artikel: &str) -> Self {
        Self {
            norm: norm.to_string(),
            artikel: artikel.to_string(),
            nb: true,
        }
    }

    /// Leesbare tekst, bijv. "EC2 NB 6.1(9)".
    pub fn tekst(&self) -> String {
        if self.nb {
            format!("{} NB {}", self.norm, self.artikel)
        } else {
            format!("{} {}", self.norm, self.artikel)
        }
    }
}

/// Uitkomst van een toets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ToetsStatus {
    /// Alle eisen vervuld.
    Voldoet,
    /// Eis overschreden.
    VoldoetNiet,
    /// De tool kan deze toets niet uitvoeren.
    NietGetoetst {
        /// Waarom de toets niet is uitgevoerd.
        reden: String,
        /// `true` als er door deze toets geen advies mag worden gegeven.
        blokkeert_advies: bool,
    },
    /// De toets rust op een aanname die de gebruiker moet kennen.
    Aanname {
        /// Beschrijving van de aanname.
        tekst: String,
    },
}

/// Een afzonderlijke toets met waarde, grens, unity check en bron.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Toets {
    /// Stabiele sleutel (bijv. "buiging").
    pub id: String,
    /// Weergavenaam.
    pub naam: String,
    /// Korte eigen formulenotatie (bijv. "M_Ed <= M_pl,Rd").
    pub formule: String,
    /// Optredende waarde (belasting-, doorbuigings- of verhoudingswaarde).
    pub waarde: Option<f64>,
    /// Grenswaarde (capaciteit of limiet).
    pub grens: Option<f64>,
    /// Eenheid van waarde en grens.
    pub eenheid: String,
    /// Unity check (onafgerond); `None` als de toets geen verhouding kent.
    pub uc: Option<f64>,
    /// Bronverwijzing.
    pub bron: Bron,
    /// Status.
    pub status: ToetsStatus,
}

impl Toets {
    /// Nieuwe toets met status `Voldoet` en zonder waarden.
    pub fn nieuw(id: &str, naam: &str, formule: &str, bron: Bron) -> Self {
        Self {
            id: id.to_string(),
            naam: naam.to_string(),
            formule: formule.to_string(),
            waarde: None,
            grens: None,
            eenheid: String::new(),
            uc: None,
            bron,
            status: ToetsStatus::Voldoet,
        }
    }

    /// Zet waarde, grens en eenheid.
    pub fn waarden(mut self, waarde: f64, grens: f64, eenheid: &str) -> Self {
        self.waarde = Some(waarde);
        self.grens = Some(grens);
        self.eenheid = eenheid.to_string();
        self
    }

    /// Zet de unity check en leidt de status af (UC <= 1,00 op de onafgeronde waarde).
    pub fn met_uc(mut self, uc: f64) -> Self {
        self.uc = Some(uc);
        self.status = if uc <= 1.0 {
            ToetsStatus::Voldoet
        } else {
            ToetsStatus::VoldoetNiet
        };
        self
    }

    /// Bepaalt UC = waarde / grens uit de ingestelde waarden.
    pub fn uc_uit_waarden(self) -> Self {
        let uc = match (self.waarde, self.grens) {
            (Some(w), Some(g)) if g > 0.0 => w / g,
            _ => f64::MAX,
        };
        self.met_uc(uc.min(UC_PLAFOND))
    }

    /// Zet de status expliciet.
    pub fn status(mut self, status: ToetsStatus) -> Self {
        self.status = status;
        self
    }

    /// Markeer als niet getoetst.
    pub fn niet_getoetst(self, reden: &str, blokkeert_advies: bool) -> Self {
        self.status(ToetsStatus::NietGetoetst {
            reden: reden.to_string(),
            blokkeert_advies,
        })
    }

    /// Markeer als aanname.
    pub fn aanname(self, tekst: &str) -> Self {
        self.status(ToetsStatus::Aanname {
            tekst: tekst.to_string(),
        })
    }

    /// `true` als de toets het advies blokkeert (overschrijding of blokkerend niet getoetst).
    pub fn blokkeert(&self) -> bool {
        match &self.status {
            ToetsStatus::VoldoetNiet => true,
            ToetsStatus::NietGetoetst {
                blokkeert_advies, ..
            } => *blokkeert_advies,
            ToetsStatus::Voldoet | ToetsStatus::Aanname { .. } => false,
        }
    }
}

/// Bovengrens voor UC-weergave, zodat de JSON eindig blijft.
pub const UC_PLAFOND: f64 = 99.0;

/// Soort melding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MeldingSoort {
    /// Aandachtspunt dat het resultaat beinvloedt.
    Waarschuwing,
    /// Signalering van iets dat buiten de toetsing valt.
    Signalering,
    /// Informatie.
    Info,
}

/// Een melding bij het resultaat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Melding {
    /// Soort.
    pub soort: MeldingSoort,
    /// Tekst.
    pub tekst: String,
    /// Optionele bron.
    pub bron: Option<Bron>,
}

impl Melding {
    /// Waarschuwing zonder bron.
    pub fn waarschuwing(tekst: &str) -> Self {
        Self::nieuw(MeldingSoort::Waarschuwing, tekst, None)
    }

    /// Signalering zonder bron.
    pub fn signalering(tekst: &str) -> Self {
        Self::nieuw(MeldingSoort::Signalering, tekst, None)
    }

    /// Info zonder bron.
    pub fn info(tekst: &str) -> Self {
        Self::nieuw(MeldingSoort::Info, tekst, None)
    }

    /// Melding met soort en optionele bron.
    pub fn nieuw(soort: MeldingSoort, tekst: &str, bron: Option<Bron>) -> Self {
        Self {
            soort,
            tekst: tekst.to_string(),
            bron,
        }
    }

    /// Voeg een bron toe.
    pub fn met_bron(mut self, bron: Bron) -> Self {
        self.bron = Some(bron);
        self
    }
}

/// Rond af op `decimalen`, half naar boven, op de onafgeronde waarde.
pub fn rond_half_op(x: f64, decimalen: u32) -> f64 {
    let f = 10f64.powi(decimalen as i32);
    let afgerond = (x.abs() * f + 1e-9).round() / f;
    if x < 0.0 {
        -afgerond
    } else {
        afgerond
    }
}

/// Formatteer een getal met `decimalen` decimalen en Nederlandse komma (half naar boven).
pub fn fmt_getal(x: f64, decimalen: u32) -> String {
    let r = rond_half_op(x, decimalen);
    format!("{:.*}", decimalen as usize, r).replace('.', ",")
}

/// Formatteer een unity check: 2 decimalen, half naar boven, Nederlandse komma.
///
/// Alleen voor weergave; beslissingen gebeuren op de onafgeronde waarde.
pub fn fmt_uc(uc: f64) -> String {
    fmt_getal(uc, 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_uc_rondt_half_naar_boven() {
        assert_eq!(fmt_uc(0.845), "0,85");
        assert_eq!(fmt_uc(0.844), "0,84");
        assert_eq!(fmt_uc(1.0), "1,00");
        assert_eq!(fmt_getal(12.5, 0), "13");
    }

    #[test]
    fn uc_bepaalt_status_op_onafgeronde_waarde() {
        let t = Toets::nieuw("x", "x", "a <= b", Bron::basis("EC3", "6.2.5"))
            .waarden(1.004, 1.0, "-")
            .uc_uit_waarden();
        // afgerond 1,00 maar onafgerond > 1: voldoet niet.
        assert_eq!(fmt_uc(t.uc.unwrap()), "1,00");
        assert!(matches!(t.status, ToetsStatus::VoldoetNiet));
    }

    #[test]
    fn niet_getoetst_kan_blokkeren() {
        let t =
            Toets::nieuw("x", "x", "-", Bron::basis("EC3", "6.2.8")).niet_getoetst("reden", true);
        assert!(t.blokkeert());
        let t =
            Toets::nieuw("x", "x", "-", Bron::basis("EC3", "6.2.8")).niet_getoetst("reden", false);
        assert!(!t.blokkeert());
    }
}
