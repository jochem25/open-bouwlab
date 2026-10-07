//! Beton: gewapende rechthoekige balk, vrij opgelegd (EC2 met NB).
//!
//! Eenheden in deze module: mm, N/mm2 (MPa), kN, kNm en mm2 waar vermeld.
//! Eigen uitwerking van het rechthoekige spanningsblok; geen normtekst.

use std::collections::BTreeMap;

use crate::advies::stel_resultaat_samen;
use crate::belasting::{bepaal_belasting, Belasting, GEWICHT_BETON};
use crate::error::Result;
use crate::model::{
    BetonInvoer, BetonKeuze, BetonResultaat, Kandidaat, Milieuklasse, Sterkteklasse, Toepassing,
};
use crate::toets::{Bron, Melding, Toets, UC_PLAFOND};

/// Partiele factor beton (EC2 tabel 2.1N).
const GAMMA_C: f64 = 1.5;
/// Partiele factor wapeningsstaal (EC2 tabel 2.1N).
const GAMMA_S: f64 = 1.15;
/// Karakteristieke vloeigrens wapening in N/mm2 (B500).
const F_YK: f64 = 500.0;
/// Factor lambda van het rechthoekige spanningsblok (f_ck <= 50).
const LAMBDA: f64 = 0.8;
/// Minimale beugelafstand die nog uitvoerbaar wordt geacht, in mm (eigen aanname).
const S_MIN_UITVOERBAAR: f64 = 50.0;
/// Beschikbare staafdiameters in mm.
const DIAMETERS: [f64; 3] = [12.0, 16.0, 20.0];

/// Materiaalwaarden van een sterkteklasse: (f_ck, f_ctm in MPa, E_cm in GPa), EC2 tabel 3.1.
pub fn materiaal(klasse: Sterkteklasse) -> (f64, f64, f64) {
    match klasse {
        Sterkteklasse::C20_25 => (20.0, 2.2, 30.0),
        Sterkteklasse::C25_30 => (25.0, 2.6, 31.0),
        Sterkteklasse::C30_37 => (30.0, 2.9, 33.0),
        Sterkteklasse::C35_45 => (35.0, 3.2, 34.0),
    }
}

/// Dekkingsgegevens (EC2 4.4.1, NB).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dekking {
    /// Constructieklasse (3 of 4).
    pub constructieklasse: u8,
    /// c_min,dur in mm.
    pub c_min_dur: f64,
    /// Nominale dekking hoofdstaaf in mm.
    pub c_nom_hoofd: f64,
    /// Nominale dekking beugel in mm.
    pub c_nom_beugel: f64,
    /// Dekking op de beugel in mm (maatgevend voor d).
    pub beugeldekking: f64,
}

/// Bepaal de dekking: S4 als basis, een klasse lager bij C30/37 of hoger (XC1) en
/// C35/45 of hoger (XC3); c_nom = c_min + 5 mm.
pub fn dekking(
    klasse: Sterkteklasse,
    milieu: Milieuklasse,
    phi_hoofd: f64,
    phi_beugel: f64,
) -> Dekking {
    let (f_ck, _, _) = materiaal(klasse);
    let verlaagd = match milieu {
        Milieuklasse::Xc1 => f_ck >= 30.0,
        Milieuklasse::Xc3 => f_ck >= 35.0,
    };
    let cl: u8 = if verlaagd { 3 } else { 4 };
    let c_min_dur = match (milieu, cl) {
        (Milieuklasse::Xc1, 3) => 10.0,
        (Milieuklasse::Xc1, _) => 15.0,
        (Milieuklasse::Xc3, 3) => 20.0,
        (Milieuklasse::Xc3, _) => 25.0,
    };
    let c_nom_hoofd = phi_hoofd.max(c_min_dur).max(10.0) + 5.0;
    let c_nom_beugel = phi_beugel.max(c_min_dur).max(10.0) + 5.0;
    Dekking {
        constructieklasse: cl,
        c_min_dur,
        c_nom_hoofd,
        c_nom_beugel,
        beugeldekking: c_nom_beugel.max(c_nom_hoofd - phi_beugel),
    }
}

/// Grens voor x_u/d: 500/(500 + f_yd) (EC2 NB 6.1(9)).
pub fn xi_grens(f_yd: f64) -> f64 {
    500.0 / (500.0 + f_yd)
}

/// Voordimensionering van een betonbalk.
pub fn bereken_beton(invoer: &BetonInvoer) -> Result<BetonResultaat> {
    invoer.valideer()?;
    let bel = bepaal_belasting(&invoer.algemeen)?;
    let dek = dekking(
        invoer.sterkteklasse,
        invoer.milieuklasse,
        invoer.phi_hoofd_mm,
        invoer.phi_beugel_mm,
    );

    let hoogtes: Vec<f64> = if invoer.hoogte_automatisch {
        (250..=1000).step_by(50).map(f64::from).collect()
    } else {
        invoer.hoogte_mm.into_iter().collect()
    };
    let kandidaten: Vec<Kandidaat> = hoogtes
        .iter()
        .map(|&h| toets_hoogte(invoer, &bel, &dek, h))
        .collect();

    let mut meldingen = bel.meldingen.clone();
    meldingen.push(Melding::info(&format!(
        "Dekking en nuttige hoogte met aangenomen hoofdstaafdiameter {} mm; staafkeuze hooguit deze diameter, d niet herberekend (conservatief).",
        invoer.phi_hoofd_mm
    )));
    meldingen.push(Melding::info(
        "Doorbuiging via de slankheidstoets; correctie voor staalspanning (7.17) niet toegepast (conservatief). Dwarskracht bij de oplegging (conservatief).",
    ));
    if bel.toepassing == Toepassing::Dak {
        meldingen.push(
            Melding::signalering("Wateraccumulatie niet getoetst - constructeur.")
                .met_bron(Bron::nb("EC0", "A1.4.3(8)")),
        );
    }

    Ok(stel_resultaat_samen(
        kandidaten,
        bel.gevolgklasse,
        bel.l_m,
        meldingen,
        false,
    ))
}

fn toets_hoogte(invoer: &BetonInvoer, bel: &Belasting, dek: &Dekking, h: f64) -> Kandidaat {
    let (f_ck, f_ctm, _e_cm) = materiaal(invoer.sterkteklasse);
    let b = invoer.balkbreedte_mm;
    let phi_s = invoer.phi_beugel_mm;
    let phi_b = invoer.phi_hoofd_mm;
    let l_mm = bel.l_m * 1000.0;
    let f_cd = f_ck / GAMMA_C;
    let f_yd = F_YK / GAMMA_S;
    let xi_lim = xi_grens(f_yd);
    let d = h - dek.beugeldekking - phi_s - phi_b / 2.0;

    let eigen_gewicht = if invoer.algemeen.eigen_gewicht_automatisch {
        GEWICHT_BETON * (b / 1000.0) * (h / 1000.0)
    } else {
        0.0
    };
    let g = bel.g_k * invoer.belastingbreedte_m + eigen_gewicht;
    let q = bel.q_k * invoer.belastingbreedte_m;
    let ugt = bel.ugt(g, q);
    let (m_ed, v_ed) = (ugt.m_ed, ugt.v_ed);

    let mut tw: BTreeMap<String, f64> = BTreeMap::new();
    tw.insert("d".into(), d);
    tw.insert("f_cd".into(), f_cd);
    tw.insert("f_yd".into(), f_yd);
    tw.insert("xi_grens".into(), xi_lim);
    tw.insert("q_d_6.10a".into(), ugt.q_d_a);
    tw.insert("q_d_6.10b".into(), ugt.q_d_b);
    tw.insert("q_d".into(), ugt.q_d);
    tw.insert("m_ed".into(), m_ed);
    tw.insert("v_ed".into(), v_ed);

    let mut k = Kandidaat {
        naam: format!("{} x {}", fmt0(b), fmt0(h)),
        hoogte_mm: h,
        eigen_gewicht_kn_m: eigen_gewicht,
        gewicht_kg_m: None,
        toetsen: Vec::new(),
        meldingen: Vec::new(),
        afgewezen: None,
        tussenwaarden: BTreeMap::new(),
        beton: None,
    };

    let buiging_bron = Bron::basis("EC2", "6.1, 3.1.7");
    let xi_toets = |waarde: f64| {
        Toets::nieuw(
            "buiging_xu_d",
            "Buiging: drukzonehoogte",
            "x_u/d <= 500/(500+f_yd)",
            Bron::nb("EC2", "6.1(9)"),
        )
        .waarden(waarde, xi_lim, "-")
        .uc_uit_waarden()
    };

    if d <= 0.0 {
        k.afgewezen = Some("nuttige hoogte is niet positief".into());
        k.tussenwaarden = tw;
        return k;
    }

    // Buiging: rechthoekig spanningsblok, horizontale staaltak, geen drukwapening.
    let mu = m_ed * 1e6 / (b * d * d * f_cd);
    tw.insert("mu".into(), mu);
    if mu > 0.5 {
        k.toetsen.push(
            Toets::nieuw(
                "buiging_xu_d",
                "Buiging: drukzonehoogte",
                "mu <= 0,5",
                buiging_bron,
            )
            .waarden(mu, 0.5, "-")
            .met_uc(UC_PLAFOND),
        );
        k.afgewezen = Some("buigend moment niet opneembaar zonder drukwapening".into());
        k.tussenwaarden = tw;
        return k;
    }
    let omega = 1.0 - (1.0 - 2.0 * mu).sqrt();
    let xi = omega / LAMBDA;
    let as_req = omega * b * d * f_cd / f_yd;
    tw.insert("omega".into(), omega);
    tw.insert("xi".into(), xi);
    tw.insert("as_req".into(), as_req);
    k.toetsen.push(xi_toets(xi));

    let w_el = b * h * h / 6.0;
    let mu_min = f_ctm * w_el / (b * d * d * f_cd);
    let as_min1 = (1.0 - (1.0 - 2.0 * mu_min).max(0.0).sqrt()) * b * d * f_cd / f_yd;
    let as_min = as_min1.min(1.25 * as_req);
    let as_max = 0.04 * b * h;
    tw.insert("as_min1".into(), as_min1);
    tw.insert("as_min".into(), as_min);
    tw.insert("as_max".into(), as_max);

    // Wapeningskeuze: kleinste A_s,prov die past in een laag.
    let nodig = as_req.max(as_min);
    let min_ruimte = [phi_b.max(0.0), invoer.d_g_mm + 5.0, 20.0];
    let mut opties: Vec<(f64, u32, f64)> = Vec::new();
    for n in 2u32..=5 {
        for &phi in DIAMETERS.iter().filter(|&&p| p <= phi_b) {
            opties.push((
                f64::from(n) * std::f64::consts::PI * phi * phi / 4.0,
                n,
                phi,
            ));
        }
    }
    opties.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    let keuze = opties.iter().find(|(a, n, phi)| {
        let vrij =
            (b - 2.0 * dek.beugeldekking - 2.0 * phi_s - f64::from(*n) * phi) / f64::from(*n - 1);
        *a >= nodig && vrij >= phi.max(min_ruimte[1]).max(min_ruimte[2])
    });
    let Some(&(as_prov, n_staven, phi)) = keuze else {
        k.afgewezen = Some("wapening past niet in een laag".into());
        k.tussenwaarden = tw;
        return k;
    };
    tw.insert("as_prov".into(), as_prov);

    k.toetsen.push(
        Toets::nieuw(
            "buiging_as_min",
            "Minimumwapening",
            "A_s,prov >= A_s,min",
            Bron::nb("EC2", "9.2.1.1(1)"),
        )
        .waarden(as_prov, as_min, "mm2"),
    );
    k.toetsen.push(
        Toets::nieuw(
            "buiging_as_max",
            "Maximumwapening",
            "A_s,prov <= 0,04 A_c",
            Bron::nb("EC2", "9.2.1.1(3)"),
        )
        .waarden(as_prov, as_max, "mm2")
        .uc_uit_waarden(),
    );

    // Dwarskracht (EC2 6.2.2, 6.2.3).
    let kk = (1.0 + (200.0 / d).sqrt()).min(2.0);
    let rho_l = (as_prov / (b * d)).min(0.02);
    let v_min = 0.035 * kk.powf(1.5) * f_ck.sqrt();
    let v_rd_c = (0.12 * kk * (100.0 * rho_l * f_ck).powf(1.0 / 3.0)).max(v_min) * b * d / 1e3;
    let z = 0.9 * d;
    let nu1 = 0.6 * (1.0 - f_ck / 250.0);
    let v_rd_max_bij = |cot: f64| b * z * nu1 * f_cd / (cot + 1.0 / cot) / 1e3;
    let cot = if v_ed <= v_rd_max_bij(2.5) {
        2.5
    } else {
        let x = b * z * nu1 * f_cd / (v_ed * 1e3);
        if x >= 2.0 {
            ((x + (x * x - 4.0).sqrt()) / 2.0).clamp(1.0, 2.5)
        } else {
            1.0
        }
    };
    let v_rd_max = v_rd_max_bij(cot);
    let uc_vmax = v_ed / v_rd_max;
    // Bij een gekozen cot(theta) op de rand geldt UC = 1 binnen afrondfouten.
    let uc_vmax = if cot < 2.5 && uc_vmax < 1.0 + 1e-9 {
        uc_vmax.min(1.0)
    } else {
        uc_vmax
    };
    let asw_s_req = v_ed * 1e3 / (z * f_yd * cot);
    let asw_s_min = 0.08 * f_ck.sqrt() / F_YK * b;
    let s_l_max = if v_ed > v_rd_c {
        (0.75 * d).min(300.0)
    } else {
        300.0
    };
    let a_sw = 2.0 * std::f64::consts::PI * phi_s * phi_s / 4.0;
    let s_ruw = s_l_max.min(a_sw / asw_s_req.max(asw_s_min));
    let s = 10.0 * (s_ruw / 10.0).floor();
    tw.insert("v_rd_c".into(), v_rd_c);
    tw.insert("v_rd_max".into(), v_rd_max);
    tw.insert("cot_theta".into(), cot);
    tw.insert("asw_s_req".into(), asw_s_req);
    tw.insert("asw_s_min".into(), asw_s_min);
    tw.insert("s_l_max".into(), s_l_max);
    tw.insert("beugel_s".into(), s);

    k.toetsen.push(
        Toets::nieuw(
            "dwarskracht_vrdmax",
            "Dwarskracht: drukdiagonaal",
            "V_Ed <= V_Rd,max",
            Bron::basis("EC2", "6.2.3 (6.9)"),
        )
        .waarden(v_ed, v_rd_max, "kN")
        .met_uc(uc_vmax),
    );
    let mut beugels = Toets::nieuw(
        "dwarskracht_beugels",
        "Dwarskrachtwapening",
        "s <= s_l,max; A_sw/s >= max(nodig; minimum)",
        Bron::nb("EC2", "9.2.2(5),(6)"),
    )
    .waarden(s, s_l_max, "mm");
    if s < S_MIN_UITVOERBAAR {
        beugels = beugels.status(crate::toets::ToetsStatus::VoldoetNiet);
    }
    k.toetsen.push(beugels);

    // Slankheid (EC2 7.4.2, formules 7.16a/b), K = 1,0.
    let rho = (as_req / (b * d)).max(1e-9);
    let rho0 = f_ck.sqrt() * 1e-3;
    let mut grens_ld = if rho <= rho0 {
        11.0 + 1.5 * f_ck.sqrt() * rho0 / rho + 3.2 * f_ck.sqrt() * (rho0 / rho - 1.0).powf(1.5)
    } else {
        11.0 + 1.5 * f_ck.sqrt() * rho0 / rho
    };
    if bel.l_m > 7.0 && bel.scheurgevoelig {
        grens_ld *= 7.0 / bel.l_m;
    }
    let ld = l_mm / d;
    tw.insert("rho".into(), rho);
    tw.insert("l_d".into(), ld);
    tw.insert("l_d_grens".into(), grens_ld);
    k.toetsen.push(
        Toets::nieuw(
            "slankheid",
            "Doorbuiging via slankheid",
            "l/d <= K [11 + ...]",
            Bron::basis("EC2", "7.4.2 (7.16a/b)"),
        )
        .waarden(ld, grens_ld, "-")
        .uc_uit_waarden(),
    );

    // Trilling: massacriterium (alleen vloeren).
    if bel.vloergedrag() {
        let trilling_bron = Bron::nb("EC0", "A1.4.4");
        let t = Toets::nieuw(
            "trilling",
            "Trilling",
            "g_k + psi_2 q_k >= 5 kN/m2 of totaal >= 150 kN",
            trilling_bron,
        );
        k.toetsen.push(match bel.trilling_massa(g, q) {
            Some((waarde, via)) => {
                let (grens, eenheid) = if via.contains("150") {
                    (150.0, "kN")
                } else {
                    (5.0, "kN/m2")
                };
                t.waarden(waarde, grens, eenheid)
            }
            None => t.niet_getoetst(
                "massacriterium niet gehaald; eigenfrequentie niet berekend in v1 - constructeur",
                false,
            ),
        });
    }

    k.beton = Some(BetonKeuze {
        aantal: n_staven,
        diameter_mm: phi,
        a_s_prov_mm2: as_prov,
        beugel_diameter_mm: phi_s,
        beugel_s_mm: s,
        cot_theta: cot,
    });
    k.tussenwaarden = tw;
    k
}

fn fmt0(x: f64) -> String {
    format!("{x:.0}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn dekking_xc1_c30_is_s3() {
        let d = dekking(Sterkteklasse::C30_37, Milieuklasse::Xc1, 20.0, 8.0);
        assert_eq!(d.constructieklasse, 3);
        assert_relative_eq!(d.c_min_dur, 10.0);
        assert_relative_eq!(d.c_nom_hoofd, 25.0);
        assert_relative_eq!(d.c_nom_beugel, 15.0);
        assert_relative_eq!(d.beugeldekking, 17.0);
    }

    #[test]
    fn dekking_xc3_alleen_bij_c35_een_klasse_lager() {
        let d = dekking(Sterkteklasse::C35_45, Milieuklasse::Xc3, 20.0, 8.0);
        assert_eq!(d.constructieklasse, 3);
        assert_relative_eq!(d.c_min_dur, 20.0);
        assert_relative_eq!(d.c_nom_hoofd, 25.0);
        assert_relative_eq!(d.beugeldekking, 25.0);
        let d = dekking(Sterkteklasse::C30_37, Milieuklasse::Xc3, 20.0, 8.0);
        assert_eq!(d.constructieklasse, 4);
        assert_relative_eq!(d.c_min_dur, 25.0);
        assert_relative_eq!(d.c_nom_hoofd, 30.0);
        let d = dekking(Sterkteklasse::C25_30, Milieuklasse::Xc1, 20.0, 8.0);
        assert_eq!(d.constructieklasse, 4);
        assert_relative_eq!(d.c_min_dur, 15.0);
    }

    #[test]
    fn xi_grens_b500() {
        assert_relative_eq!(xi_grens(500.0 / 1.15), 0.535, epsilon = 5e-4);
    }

    fn invoer(json: &str) -> BetonInvoer {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn balk_te_smal_voor_wapening_wordt_afgewezen_met_reden() {
        let i = invoer(
            r#"{ "algemeen": { "overspanning_m": 8.0, "permanent_kn_m2": 8.0 },
                 "belastingbreedte_m": 5.0, "sterkteklasse": "C30/37", "balkbreedte_mm": 150 }"#,
        );
        let r = bereken_beton(&i).unwrap();
        assert!(r
            .kandidaten
            .iter()
            .any(|k| k.afgewezen.as_deref() == Some("wapening past niet in een laag")));
        // Een afgewezen kandidaat wordt nooit het advies.
        if let Some(a) = &r.advies {
            let k = r.kandidaten.iter().find(|k| k.naam == a.naam).unwrap();
            assert!(k.afgewezen.is_none());
        }
    }

    #[test]
    fn vaste_hoogte_vereist_hoogte() {
        let i = invoer(
            r#"{ "algemeen": { "overspanning_m": 6.0, "permanent_kn_m2": 4.0 },
                 "belastingbreedte_m": 3.0, "sterkteklasse": "C30/37", "balkbreedte_mm": 300,
                 "hoogte_automatisch": false }"#,
        );
        assert!(bereken_beton(&i).is_err());
    }
}
