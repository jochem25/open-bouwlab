//! Hout: massieve balk of balklaag, vrij opgelegd (EC5 met NB).
//!
//! Eenheden in deze module: mm, N/mm2 (MPa), kN, kNm; trillingsgrootheden in m, kg/m2
//! en Nm2/m waar vermeld. Eigen uitwerking; geen normtekst. Sterkteklassewaarden komen
//! uit een secundaire bron en zijn vervangbaar via [`bereken_hout_met_klassen`].

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::advies::stel_resultaat_samen;
use crate::belasting::{bepaal_belasting, Belasting, Leidend};
use crate::error::{ConstructieFout, Result};
use crate::model::{
    Gebruiksfunctie, HoutElement, HoutInvoer, HoutResultaat, Kandidaat, Toepassing,
};
use crate::toets::{Bron, Melding, Toets, ToetsStatus, UC_PLAFOND};

/// Partiele factor massief hout (EC5 tabel 2.3).
pub const GAMMA_M: f64 = 1.3;
/// Factor k_cr voor afschuiving, prismatische ligger (EC5 NB 6.1.7(2)).
pub const K_CR: f64 = 1.0;
/// Referentiestijfheid EI_1 in Nm2/m voor k_r (EC5 NB 5.2(5)).
const EI_REF: f64 = 50_000.0;
/// Grens a voor de statische puntlast in mm/kN (EC5 NB 7.3.3(2)).
const A_GRENS: f64 = 1.0;
/// Ondergrens eerste eigenfrequentie in Hz (EC5 7.3.3(1)).
const F1_GRENS: f64 = 8.0;
/// Dempingsverhouding zeta voor vloeren (EC5 7.3.1(3)).
const ZETA: f64 = 0.01;
/// Parameter b voor de snelheidsrespons (EC5 NB 7.3.3(2)).
const B_SNELHEID: f64 = 120.0;
/// Hoogteassortiment in mm (eigen keuze, geen norm; te bevestigen).
pub const HOOGTES_MM: [f64; 7] = [146.0, 171.0, 196.0, 221.0, 246.0, 271.0, 296.0];

/// Gebundelde sterkteklassedata (secundaire bron, te verifieren aan EN 338).
const KLASSEN_JSON: &str = include_str!("../data/hout_sterkteklassen.json");

/// Materiaalwaarden van een sterkteklasse (N/mm2 en kg/m3).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Klasse {
    /// Naam, bijv. "C24".
    pub naam: String,
    /// Karakteristieke buigsterkte f_m,k.
    pub f_m_k: f64,
    /// Karakteristieke afschuifsterkte f_v,k.
    pub f_v_k: f64,
    /// Gemiddelde elasticiteitsmodulus evenwijdig aan de vezel E_0,mean.
    pub e_0_mean: f64,
    /// 5%-waarde van de elasticiteitsmodulus E_0,05.
    pub e_0_05: f64,
    /// Karakteristieke dichtheid rho_k in kg/m3.
    pub rho_k: f64,
    /// Gemiddelde dichtheid rho_mean in kg/m3.
    pub rho_mean: f64,
    /// Herkomst van de waarden.
    pub bron: String,
}

/// De gebundelde sterkteklassen.
pub fn standaard_klassen() -> Result<Vec<Klasse>> {
    serde_json::from_str(KLASSEN_JSON).map_err(|e| ConstructieFout::Data(e.to_string()))
}

/// Belastingduurklasse (EC5 tabel 2.1, 3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Duur {
    /// Blijvend (langer dan 10 jaar).
    Blijvend,
    /// Lang.
    Lang,
    /// Middellang.
    Middellang,
    /// Kort (minder dan een week).
    Kort,
    /// Zeer kort.
    ZeerKort,
}

impl Duur {
    /// Naam in de rapportage.
    pub fn naam(self) -> &'static str {
        match self {
            Duur::Blijvend => "blijvend",
            Duur::Lang => "lang",
            Duur::Middellang => "middellang",
            Duur::Kort => "kort",
            Duur::ZeerKort => "zeer kort",
        }
    }
}

/// k_mod voor massief hout; klimaatklasse 1 en 2 hebben dezelfde waarden (EC5 tabel 3.1).
pub fn k_mod(duur: Duur) -> f64 {
    match duur {
        Duur::Blijvend => 0.60,
        Duur::Lang => 0.70,
        Duur::Middellang => 0.80,
        Duur::Kort => 0.90,
        Duur::ZeerKort => 1.10,
    }
}

/// k_def voor massief hout (EC5 tabel 3.2).
pub fn k_def(klimaatklasse: u8) -> f64 {
    if klimaatklasse == 2 {
        0.80
    } else {
        0.60
    }
}

/// Duurklasse van de leidende veranderlijke belasting.
///
/// Vloerbelasting (en lichte wanden): middellang (EC5 NB 2.3.1.2 tabel 2.2). Sneeuw: kort
/// (idem). Dak categorie H staat niet in die tabel en volgt het duurcriterium van
/// EC5 2.3.1.2 tabel 2.1: kort.
pub fn duur_veranderlijk(leidend: Leidend) -> Duur {
    match leidend {
        Leidend::Vloer => Duur::Middellang,
        Leidend::DakH | Leidend::Sneeuw => Duur::Kort,
    }
}

/// k_h voor massief hout (EC5 3.2(3), formule 3.1).
pub fn k_h(h_mm: f64) -> f64 {
    if h_mm < 150.0 {
        (150.0 / h_mm).powf(0.2).min(1.3)
    } else {
        1.0
    }
}

/// Buigstijfheid van de vloerplaat in Nm2/m: E t^3 / 12 per m breedte.
pub fn ei_plaat(dikte_mm: f64, e_mean: f64) -> f64 {
    e_mean * dikte_mm.powi(3) / 12.0 * 1e-3
}

/// Reductiefactor k_r voor het puntlastmoment (EC5 NB 5.2(5)), begrensd tot 0 .. 1.
pub fn k_r(hoh_m: f64, ei_plaat_nm2_m: f64) -> f64 {
    (0.37 + 0.8 * hoh_m - ei_plaat_nm2_m / EI_REF).clamp(0.0, 1.0)
}

/// k_crit uit de relatieve slankheid (EC5 6.3.3, formule 6.34).
pub fn k_crit(lambda_rel: f64) -> f64 {
    if lambda_rel <= 0.75 {
        1.0
    } else if lambda_rel <= 1.4 {
        1.56 - 0.75 * lambda_rel
    } else {
        1.0 / (lambda_rel * lambda_rel)
    }
}

/// Voordimensionering van een houten balk of balklaag met de gebundelde klassendata.
pub fn bereken_hout(invoer: &HoutInvoer) -> Result<HoutResultaat> {
    bereken_hout_met_klassen(invoer, &standaard_klassen()?)
}

/// Als [`bereken_hout`], met eigen klassendata (bijv. na verificatie aan EN 338).
pub fn bereken_hout_met_klassen(invoer: &HoutInvoer, klassen: &[Klasse]) -> Result<HoutResultaat> {
    invoer.valideer()?;
    let bel = bepaal_belasting(&invoer.algemeen)?;
    let klasse = klassen
        .iter()
        .find(|k| k.naam == invoer.sterkteklasse.naam())
        .ok_or_else(|| {
            ConstructieFout::Data(format!(
                "sterkteklasse {} ontbreekt in de klassendata",
                invoer.sterkteklasse.naam()
            ))
        })?;

    let hoogtes: Vec<f64> = if invoer.hoogte_automatisch {
        HOOGTES_MM.to_vec()
    } else {
        invoer.hoogte_mm.into_iter().collect()
    };
    let kandidaten: Vec<Kandidaat> = hoogtes
        .iter()
        .map(|&h| toets_hoogte(invoer, &bel, klasse, h))
        .collect();

    let mut meldingen = bel.meldingen.clone();
    meldingen.push(Melding::waarschuwing(
        "Sterkteklassewaarden uit secundaire bron, te verifieren aan EN 338.",
    ));
    meldingen.push(Melding::info(
        "Opleggingen star aangenomen (geen elastische opleggingen).",
    ));
    if let (HoutElement::Balklaag { hoh_mm }, Some(p)) = (&invoer.element, &invoer.vloerplaat) {
        if k_r(hoh_mm / 1000.0, ei_plaat(p.dikte_mm, p.e_mean_n_mm2)) <= 0.0 {
            meldingen.push(Melding::waarschuwing(
                "k_r valt op de ondergrens: de puntlast telt niet mee in het buigend moment door de stijve vloerplaat - controle constructeur.",
            ));
        }
    }
    if bel.leidend == Leidend::DakH {
        meldingen.push(
            Melding::info(
                "Dakbelasting categorie H: belastingduur kort (duurcriterium); de NB-tabel noemt deze categorie niet.",
            )
            .met_bron(Bron::basis("EC5", "2.3.1.2 tabel 2.1")),
        );
    }
    if matches!(invoer.element, HoutElement::Balk { .. }) && bel.vloergedrag() {
        meldingen.push(Melding::signalering(
            "Trilling niet getoetst voor een enkele balk - constructeur.",
        ));
    }
    if bel.toepassing == Toepassing::Dak {
        let w = kandidaten
            .iter()
            .find(|k| k.voldoet())
            .or_else(|| kandidaten.last())
            .map_or(0.0, |k| {
                k.tussenwaarden
                    .get("w_karakteristiek")
                    .copied()
                    .unwrap_or(0.0)
            });
        meldingen.push(bel.afschot_melding(w));
    }

    let mut res = stel_resultaat_samen(kandidaten, bel.gevolgklasse, bel.l_m, meldingen, false);
    if let Some(a) = res.advies.as_mut() {
        a.kengetallen.eigenfrequentie_hz = res
            .kandidaten
            .iter()
            .find(|k| k.naam == a.naam)
            .and_then(|k| k.tussenwaarden.get("f_1").copied());
    }
    Ok(res)
}

/// Hulp: UC van de kip voor moment `m_knm` en `f_m_d`; geeft (UC, sigma_crit, lambda, k_crit).
fn kip_uc(
    m_knm: f64,
    f_m_k: f64,
    f_m_d: f64,
    e_0_05: f64,
    b: f64,
    h: f64,
    l_mm: f64,
) -> (f64, f64, f64, f64) {
    let sigma = m_knm * 1e6 / (b * h * h / 6.0);
    // Last op de drukzijde: l_ef = 0,9 l + 2 h (tabel 6.1).
    let l_ef = 0.9 * l_mm + 2.0 * h;
    let sigma_crit = 0.78 * b * b * e_0_05 / (h * l_ef);
    let lambda = (f_m_k / sigma_crit).sqrt();
    let kc = k_crit(lambda);
    (sigma / (kc * f_m_d), sigma_crit, lambda, kc)
}

fn toets_hoogte(invoer: &HoutInvoer, bel: &Belasting, klasse: &Klasse, h: f64) -> Kandidaat {
    let b = invoer.breedte_mm;
    let l_m = bel.l_m;
    let l_mm = l_m * 1000.0;
    let (strook_m, hoh_m) = match invoer.element {
        HoutElement::Balklaag { hoh_mm } => (hoh_mm / 1000.0, Some(hoh_mm / 1000.0)),
        HoutElement::Balk { belastingbreedte_m } => (belastingbreedte_m, None),
    };
    let c = &bel.combinatie;
    let kdef = k_def(invoer.klimaatklasse);

    // Belasting per balk.
    let eigen_gewicht = if invoer.algemeen.eigen_gewicht_automatisch {
        klasse.rho_mean / 100.0 * (b / 1000.0) * (h / 1000.0)
    } else {
        0.0
    };
    let g = bel.g_k * strook_m + eigen_gewicht;
    let q = bel.q_k * strook_m;
    let qp = bel.q_punt_kn;
    let ugt = bel.ugt(g, q);

    // k_r op het puntlastmoment (alleen balklaag).
    let ei_pl = invoer
        .vloerplaat
        .map(|p| ei_plaat(p.dikte_mm, p.e_mean_n_mm2));
    let kr = match hoh_m {
        Some(a) => k_r(a, ei_pl.unwrap_or(0.0)),
        None => 1.0,
    };

    // Momenten (kNm): gelijkmatig en puntlast in het midden, 6.10a en 6.10b.
    let m_punt_a = c.gamma_g_a * g * l_m * l_m / 8.0 + c.gamma_q * bel.psi0 * kr * qp * l_m / 4.0;
    let m_punt_b = c.gamma_g_b * g * l_m * l_m / 8.0 + c.gamma_q * kr * qp * l_m / 4.0;
    let m_punt = m_punt_a.max(m_punt_b);
    let m_d = ugt.m_gelijkmatig.max(m_punt);
    let m_g = c.gamma_g_a.max(c.gamma_g_b) * g * l_m * l_m / 8.0;

    // Dwarskrachten (kN): puntlast op afstand h van de oplegging, zonder k_r.
    let h_m = h / 1000.0;
    let v_punt_a = c.gamma_g_a * g * l_m / 2.0 + c.gamma_q * bel.psi0 * qp * (l_m - h_m) / l_m;
    let v_punt_b = c.gamma_g_b * g * l_m / 2.0 + c.gamma_q * qp * (l_m - h_m) / l_m;
    let v_punt = v_punt_a.max(v_punt_b);
    let v_d = ugt.v_gelijkmatig.max(v_punt);
    let v_g = c.gamma_g_a.max(c.gamma_g_b) * g * l_m / 2.0;

    // Sterkten.
    let kmod_gq = k_mod(duur_veranderlijk(bel.leidend));
    let kmod_g = k_mod(Duur::Blijvend);
    let f_m_d = kmod_gq * klasse.f_m_k / GAMMA_M;
    let f_v_d = kmod_gq * klasse.f_v_k / GAMMA_M;
    let f_m_d_g = kmod_g * klasse.f_m_k / GAMMA_M;
    let f_v_d_g = kmod_g * klasse.f_v_k / GAMMA_M;
    let kh = k_h(h);
    let w_mod = b * h * h / 6.0;
    let traagheid = b * h.powi(3) / 12.0;

    let mut tw: BTreeMap<String, f64> = BTreeMap::new();
    tw.insert("q_d_6.10a".into(), ugt.q_d_a);
    tw.insert("q_d_6.10b".into(), ugt.q_d_b);
    tw.insert("q_d".into(), ugt.q_d);
    tw.insert("g_lijn".into(), g);
    tw.insert("q_lijn".into(), q);
    tw.insert("k_r".into(), kr);
    tw.insert("k_h".into(), kh);
    tw.insert("k_mod_gq".into(), kmod_gq);
    tw.insert("k_mod_g".into(), kmod_g);
    tw.insert("k_def".into(), kdef);
    tw.insert("f_m_d".into(), f_m_d);
    tw.insert("f_v_d".into(), f_v_d);
    tw.insert("m_d_gelijkmatig".into(), ugt.m_gelijkmatig);
    tw.insert("m_d_punt".into(), m_punt);
    tw.insert("m_d".into(), m_d);
    tw.insert("m_d_blijvend".into(), m_g);
    tw.insert("v_d_gelijkmatig".into(), ugt.v_gelijkmatig);
    tw.insert("v_d_punt".into(), v_punt);
    tw.insert("v_d".into(), v_d);
    tw.insert("v_d_blijvend".into(), v_g);

    let mut toetsen: Vec<Toets> = Vec::new();
    let mut meldingen: Vec<Melding> = Vec::new();

    // Buiging (EC5 6.1.6, formule 6.11).
    let bron_buiging = Bron::basis("EC5", "6.1.6 (6.11), 3.2(3)");
    toetsen.push(
        Toets::nieuw(
            "buiging",
            "Buiging",
            "sigma_m,d <= k_h f_m,d",
            bron_buiging.clone(),
        )
        .waarden(m_d * 1e6 / w_mod, kh * f_m_d, "N/mm2")
        .uc_uit_waarden(),
    );
    toetsen.push(
        Toets::nieuw(
            "buiging_blijvend",
            "Buiging (blijvend)",
            "sigma_m,d <= k_h f_m,d (alleen G, k_mod blijvend)",
            Bron::basis("EC5", "6.1.6 (6.11), 3.1.3(2)"),
        )
        .waarden(m_g * 1e6 / w_mod, kh * f_m_d_g, "N/mm2")
        .uc_uit_waarden(),
    );

    // Afschuiving (EC5 6.1.7, formule 6.13).
    let tau = |v_kn: f64| 1.5 * v_kn * 1e3 / (K_CR * b * h);
    toetsen.push(
        Toets::nieuw(
            "afschuiving",
            "Afschuiving",
            "tau_d <= f_v,d",
            Bron::basis("EC5", "6.1.7 (6.13)"),
        )
        .waarden(tau(v_d), f_v_d, "N/mm2")
        .uc_uit_waarden(),
    );
    toetsen.push(
        Toets::nieuw(
            "afschuiving_blijvend",
            "Afschuiving (blijvend)",
            "tau_d <= f_v,d (alleen G, k_mod blijvend)",
            Bron::basis("EC5", "6.1.7 (6.13), 3.1.3(2)"),
        )
        .waarden(tau(v_g), f_v_d_g, "N/mm2")
        .uc_uit_waarden(),
    );

    // Kip (EC5 6.3.3).
    if invoer.drukrand_gesteund {
        toetsen.push(
            Toets::nieuw("kip", "Kip", "k_crit = 1,0", Bron::basis("EC5", "6.3.3(5)"))
                .aanname("drukrand doorgaand gesteund aangenomen"),
        );
    } else {
        let (uc, sc, lam, kc) = kip_uc(m_d, klasse.f_m_k, f_m_d, klasse.e_0_05, b, h, l_mm);
        tw.insert("sigma_m_crit".into(), sc);
        tw.insert("lambda_rel_m".into(), lam);
        tw.insert("k_crit".into(), kc);
        let bron = Bron::basis("EC5", "6.3.3 (6.30), (6.32), (6.33), (6.34)");
        toetsen.push(
            Toets::nieuw("kip", "Kip", "sigma_m,d <= k_crit f_m,d", bron.clone())
                .waarden(m_d * 1e6 / w_mod, kc * f_m_d, "N/mm2")
                .met_uc(uc.min(UC_PLAFOND)),
        );
        let (uc_g, ..) = kip_uc(m_g, klasse.f_m_k, f_m_d_g, klasse.e_0_05, b, h, l_mm);
        toetsen.push(
            Toets::nieuw(
                "kip_blijvend",
                "Kip (blijvend)",
                "sigma_m,d <= k_crit f_m,d (alleen G, k_mod blijvend)",
                bron,
            )
            .waarden(m_g * 1e6 / w_mod, kc * f_m_d_g, "N/mm2")
            .met_uc(uc_g.min(UC_PLAFOND)),
        );
    }

    // Doorbuiging (spec 4.6 en 5.7): E_0,mean en k_def.
    let eis = bel.doorbuigingseis();
    let c_w = 5.0 * l_mm.powi(4) / (384.0 * klasse.e_0_mean * traagheid);
    let w_g = g * c_w;
    let w_q = q * c_w;
    let w2 = kdef * (w_g + bel.psi2 * w_q);
    let w3 = if eis.w3_met_psi1 { bel.psi1 * w_q } else { w_q };
    let w_max = (1.0 + kdef) * (w_g + bel.psi2 * w_q);
    tw.insert("w_g".into(), w_g);
    tw.insert("w_q".into(), w_q);
    tw.insert("w_2".into(), w2);
    tw.insert("w_3".into(), w3);
    tw.insert("w_max".into(), w_max);
    tw.insert("w_karakteristiek".into(), w_g + w_q + w2);
    toetsen.push(
        Toets::nieuw(
            "doorbuiging_w2w3",
            "Doorbuiging w_2 + w_3",
            eis.formule_w23,
            eis.bron_w23.clone(),
        )
        .waarden(w2 + w3, eis.grens_w23_mm, "mm")
        .uc_uit_waarden(),
    );
    if invoer.algemeen.uiterlijk_van_belang {
        toetsen.push(
            Toets::nieuw(
                "doorbuiging_wmax",
                "Doorbuiging w_max",
                "w_max <= l/250",
                Bron::nb("EC0", "A1.4.3(4)"),
            )
            .waarden(w_max, eis.grens_wmax_mm, "mm")
            .uc_uit_waarden(),
        );
    } else {
        meldingen.push(Melding::info(
            "w_max niet getoetst: uiterlijk is niet van belang.",
        ));
    }

    // Trilling (EC5 7.3.3): alleen balklaag met vloergedrag.
    if let (true, Some(hoh_m)) = (bel.vloergedrag(), hoh_m) {
        let niet_toetsbaar = match invoer.algemeen.gebruiksfunctie {
            Gebruiksfunctie::WoonVloer | Gebruiksfunctie::Gemeenschappelijk => None,
            Gebruiksfunctie::Kantoor => {
                Some("trilling buiten woonfunctie niet getoetst - constructeur")
            }
            Gebruiksfunctie::WoonTrap | Gebruiksfunctie::WoonBalkon => Some(
                "trilling van trap of balkon niet getoetst (toets geldt voor vloervelden) - constructeur",
            ),
        };
        if let Some(reden) = niet_toetsbaar {
            toetsen.push(
                Toets::nieuw(
                    "trilling",
                    "Trilling",
                    "EC5 7.3.3",
                    Bron::basis("EC5", "7.3.3"),
                )
                .niet_getoetst(reden, false),
            );
        } else {
            trilling(
                &mut toetsen,
                &mut tw,
                invoer,
                bel,
                klasse,
                eigen_gewicht,
                hoh_m,
                ei_pl,
                traagheid,
            );
        }
    }

    Kandidaat {
        naam: format!("{} x {}", fmt0(b), fmt0(h)),
        hoogte_mm: h,
        eigen_gewicht_kn_m: eigen_gewicht,
        gewicht_kg_m: None,
        toetsen,
        meldingen,
        afgewezen: None,
        tussenwaarden: tw,
        beton: None,
    }
}

/// Trillingstoetsen voor een woonvloer (balklaag): f_1, w/F en snelheidsrespons.
#[allow(clippy::too_many_arguments)]
fn trilling(
    toetsen: &mut Vec<Toets>,
    tw: &mut BTreeMap<String, f64>,
    invoer: &HoutInvoer,
    bel: &Belasting,
    klasse: &Klasse,
    eigen_gewicht: f64,
    hoh_m: f64,
    ei_pl: Option<f64>,
    traagheid: f64,
) {
    let l = bel.l_m;
    // Massa uit alleen permanente belasting (EC5 7.3.3(3)), in kg/m2.
    let m = (bel.g_k + eigen_gewicht / hoh_m) * 1000.0 / 9.81;
    // (EI)_L per m breedte in Nm2/m.
    let ei_l = klasse.e_0_mean * traagheid / hoh_m * 1e-6;
    let f1 = std::f64::consts::PI / (2.0 * l * l) * (ei_l / m).sqrt();
    tw.insert("m_trilling".into(), m);
    tw.insert("ei_l".into(), ei_l);
    tw.insert("f_1".into(), f1);

    let f1_toets = Toets::nieuw(
        "trilling_f1",
        "Trilling: eigenfrequentie",
        "f_1 > 8 Hz",
        Bron::basis("EC5", "7.3.3(1), (7.5)"),
    )
    .waarden(f1, F1_GRENS, "Hz");
    if f1 <= F1_GRENS {
        toetsen.push(f1_toets.niet_getoetst(
            "eigenfrequentie <= 8 Hz: nader onderzoek - constructeur",
            true,
        ));
    } else {
        toetsen.push(f1_toets);
    }

    // Statische puntlast: w_1kN = F l^3 / (48 (EI)_L b_ef), F = 1 kN, in mm.
    let vloerbreedte = invoer.vloerbreedte_m;
    // Zonder vloerbreedte geen begrenzing door B mogelijk: conservatief b_ef = hoh.
    let b_ef = match (ei_pl, vloerbreedte) {
        (Some(ei_t), Some(bb)) => (0.95 * l * (ei_t / ei_l).powf(0.25)).min(bb).max(hoh_m),
        _ => hoh_m,
    };
    let w1 = 1e6 * l.powi(3) / (48.0 * ei_l * b_ef);
    tw.insert("b_ef".into(), b_ef);
    tw.insert("w_1kn".into(), w1);
    let mut t = Toets::nieuw(
        "trilling_w1kn",
        "Trilling: statische puntlast",
        "w_1kN <= a (a = 1 mm/kN)",
        Bron::nb("EC5", "7.3.3(2), (7.3)"),
    )
    .waarden(w1, A_GRENS, "mm/kN")
    .uc_uit_waarden();
    if matches!(t.status, ToetsStatus::Voldoet) {
        let tekst = match ei_pl {
            Some(_) if vloerbreedte.is_none() => "zonder vloerbreedte: lastspreiding over een balk (b_ef = hoh); vul de vloerbreedte in voor lastspreiding",
            Some(_) => "lastspreiding volgens EN 1995-1-1:2026 9.3.2.5 (2e generatie); de geldende 1e generatie geeft geen methode",
            None => "zonder vloerplaat: lastspreiding over een balk (b_ef = hoh)",
        };
        t = t.aanname(tekst);
    }
    toetsen.push(t);

    // Snelheidsrespons (7.4, 7.6, 7.7): alleen met vloerplaat en vloerbreedte.
    let v_bron = Bron::nb("EC5", "7.3.3(2), (7.4), (7.6), (7.7)");
    let v_toets = Toets::nieuw(
        "trilling_v",
        "Trilling: snelheidsrespons",
        "v <= b^(f_1 zeta - 1)",
        v_bron,
    );
    let rekenbaar = match (ei_pl, vloerbreedte) {
        (Some(ei_t), Some(bb)) if ei_t < ei_l => Some((ei_t, bb)),
        _ => None,
    };
    match rekenbaar {
        Some((ei_t, bb)) => {
            // n_40 >= 0: bij f_1 >= 40 Hz is de eerste term niet positief en telt n_40 niet mee.
            let n40 =
                (((40.0 / f1).powi(2) - 1.0).max(0.0) * (bb / l).powi(4) * ei_l / ei_t).powf(0.25);
            let v = 4.0 * (0.4 + 0.6 * n40) / (m * bb * l + 200.0);
            let grens = B_SNELHEID.powf(f1 * ZETA - 1.0);
            tw.insert("n_40".into(), n40);
            tw.insert("v".into(), v);
            tw.insert("v_grens".into(), grens);
            toetsen.push(v_toets.waarden(v, grens, "m/(N s2)").uc_uit_waarden());
        }
        None => {
            let reden = match (ei_pl, vloerbreedte) {
                (Some(_), Some(_)) => "snelheidsrespons niet getoetst: vloerplaat stijver dan de balklaag ((EI)_T >= (EI)_L) - constructeur",
                _ => "snelheidsrespons niet getoetst: vloerplaat en vloerbreedte invoeren",
            };
            toetsen.push(v_toets.niet_getoetst(reden, false))
        }
    }
}

fn fmt0(x: f64) -> String {
    format!("{x:.0}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn invoer(extra: &str, algemeen_extra: &str) -> HoutInvoer {
        serde_json::from_str(&format!(
            r#"{{"algemeen":{{"overspanning_m":4.2,"permanent_kn_m2":0.75,
                "eigen_gewicht_automatisch":false,"gevolgklasse":"CC2"{algemeen_extra}}},
              "element":{{"type":"balklaag","hoh_mm":400}},
              "sterkteklasse":"C24"{extra}}}"#
        ))
        .unwrap()
    }

    fn toets<'a>(k: &'a Kandidaat, id: &str) -> &'a Toets {
        k.toetsen
            .iter()
            .find(|t| t.id == id)
            .unwrap_or_else(|| panic!("toets {id} ontbreekt"))
    }

    #[test]
    fn vloerplaat_zonder_vloerbreedte_spreidt_niet() {
        let r = bereken_hout(&invoer(
            r#","vloerplaat":{"dikte_mm":18,"e_mean_n_mm2":4000}"#,
            "",
        ))
        .unwrap();
        for k in &r.kandidaten {
            assert_relative_eq!(k.tussenwaarden["b_ef"], 0.4, epsilon = 1e-12);
            assert!(matches!(
                toets(k, "trilling_v").status,
                ToetsStatus::NietGetoetst {
                    blokkeert_advies: false,
                    ..
                }
            ));
        }
    }

    #[test]
    fn trap_en_balkon_krijgen_geen_trillingstoets() {
        for functie in ["woon_trap", "woon_balkon"] {
            let r =
                bereken_hout(&invoer("", &format!(r#","gebruiksfunctie":"{functie}""#))).unwrap();
            let k = &r.kandidaten[0];
            assert!(k.toetsen.iter().all(|t| t.id != "trilling_f1"));
            assert!(matches!(
                toets(k, "trilling").status,
                ToetsStatus::NietGetoetst {
                    blokkeert_advies: false,
                    ..
                }
            ));
        }
    }

    #[test]
    fn stijve_plaat_geeft_eigen_reden_en_k_r_melding() {
        // 60 mm, E 30000: EI_T = 540 kNm2/m, groter dan EI_L van 71x146 (ca. 506) en k_r op de ondergrens.
        let r = bereken_hout(&invoer(
            r#","vloerplaat":{"dikte_mm":60,"e_mean_n_mm2":30000},"vloerbreedte_m":4.0"#,
            "",
        ))
        .unwrap();
        let k = r.kandidaten.iter().find(|k| k.hoogte_mm == 146.0).unwrap();
        match &toets(k, "trilling_v").status {
            ToetsStatus::NietGetoetst { reden, .. } => assert!(reden.contains("stijver")),
            s => panic!("verwacht niet getoetst, kreeg {s:?}"),
        }
        assert!(r
            .meldingen
            .iter()
            .any(|m| m.tekst.starts_with("k_r valt op de ondergrens")));
    }

    #[test]
    fn duurklassen_en_k_mod() {
        assert_eq!(duur_veranderlijk(Leidend::Vloer), Duur::Middellang);
        assert_eq!(duur_veranderlijk(Leidend::Sneeuw), Duur::Kort);
        // Categorie H volgt het duurcriterium: kort, k_mod 0,90 voor klimaatklasse 1.
        assert_eq!(duur_veranderlijk(Leidend::DakH), Duur::Kort);
        assert_relative_eq!(k_mod(Duur::Kort), 0.90);
        assert_relative_eq!(k_mod(Duur::Blijvend), 0.60);
    }

    #[test]
    fn dak_h_en_sneeuw_hebben_k_mod_kort() {
        // Plat dak: sneeuw 0,56 < H 1,0 -> categorie H leidend.
        let i = invoer("", r#","toepassing":"dak","uiterlijk_van_belang":false"#);
        let r = bereken_hout(&i).unwrap();
        let k = &r.kandidaten[0];
        assert_relative_eq!(k.tussenwaarden["k_mod_gq"], 0.90);
        // Hellend dak: sneeuw leidend.
        let i = invoer("", r#","toepassing":"dak","dak_hellingshoek_graden":40"#);
        let r = bereken_hout(&i).unwrap();
        assert_relative_eq!(r.kandidaten[0].tussenwaarden["k_mod_gq"], 0.90);
        // Dak: geen trillingstoets.
        assert!(r.kandidaten[0]
            .toetsen
            .iter()
            .all(|t| !t.id.starts_with("trilling")));
    }

    #[test]
    fn b_ef_ondergrens_is_hoh() {
        // Zeer slappe plaat: 0,95 l ((EI)_T/(EI)_L)^0,25 < hoh -> b_ef = hoh.
        let i = invoer(
            r#","vloerplaat":{"dikte_mm":5,"e_mean_n_mm2":100},"vloerbreedte_m":4.0"#,
            "",
        );
        let r = bereken_hout(&i).unwrap();
        assert_relative_eq!(r.kandidaten[3].tussenwaarden["b_ef"], 0.4, epsilon = 1e-12);
    }

    #[test]
    fn lage_eigenfrequentie_blokkeert() {
        let r = bereken_hout(&invoer("", "")).unwrap();
        let k = &r.kandidaten[0];
        assert_eq!(k.hoogte_mm, 146.0);
        let f1 = k.tussenwaarden["f_1"];
        assert!((f1 - 7.25).abs() < 0.05, "f_1 = {f1}");
        assert!(matches!(
            toets(k, "trilling_f1").status,
            ToetsStatus::NietGetoetst {
                blokkeert_advies: true,
                ..
            }
        ));
        assert!(!k.voldoet());
    }

    #[test]
    fn kantoor_trilling_niet_getoetst_niet_blokkerend() {
        let i = invoer("", r#","gebruiksfunctie":"kantoor""#);
        let r = bereken_hout(&i).unwrap();
        let k = &r.kandidaten[2];
        let t = toets(k, "trilling");
        assert!(matches!(
            &t.status,
            ToetsStatus::NietGetoetst { reden, blokkeert_advies: false }
                if reden.contains("buiten woonfunctie") && !reden.contains("O9")
        ));
        assert!(!t.blokkeert());
        assert!(k.toetsen.iter().all(|t| t.id != "trilling_w1kn"));
    }

    #[test]
    fn w_1kn_boven_een_blokkeert_ondanks_aanname() {
        let r = bereken_hout(&invoer("", "")).unwrap();
        // 171 mm: w_1kN (een balk) is groter dan 1 mm/kN.
        let k = &r.kandidaten[1];
        let t = toets(k, "trilling_w1kn");
        assert!(t.uc.unwrap() > 1.0);
        assert!(t.blokkeert());
        assert!(!k.voldoet());
        // Het advies ligt hoger dan 171 mm.
        assert!(r.advies.as_ref().unwrap().hoogte_mm > 171.0);
    }

    #[test]
    fn aanname_tekst_zonder_plaat_en_met_plaat() {
        let r = bereken_hout(&invoer("", "")).unwrap();
        let t = toets(&r.kandidaten[6], "trilling_w1kn");
        assert!(
            matches!(&t.status, ToetsStatus::Aanname { tekst } if tekst.contains("b_ef = hoh"))
        );
        let i = invoer(
            r#","vloerplaat":{"dikte_mm":18,"e_mean_n_mm2":4000},"vloerbreedte_m":4.0"#,
            "",
        );
        let r = bereken_hout(&i).unwrap();
        let t = toets(&r.kandidaten[6], "trilling_w1kn");
        assert!(matches!(&t.status, ToetsStatus::Aanname { tekst } if tekst.contains("9.3.2.5")));
    }

    #[test]
    fn kip_niet_gesteund_geeft_uc() {
        let i = invoer(r#","drukrand_gesteund":false"#, "");
        let r = bereken_hout(&i).unwrap();
        let k = &r.kandidaten[2];
        let t = toets(k, "kip");
        assert!(t.uc.is_some());
        assert!(matches!(
            t.status,
            ToetsStatus::Voldoet | ToetsStatus::VoldoetNiet
        ));
        assert!(k.tussenwaarden.contains_key("k_crit"));
        // Gesteund: aanname.
        let r = bereken_hout(&invoer("", "")).unwrap();
        assert!(matches!(
            &toets(&r.kandidaten[2], "kip").status,
            ToetsStatus::Aanname { tekst } if tekst.contains("drukrand doorgaand gesteund")
        ));
    }

    #[test]
    fn klimaatklasse_2_geeft_k_def_0_8() {
        let r = bereken_hout(&invoer(r#","klimaatklasse":2"#, "")).unwrap();
        assert_relative_eq!(r.kandidaten[0].tussenwaarden["k_def"], 0.80);
        let r = bereken_hout(&invoer("", "")).unwrap();
        assert_relative_eq!(r.kandidaten[0].tussenwaarden["k_def"], 0.60);
        assert!(bereken_hout(&invoer(r#","klimaatklasse":3"#, "")).is_err());
    }

    #[test]
    fn vervangbare_klassendata() {
        let mut klassen = standaard_klassen().unwrap();
        assert_eq!(klassen.len(), 8);
        klassen.retain(|k| k.naam != "C24");
        assert!(bereken_hout_met_klassen(&invoer("", ""), &klassen).is_err());
    }
}
