//! Belastingen en combinaties, voor alle materialen.
//!
//! Combinaties: UGT (STR) met beide combinaties 6.10a en 6.10b, de ongunstigste
//! is maatgevend. Veranderlijke belastingen per categorie, sneeuw, doorbuigings-
//! grenzen en het massacriterium voor trilling.

use crate::error::Result;
use crate::model::{
    Algemeen, Gebouwtype, Gebruiksfunctie, Gevolgklasse, Scheidingswanden, Toepassing,
};
use crate::toets::{Bron, Melding};

/// Volumieke massa (gewicht) gewapend beton in kN/m3 (EC1-1 tabel A.1).
pub const GEWICHT_BETON: f64 = 25.0;
/// Volumieke massa (gewicht) staal in kN/m3 (EC1-1 tabel A.4).
pub const GEWICHT_STAAL: f64 = 78.5;
/// Karakteristieke sneeuwbelasting op de grond s_k in kN/m2 (EC1-3 NB 4.1(1)).
pub const SNEEUW_SK: f64 = 0.7;
/// Vormfactor mu_1 voor een plat dak. // O7: basistekstwaarde, NB-waarde niet gevonden.
pub const SNEEUW_MU1_0: f64 = 0.8;
/// Puntlast Q_k voor dakbalken in kN (EC1-1 NB bij NB.4-6.10).
pub const DAK_PUNTLAST_KN: f64 = 2.0;

/// Minimale gevolgklasse volgens het gebouwtype (EC0 NB tabel NB.24-B1).
pub fn minimale_gevolgklasse(gebouwtype: Gebouwtype) -> Gevolgklasse {
    match gebouwtype {
        Gebouwtype::Eengezinswoning1Tot3 => Gevolgklasse::CC1,
        Gebouwtype::Eengezinswoning4Plus | Gebouwtype::Woongebouw | Gebouwtype::Kantoorgebouw => {
            Gevolgklasse::CC2
        }
    }
}

/// Partiele factoren voor de combinaties 6.10a en 6.10b.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Combinatie {
    /// gamma_G in 6.10a.
    pub gamma_g_a: f64,
    /// gamma_G in 6.10b.
    pub gamma_g_b: f64,
    /// gamma_Q (in beide combinaties).
    pub gamma_q: f64,
    /// gamma_G,inf (gunstig werkende permanente belasting).
    pub gamma_g_inf: f64,
}

/// Factoren per gevolgklasse (EC0 NB A1.3.1(1), tabel NB.4-A1.2(B)).
pub fn combinatie(cc: Gevolgklasse) -> Combinatie {
    match cc {
        Gevolgklasse::CC1 => Combinatie {
            gamma_g_a: 1.2,
            gamma_g_b: 1.1,
            gamma_q: 1.35,
            gamma_g_inf: 0.9,
        },
        Gevolgklasse::CC2 => Combinatie {
            gamma_g_a: 1.35,
            gamma_g_b: 1.2,
            gamma_q: 1.5,
            gamma_g_inf: 0.9,
        },
        Gevolgklasse::CC3 => Combinatie {
            gamma_g_a: 1.5,
            gamma_g_b: 1.3,
            gamma_q: 1.65,
            gamma_g_inf: 0.9,
        },
    }
}

/// Welke veranderlijke belasting leidend is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leidend {
    /// Vloerbelasting volgens de gebruiksfunctie.
    Vloer,
    /// Dakbelasting categorie H.
    DakH,
    /// Sneeuw.
    Sneeuw,
}

/// Veranderlijke belasting van een vloer volgens categorie: (q_k, Q_k, psi0, psi1, psi2, label).
/// Bron: EC1-1 NB tabel NB.1-6.2; psi: EC0 NB tabel NB.2-A1.1.
pub fn vloer_categorie(gf: Gebruiksfunctie) -> (f64, f64, [f64; 3], &'static str) {
    match gf {
        Gebruiksfunctie::WoonVloer => (1.75, 3.0, [0.4, 0.5, 0.3], "A woonfunctie, vloer"),
        Gebruiksfunctie::WoonTrap => (2.0, 3.0, [0.4, 0.5, 0.3], "A woonfunctie, trap"),
        Gebruiksfunctie::WoonBalkon => (2.5, 3.0, [0.4, 0.5, 0.3], "A woonfunctie, balkon"),
        Gebruiksfunctie::Gemeenschappelijk => (3.0, 3.0, [0.4, 0.5, 0.3], "A gemeenschappelijk"),
        Gebruiksfunctie::Kantoor => (2.5, 3.0, [0.5, 0.5, 0.3], "B kantoor"),
    }
}

/// Gelijkmatige belasting in kN/m2 voor lichte scheidingswanden (EC1-1 6.3.1.2(8),(9)).
pub fn wandbelasting(w: Scheidingswanden) -> f64 {
    match w {
        Scheidingswanden::Geen => 0.0,
        Scheidingswanden::TotEen => 0.5,
        Scheidingswanden::TotTwee => 0.8,
        Scheidingswanden::TotDrie => 1.2,
    }
}

/// Dak categorie H: q_k in kN/m2 bij een hellingshoek in graden (EC1-1 NB.4-6.10).
pub fn dak_h_q_k(helling_graden: f64) -> f64 {
    if helling_graden < 15.0 {
        1.0
    } else if helling_graden < 20.0 {
        4.0 - 0.2 * helling_graden
    } else {
        0.0
    }
}

/// Sneeuwbelasting op het dak in kN/m2: s = mu_1 * C_e * C_t * s_k (EC1-3 5.2, 5.3.2).
pub fn sneeuwbelasting(helling_graden: f64) -> f64 {
    let mu1 = if helling_graden <= 30.0 {
        SNEEUW_MU1_0
    } else if helling_graden < 60.0 {
        SNEEUW_MU1_0 * (60.0 - helling_graden) / 30.0
    } else {
        0.0
    };
    // C_e = 1,0 en C_t = 1,0 (EC1-3 NB 5.2(7), 5.2(8)).
    mu1 * 1.0 * 1.0 * SNEEUW_SK
}

/// Belastingen en factoren voor een berekening.
#[derive(Debug, Clone)]
pub struct Belasting {
    /// Toegepaste gevolgklasse.
    pub gevolgklasse: Gevolgklasse,
    /// Combinatiefactoren.
    pub combinatie: Combinatie,
    /// Leidende veranderlijke belasting.
    pub leidend: Leidend,
    /// Label van de leidende belasting.
    pub label: String,
    /// Leidende gelijkmatige veranderlijke belasting in kN/m2 (inclusief lichte wanden).
    pub q_k: f64,
    /// Puntlast Q_k in kN (afzonderlijk, niet gecombineerd met q_k).
    pub q_punt_kn: f64,
    /// psi_0.
    pub psi0: f64,
    /// psi_1.
    pub psi1: f64,
    /// psi_2.
    pub psi2: f64,
    /// Permanente belasting g_k in kN/m2.
    pub g_k: f64,
    /// Overspanning in m.
    pub l_m: f64,
    /// Toepassing.
    pub toepassing: Toepassing,
    /// Beloopbaar dak.
    pub beloopbaar: bool,
    /// Scheurgevoelige scheidingswanden.
    pub scheurgevoelig: bool,
    /// Afschot in procent (dak).
    pub afschot_procent: f64,
    /// Meldingen die uit de belastingbepaling volgen.
    pub meldingen: Vec<Melding>,
    /// Bron van de leidende veranderlijke belasting.
    pub bron_q: Bron,
}

/// Resultaat van de UGT-combinaties voor een vrij opgelegde ligger.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ugt {
    /// q_d volgens 6.10a in kN/m.
    pub q_d_a: f64,
    /// q_d volgens 6.10b in kN/m.
    pub q_d_b: f64,
    /// Maatgevende q_d in kN/m.
    pub q_d: f64,
    /// Moment uit de gelijkmatige belasting in kNm.
    pub m_gelijkmatig: f64,
    /// Moment uit de puntlast Q_k in het midden (ongunstigste combinatie) in kNm.
    pub m_punt: f64,
    /// Maatgevend moment in kNm.
    pub m_ed: f64,
    /// Dwarskracht bij de oplegging uit de gelijkmatige belasting in kN.
    pub v_gelijkmatig: f64,
    /// Dwarskracht bij de oplegging met de puntlast Q_k op de oplegging (ongunstigste
    /// combinatie) in kN.
    pub v_punt: f64,
    /// Maatgevende dwarskracht bij de oplegging in kN.
    pub v_ed: f64,
}

/// Doorbuigingseis volgens de gekozen situatie.
#[derive(Debug, Clone)]
pub struct DoorbuigingsEis {
    /// `true`: w_3 = psi_1 * w_Q (frequent); `false`: w_3 = w_Q (karakteristiek).
    pub w3_met_psi1: bool,
    /// Grens voor w_2 + w_3 in mm.
    pub grens_w23_mm: f64,
    /// Formule voor de toets op w_2 + w_3.
    pub formule_w23: &'static str,
    /// Bron van de grens.
    pub bron_w23: Bron,
    /// Grens voor w_max in mm.
    pub grens_wmax_mm: f64,
}

impl Belasting {
    /// `true` als de constructie zich als vloer gedraagt (vloer of beloopbaar dak).
    pub fn vloergedrag(&self) -> bool {
        self.toepassing == Toepassing::Vloer || self.beloopbaar
    }

    /// UGT-combinaties voor permanente `g_lijn` en veranderlijke `q_lijn` in kN/m.
    pub fn ugt(&self, g_lijn: f64, q_lijn: f64) -> Ugt {
        let c = &self.combinatie;
        let l = self.l_m;
        let q_d_a = c.gamma_g_a * g_lijn + c.gamma_q * self.psi0 * q_lijn;
        let q_d_b = c.gamma_g_b * g_lijn + c.gamma_q * q_lijn;
        let q_d = q_d_a.max(q_d_b);
        let m_gelijkmatig = q_d * l * l / 8.0;
        let m_a =
            c.gamma_g_a * g_lijn * l * l / 8.0 + c.gamma_q * self.psi0 * self.q_punt_kn * l / 4.0;
        let m_b = c.gamma_g_b * g_lijn * l * l / 8.0 + c.gamma_q * self.q_punt_kn * l / 4.0;
        let m_punt = m_a.max(m_b);
        let v_gelijkmatig = q_d * l / 2.0;
        // Puntlast Q_k afzonderlijk van q_k, net als bij het moment; bij de oplegging.
        let v_a = c.gamma_g_a * g_lijn * l / 2.0 + c.gamma_q * self.psi0 * self.q_punt_kn;
        let v_b = c.gamma_g_b * g_lijn * l / 2.0 + c.gamma_q * self.q_punt_kn;
        let v_punt = v_a.max(v_b);
        Ugt {
            q_d_a,
            q_d_b,
            q_d,
            m_gelijkmatig,
            m_punt,
            m_ed: m_gelijkmatig.max(m_punt),
            v_gelijkmatig,
            v_punt,
            v_ed: v_gelijkmatig.max(v_punt),
        }
    }

    /// Selectie van de doorbuigingsgrenzen (EC0 NB A1.4.3(3),(4)).
    pub fn doorbuigingseis(&self) -> DoorbuigingsEis {
        let l_mm = self.l_m * 1000.0;
        let bron = Bron::nb("EC0", "A1.4.3(3)");
        let (w3_met_psi1, grens, formule) = if self.toepassing == Toepassing::Vloer {
            if self.scheurgevoelig {
                (true, l_mm / 500.0, "w_2 + w_3 <= l/500")
            } else {
                (true, 0.003 * l_mm, "w_2 + w_3 <= 0,003 l")
            }
        } else if self.beloopbaar {
            (true, 0.003 * l_mm, "w_2 + w_3 <= 0,003 l")
        } else {
            (false, l_mm / 250.0, "w_2 + w_3 <= l/250")
        };
        DoorbuigingsEis {
            w3_met_psi1,
            grens_w23_mm: grens,
            formule_w23: formule,
            bron_w23: bron,
            grens_wmax_mm: l_mm / 250.0,
        }
    }

    /// Massacriterium voor trilling (EC0 NB A1.4.4, opmerking 1).
    ///
    /// Geeft `Some((waarde, omschrijving))` als het criterium is gehaald.
    pub fn trilling_massa(&self, g_lijn: f64, q_lijn: f64) -> Option<(f64, String)> {
        let vlak = self.g_k + self.psi2 * self.q_k;
        if vlak >= 5.0 {
            return Some((vlak, "g_k + psi_2 q_k >= 5 kN/m2".to_string()));
        }
        let totaal = (g_lijn + self.psi2 * q_lijn) * self.l_m;
        if totaal >= 150.0 {
            return Some((totaal, "totale last per ligger >= 150 kN".to_string()));
        }
        None
    }

    /// Signalering voor afschot en wateraccumulatie (dak), uit de karakteristieke doorbuiging.
    pub fn afschot_melding(&self, w_karakteristiek_mm: f64) -> Melding {
        let l_mm = self.l_m * 1000.0;
        let bron = Bron::nb("EC0", "A1.4.3(8)");
        if self.afschot_procent >= 1.6 && w_karakteristiek_mm <= 0.004 * l_mm {
            Melding::signalering(
                "Afschot >= 1,6 % en karakteristieke doorbuiging <= 0,004 l: voldoende afwatering aangenomen bij vaste opleggingen.",
            )
            .met_bron(bron)
        } else {
            Melding::signalering(
                "Wateraccumulatie niet getoetst - constructeur (afschot < 1,6 % of doorbuiging > 0,004 l).",
            )
            .met_bron(bron)
        }
    }
}

/// Bepaal gevolgklasse, leidende veranderlijke belasting en meldingen uit de invoer.
pub fn bepaal_belasting(alg: &Algemeen) -> Result<Belasting> {
    alg.valideer()?;
    let minimum = minimale_gevolgklasse(alg.gebouwtype);
    let cc = alg.gevolgklasse.unwrap_or(minimum);
    let mut meldingen = Vec::new();
    if cc == Gevolgklasse::CC3 {
        meldingen.push(
            Melding::waarschuwing("Gevolgklasse CC3 valt buiten scope v1: resultaat indicatief, constructeur raadplegen.")
                .met_bron(Bron::nb("EC0", "NB.24")),
        );
    }
    let wand = wandbelasting(alg.lichte_scheidingswanden);
    let l_m = alg.overspanning_m;
    let vloer_als_vloer = alg.toepassing == Toepassing::Vloer || alg.dak_beloopbaar;

    let (leidend, label, q_basis, q_punt, psi, bron_q) = if vloer_als_vloer {
        let (q, qp, psi, label) = vloer_categorie(alg.gebruiksfunctie);
        meldingen.push(
            Melding::info(
                "Puntlast Q_k wordt afzonderlijk van q_k getoetst (interpretatie, ter bevestiging door de constructeur).",
            )
            .met_bron(Bron::basis("EC1-1", "6.3.1.2(3)")),
        );
        (
            Leidend::Vloer,
            label.to_string(),
            q,
            qp,
            psi,
            Bron::nb("EC1-1", "tabel NB.1-6.2"),
        )
    } else {
        let q_h = dak_h_q_k(alg.dak_hellingshoek_graden);
        let s = sneeuwbelasting(alg.dak_hellingshoek_graden);
        meldingen.push(
            Melding::info("Dakbelasting H en sneeuw worden niet gecombineerd: de grootste is leidend; Q_k = 2 kN voor dakbalken.")
                .met_bron(Bron::basis("EC1-1", "3.3.2(1)")),
        );
        if s > q_h {
            (
                Leidend::Sneeuw,
                "sneeuw".to_string(),
                s,
                DAK_PUNTLAST_KN,
                [0.0, 0.2, 0.0],
                Bron::nb("EC1-3", "4.1(1), 5.2"),
            )
        } else {
            (
                Leidend::DakH,
                "dak categorie H".to_string(),
                q_h,
                DAK_PUNTLAST_KN,
                [0.0, 0.0, 0.0],
                Bron::nb("EC1-1", "tabel NB.4-6.10"),
            )
        }
    };
    // Lichte scheidingswanden gelden alleen voor vloeren.
    let wand = if vloer_als_vloer { wand } else { 0.0 };
    Ok(Belasting {
        gevolgklasse: cc,
        combinatie: combinatie(cc),
        leidend,
        label,
        q_k: q_basis + wand,
        q_punt_kn: q_punt,
        psi0: psi[0],
        psi1: psi[1],
        psi2: psi[2],
        g_k: alg.permanent_kn_m2,
        l_m,
        toepassing: alg.toepassing,
        beloopbaar: alg.dak_beloopbaar,
        scheurgevoelig: alg.scheurgevoelige_scheidingswanden && alg.toepassing == Toepassing::Vloer,
        afschot_procent: alg.dak_afschot_procent,
        meldingen,
        bron_q,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn alg(json: &str) -> Algemeen {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn dwarskracht_uit_puntlast_bij_korte_overspanning() {
        // CC2, cat A vloer: Q_k = 3 kN, psi_0 = 0,4.
        let mut a = alg(r#"{"overspanning_m":1,"permanent_kn_m2":1}"#);
        a.gevolgklasse = Some(Gevolgklasse::CC2);
        let b = bepaal_belasting(&a).unwrap();
        let u = b.ugt(1.0, 0.5);
        let v_a: f64 = 1.35 * 1.0 * 1.0 / 2.0 + 1.5 * 0.4 * 3.0;
        let v_b = 1.2 * 1.0 * 1.0 / 2.0 + 1.5 * 3.0;
        assert_relative_eq!(u.v_punt, v_a.max(v_b), epsilon = 1e-12);
        assert_relative_eq!(u.v_gelijkmatig, u.q_d / 2.0, epsilon = 1e-12);
        assert!(u.v_punt > u.v_gelijkmatig);
        assert_relative_eq!(u.v_ed, u.v_punt, epsilon = 1e-12);
    }

    #[test]
    fn combinaties_cc1_en_cc3() {
        let mut a = alg(r#"{"overspanning_m":5,"permanent_kn_m2":1}"#);
        a.gevolgklasse = Some(Gevolgklasse::CC1);
        let b = bepaal_belasting(&a).unwrap();
        let u = b.ugt(10.0, 5.0);
        assert_relative_eq!(u.q_d_a, 1.2 * 10.0 + 1.35 * 0.4 * 5.0, epsilon = 1e-12);
        assert_relative_eq!(u.q_d_b, 1.1 * 10.0 + 1.35 * 5.0, epsilon = 1e-12);
        assert_relative_eq!(u.q_d, 17.75, epsilon = 1e-12);

        a.gevolgklasse = Some(Gevolgklasse::CC3);
        let b = bepaal_belasting(&a).unwrap();
        let u = b.ugt(10.0, 5.0);
        assert_relative_eq!(u.q_d_a, 15.0 + 1.65 * 0.4 * 5.0, epsilon = 1e-12);
        assert_relative_eq!(u.q_d_b, 13.0 + 1.65 * 5.0, epsilon = 1e-12);
        assert_relative_eq!(u.q_d, 21.25, epsilon = 1e-12);
        assert!(b.meldingen.iter().any(|m| m.tekst.contains("CC3")));
    }

    #[test]
    fn gevolgklasse_volgt_gebouwtype_en_mag_niet_lager() {
        let a = alg(r#"{"overspanning_m":5,"permanent_kn_m2":1}"#);
        assert_eq!(
            bepaal_belasting(&a).unwrap().gevolgklasse,
            Gevolgklasse::CC1
        );
        let a = alg(
            r#"{"overspanning_m":5,"permanent_kn_m2":1,"gebouwtype":"woongebouw","gevolgklasse":"CC1"}"#,
        );
        assert!(bepaal_belasting(&a).is_err());
        let a = alg(
            r#"{"overspanning_m":5,"permanent_kn_m2":1,"gebouwtype":"woongebouw","gevolgklasse":"CC3"}"#,
        );
        assert_eq!(
            bepaal_belasting(&a).unwrap().gevolgklasse,
            Gevolgklasse::CC3
        );
    }

    #[test]
    fn dak_h_hellingen_en_sneeuw() {
        assert_relative_eq!(dak_h_q_k(0.0), 1.0);
        assert_relative_eq!(dak_h_q_k(14.9), 1.0);
        assert_relative_eq!(dak_h_q_k(15.0), 1.0, epsilon = 1e-12);
        assert_relative_eq!(dak_h_q_k(17.5), 0.5, epsilon = 1e-12);
        assert_relative_eq!(dak_h_q_k(20.0), 0.0, epsilon = 1e-12);
        assert_relative_eq!(dak_h_q_k(45.0), 0.0);
        assert_relative_eq!(sneeuwbelasting(0.0), 0.56, epsilon = 1e-12);
        assert_relative_eq!(sneeuwbelasting(45.0), 0.56 * 0.5, epsilon = 1e-12);
        assert_relative_eq!(sneeuwbelasting(60.0), 0.0);
    }

    #[test]
    fn dak_kiest_grootste_van_h_en_sneeuw() {
        let a = alg(r#"{"overspanning_m":5,"permanent_kn_m2":1,"toepassing":"dak"}"#);
        let b = bepaal_belasting(&a).unwrap();
        assert_eq!(b.leidend, Leidend::DakH);
        assert_relative_eq!(b.q_k, 1.0);
        assert_relative_eq!(b.q_punt_kn, 2.0);
        let a = alg(
            r#"{"overspanning_m":5,"permanent_kn_m2":1,"toepassing":"dak","dak_hellingshoek_graden":25}"#,
        );
        let b = bepaal_belasting(&a).unwrap();
        assert_eq!(b.leidend, Leidend::Sneeuw);
        assert_relative_eq!(b.q_k, 0.56, epsilon = 1e-12);
        assert_relative_eq!(b.psi1, 0.2);
        assert!(!b.doorbuigingseis().w3_met_psi1);
        assert_relative_eq!(b.doorbuigingseis().grens_w23_mm, 20.0);
    }

    #[test]
    fn doorbuigingsgrenzen_vloer() {
        let a = alg(r#"{"overspanning_m":5,"permanent_kn_m2":1}"#);
        let e = bepaal_belasting(&a).unwrap().doorbuigingseis();
        assert_relative_eq!(e.grens_w23_mm, 15.0, epsilon = 1e-12);
        let a = alg(
            r#"{"overspanning_m":5,"permanent_kn_m2":1,"scheurgevoelige_scheidingswanden":true}"#,
        );
        let e = bepaal_belasting(&a).unwrap().doorbuigingseis();
        assert_relative_eq!(e.grens_w23_mm, 10.0, epsilon = 1e-12);
    }

    #[test]
    fn massacriterium() {
        let a = alg(r#"{"overspanning_m":6,"permanent_kn_m2":6}"#);
        let b = bepaal_belasting(&a).unwrap();
        assert!(b.trilling_massa(20.0, 5.0).is_some());
        let a = alg(r#"{"overspanning_m":6,"permanent_kn_m2":1}"#);
        let b = bepaal_belasting(&a).unwrap();
        assert!(b.trilling_massa(5.0, 5.0).is_none());
        assert!(b.trilling_massa(30.0, 5.0).is_some());
    }

    #[test]
    fn onbekend_veld_wordt_geweigerd() {
        let r: std::result::Result<Algemeen, _> =
            serde_json::from_str(r#"{"overspanning_m":5,"permanent_kn_m2":1,"onzin":1}"#);
        assert!(r.is_err());
    }
}
