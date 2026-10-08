//! Staal: toetsen van gewalste I-, H- en U-profielen als vrij opgelegde ligger (EC3).
//!
//! Eenheden in deze module: mm, N/mm2, kN en kNm waar vermeld. Profieldata wordt als
//! gegevens geladen (`data/staal_profielen.json`) en kan worden vervangen zonder
//! codewijziging via [`bereken_staal_met_profielen`].

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::advies::stel_resultaat_samen;
use crate::belasting::{bepaal_belasting, Belasting, GEWICHT_STAAL};
use crate::error::{ConstructieFout, Result};
use crate::model::{Kandidaat, Reeks, StaalInvoer, StaalResultaat, Staalsoort};
use crate::toets::{Bron, Melding, Toets};

/// E-modulus staal in N/mm2 (EC3 3.2.6(1)).
const E_STAAL: f64 = 210_000.0;
/// Dichtheid staal in kg/m3 voor de massa per meter.
const DICHTHEID_STAAL: f64 = 7850.0;
/// Grenswaarde voor de trillingstoets: w_inst,qp in mm (EC0 NB A1.4.4).
const TRILLING_GRENS_MM: f64 = 34.0;

/// Vaste waarschuwing bij U-profielen (scope v1: zijdelingse steun, belasting via het lijf).
pub const UNP_WAARSCHUWING: &str =
    "U-profiel: wringing en kip niet getoetst - alleen bij zijdelingse steun, anders constructeur";
/// Toelichting op de elastische buigweerstand van U-profielen.
pub const UNP_ELASTISCH: &str = "U-profiel: buigweerstand elastisch (W_el,y). De plastische W_pl,y uit de \
fabrikantentabel geldt alleen als wringing is uitgesloten (bijv. twee profielen gekoppeld).";

/// Profielrecord met nominale maten. Eenheden: mm, mm2, mm4, mm3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Profiel {
    /// Naam, bijv. "IPE 200".
    pub naam: String,
    /// Reeks.
    pub reeks: Reeks,
    /// Hoogte h.
    pub h: f64,
    /// Flensbreedte b.
    pub b: f64,
    /// Lijfdikte t_w.
    pub t_w: f64,
    /// Flensdikte t_f.
    pub t_f: f64,
    /// Ronding r.
    pub r: f64,
    /// Oppervlak A.
    #[serde(rename = "A")]
    pub a: f64,
    /// Traagheidsmoment om de sterke as I_y.
    #[serde(rename = "I_y")]
    pub i_y: f64,
    /// Plastisch weerstandsmoment W_pl,y.
    #[serde(rename = "W_pl_y")]
    pub w_pl_y: f64,
    /// Elastisch weerstandsmoment W_el,y (verplicht voor U-profielen).
    #[serde(rename = "W_el_y", default, skip_serializing_if = "Option::is_none")]
    pub w_el_y: Option<f64>,
    /// Traagheidsmoment om de zwakke as I_z.
    #[serde(rename = "I_z", default, skip_serializing_if = "Option::is_none")]
    pub i_z: Option<f64>,
    /// Tweede afrondingsstraal r_2 (flenstip, U-profielen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r2: Option<f64>,
    /// Massa per meter G in kg/m volgens de bron.
    #[serde(rename = "G", default, skip_serializing_if = "Option::is_none")]
    pub g: Option<f64>,
    /// Afschuifoppervlak A_vz volgens de bron (ter vergelijking; de toets rekent A_v zelf).
    #[serde(rename = "A_vz", default, skip_serializing_if = "Option::is_none")]
    pub a_vz: Option<f64>,
    /// Herkomst van de gegevens (bron en versie; geldt voor elke waarde in het record).
    pub bron: String,
}

impl Profiel {
    /// U-profiel (hellende flenzen, asymmetrisch om de z-as).
    fn is_u_profiel(&self) -> bool {
        self.reeks == Reeks::Unp
    }
}

/// Standaard profieldata (voorlopig, bron volgt).
pub fn standaard_profielen() -> Result<Vec<Profiel>> {
    serde_json::from_str(include_str!("../data/staal_profielen.json"))
        .map_err(|e| ConstructieFout::Data(e.to_string()))
}

/// Voordimensionering van een stalen ligger met de ingebouwde profieldata.
pub fn bereken_staal(invoer: &StaalInvoer) -> Result<StaalResultaat> {
    let profielen = standaard_profielen()?;
    bereken_staal_met_profielen(invoer, &profielen)
}

/// Voordimensionering met eigen profieldata.
pub fn bereken_staal_met_profielen(
    invoer: &StaalInvoer,
    profielen: &[Profiel],
) -> Result<StaalResultaat> {
    invoer.valideer()?;
    if let Some(p) = profielen.iter().find(|p| p.is_u_profiel() && p.w_el_y.is_none()) {
        return Err(ConstructieFout::Data(format!(
            "{}: W_el_y ontbreekt (verplicht voor U-profielen)",
            p.naam
        )));
    }
    let bel = bepaal_belasting(&invoer.algemeen)?;
    let mut meldingen = bel.meldingen.clone();

    let mut gekozen: Vec<&Profiel> = Vec::new();
    let mut gezien: Vec<Reeks> = Vec::new();
    for reeks in &invoer.reeksen {
        if gezien.contains(reeks) {
            continue;
        }
        gezien.push(*reeks);
        let n = gekozen.len();
        gekozen.extend(profielen.iter().filter(|p| p.reeks == *reeks));
        if gekozen.len() == n {
            meldingen.push(Melding::waarschuwing(&format!(
                "Reeks {} bevat geen profielen in de profieldata; deze reeks is niet beschouwd.",
                reeks.naam()
            )));
        }
    }
    if gekozen.iter().any(|p| p.is_u_profiel()) {
        meldingen.push(Melding::waarschuwing(UNP_WAARSCHUWING));
        meldingen.push(Melding::info(UNP_ELASTISCH));
    }
    if gekozen.iter().any(|p| p.bron.contains("VOORLOPIG")) {
        meldingen.push(Melding::waarschuwing(
            "Profieldata is VOORLOPIG (nominale waarden, bron volgt).",
        ));
    }
    // Lichtste profiel eerst.
    gekozen.sort_by(|a, b| a.a.partial_cmp(&b.a).unwrap_or(std::cmp::Ordering::Equal));

    let kandidaten: Vec<Kandidaat> = gekozen
        .iter()
        .map(|p| toets_profiel(p, invoer, &bel))
        .collect();

    Ok(stel_resultaat_samen(
        kandidaten,
        bel.gevolgklasse,
        bel.l_m,
        meldingen,
        true,
    ))
}

/// Vloeigrens f_y in N/mm2 bij grootste plaatdikte `t` (EC3 tabel 3.1).
fn vloeigrens(staalsoort: Staalsoort, t: f64) -> f64 {
    let (dun, dik) = match staalsoort {
        Staalsoort::S235 => (235.0, 215.0),
        Staalsoort::S275 => (275.0, 255.0),
        Staalsoort::S355 => (355.0, 335.0),
    };
    if t <= 40.0 {
        dun
    } else {
        dik
    }
}

/// Klasse 1, 2 of 3 uit c/t en de grenzen voor klasse 1-3; 4.0 als ook klasse 3 niet wordt gehaald.
fn klasse_uit_grenzen(ct: f64, grenzen: [f64; 3]) -> f64 {
    grenzen
        .iter()
        .position(|&g| ct <= g)
        .map_or(4.0, |i| (i + 1) as f64)
}

fn toets_profiel(p: &Profiel, invoer: &StaalInvoer, bel: &Belasting) -> Kandidaat {
    let alg = &invoer.algemeen;
    let fy = vloeigrens(invoer.staalsoort, p.t_f);
    let eps = (235.0 / fy).sqrt();
    let l_mm = bel.l_m * 1000.0;
    let breedte = invoer.belastingbreedte_m;

    let eigen_gewicht = if alg.eigen_gewicht_automatisch {
        p.a * 1e-6 * GEWICHT_STAAL
    } else {
        0.0
    };
    let g = bel.g_k * breedte + eigen_gewicht;
    let q = bel.q_k * breedte;
    let ugt = bel.ugt(g, q);

    let mut toetsen: Vec<Toets> = Vec::new();
    let mut meldingen: Vec<Melding> = Vec::new();
    let mut tw: BTreeMap<String, f64> = BTreeMap::new();
    tw.insert("f_y".into(), fy);
    tw.insert("epsilon".into(), eps);
    tw.insert("g_lijn".into(), g);
    tw.insert("q_lijn".into(), q);
    tw.insert("q_d_6.10a".into(), ugt.q_d_a);
    tw.insert("q_d_6.10b".into(), ugt.q_d_b);
    tw.insert("q_d".into(), ugt.q_d);
    tw.insert("m_ed_gelijkmatig".into(), ugt.m_gelijkmatig);
    tw.insert("m_ed_punt".into(), ugt.m_punt);
    tw.insert("m_ed".into(), ugt.m_ed);
    tw.insert("v_ed_gelijkmatig".into(), ugt.v_gelijkmatig);
    tw.insert("v_ed_punt".into(), ugt.v_punt);
    tw.insert("v_ed".into(), ugt.v_ed);

    if p.t_f > 80.0 {
        toetsen.push(
            Toets::nieuw(
                "plaatdikte",
                "Plaatdikte",
                "t <= 80 mm",
                Bron::basis("EC3", "tabel 3.1"),
            )
            .niet_getoetst("plaatdikte groter dan 80 mm: buiten scope v1", true),
        );
    }

    // Dwarsdoorsnedeklasse (EC3 tabel 5.2): lijf inwendig deel op buiging (blad 1),
    // flens uitstekend deel op druk (blad 2). Klasse 3/4 valt buiten scope v1.
    // U-profiel: de flens steekt aan een kant uit, c gemeten vanaf het lijf; t_f is de
    // tabelwaarde (dikte op de halve flensbreedte bij de hellende flens).
    let unp = p.is_u_profiel();
    let c_lijf = p.h - 2.0 * p.t_f - 2.0 * p.r;
    let ct_lijf = c_lijf / p.t_w;
    let c_flens = if unp {
        p.b - p.t_w - p.r
    } else {
        (p.b - p.t_w - 2.0 * p.r) / 2.0
    };
    let ct_flens = c_flens / p.t_f;
    tw.insert("c_t_lijf".into(), ct_lijf);
    tw.insert("c_t_flens".into(), ct_flens);
    let klasse_lijf = klasse_uit_grenzen(ct_lijf, [72.0 * eps, 83.0 * eps, 124.0 * eps]);
    let klasse_flens = klasse_uit_grenzen(ct_flens, [9.0 * eps, 10.0 * eps, 14.0 * eps]);
    tw.insert("klasse_lijf".into(), klasse_lijf);
    tw.insert("klasse_flens".into(), klasse_flens);
    for (id, naam, formule, ct, klasse, grens_kl2) in [
        (
            "klasse_lijf",
            "Dwarsdoorsnedeklasse lijf",
            "c/t <= 72 eps (klasse 1), <= 83 eps (klasse 2)",
            ct_lijf,
            klasse_lijf,
            83.0 * eps,
        ),
        (
            "klasse_flens",
            "Dwarsdoorsnedeklasse flens",
            "c/t <= 9 eps (klasse 1), <= 10 eps (klasse 2)",
            ct_flens,
            klasse_flens,
            10.0 * eps,
        ),
    ] {
        let mut t = Toets::nieuw(id, naam, formule, Bron::basis("EC3", "tabel 5.2"))
            .waarden(ct, grens_kl2, "-");
        if klasse > 2.0 {
            t = t.niet_getoetst(
                "klasse 3 of 4: elastische of effectieve doorsnede buiten scope v1 - constructeur",
                true,
            );
        }
        toetsen.push(t);
    }

    // Dwarskracht (EC3 6.2.6(2) formule 6.18, eta = 1,0 veilige kant). A_v volgens
    // 6.2.6(3)a (gewalste I/H) of 6.2.6(3)b (gewalste U-profielen, zonder eta-ondergrens).
    let eta = 1.0;
    let h_w = p.h - 2.0 * p.t_f;
    let a_v = if unp {
        p.a - 2.0 * p.b * p.t_f + (p.t_w + p.r) * p.t_f
    } else {
        (p.a - 2.0 * p.b * p.t_f + (p.t_w + 2.0 * p.r) * p.t_f).max(eta * h_w * p.t_w)
    };
    let v_rd = a_v * fy / 3f64.sqrt() / 1e3;
    tw.insert("a_v".into(), a_v);
    tw.insert("v_pl_rd".into(), v_rd);

    // Interactie M-V (EC3 6.2.8(2)-(3), formules 6.29 en 6.30). Conservatief: V_Ed bij de
    // oplegging gecombineerd met M_Ed in het veld.
    // U-profiel: elastische buigweerstand (W_el,y, 6.2.5 (6.14)); bij V_Ed > 0,5 V_pl,Rd
    // conservatief (1 - rho) M_el,Rd, want (6.30) geldt voor I-doorsneden.
    let ratio_v = ugt.v_ed / v_rd;
    let rho = if ratio_v <= 0.5 {
        0.0
    } else {
        (2.0 * ratio_v - 1.0).powi(2)
    };
    let m_rd = if unp {
        let m_el_rd = p.w_el_y.unwrap_or(0.0) * fy / 1e6;
        tw.insert("m_el_rd".into(), m_el_rd);
        (1.0 - rho) * m_el_rd
    } else {
        let m_pl_rd = p.w_pl_y * fy / 1e6;
        tw.insert("m_pl_rd".into(), m_pl_rd);
        if rho > 0.0 {
            let a_w = h_w * p.t_w;
            let m_v_rd = (p.w_pl_y - rho * a_w * a_w / (4.0 * p.t_w)) * fy / 1e6;
            m_v_rd.min(m_pl_rd)
        } else {
            m_pl_rd
        }
    };
    tw.insert("rho_mv".into(), rho);
    tw.insert("m_rd".into(), m_rd);
    let mv = Toets::nieuw(
        "interactie_mv",
        "Interactie moment en dwarskracht",
        if unp {
            "V_Ed <= 0,5 V_pl,Rd: geen reductie; anders (1 - rho) M_el,Rd"
        } else {
            "V_Ed <= 0,5 V_pl,Rd: geen reductie; anders M_y,V,Rd"
        },
        if unp {
            Bron::basis("EC3", "6.2.8(3)")
        } else {
            Bron::basis("EC3", "6.2.8 (6.29), (6.30)")
        },
    )
    .waarden(ratio_v, 0.5, "-");
    toetsen.push(if rho > 0.0 {
        mv.aanname(if unp {
            "V_Ed > 0,5 V_pl,Rd: buigweerstand conservatief gereduceerd tot (1 - rho) M_el,Rd"
        } else {
            "V_Ed > 0,5 V_pl,Rd: buigweerstand gereduceerd tot M_y,V,Rd"
        })
    } else {
        mv
    });

    // Buiging (EC3 6.2.5; I/H klasse 1/2 plastisch (6.13), U-profiel elastisch (6.14)).
    toetsen.push(
        Toets::nieuw(
            "buiging",
            "Buiging",
            match (unp, rho > 0.0) {
                (true, true) => "M_Ed <= (1 - rho) M_el,Rd",
                (true, false) => "M_Ed <= M_el,Rd",
                (false, true) => "M_Ed <= M_y,V,Rd",
                (false, false) => "M_Ed <= M_pl,Rd",
            },
            if unp {
                Bron::basis("EC3", "6.2.5 (6.14)")
            } else {
                Bron::basis("EC3", "6.2.5 (6.13)")
            },
        )
        .waarden(ugt.m_ed, m_rd, "kNm")
        .uc_uit_waarden(),
    );

    toetsen.push(
        Toets::nieuw(
            "dwarskracht",
            "Dwarskracht",
            "V_Ed <= V_pl,Rd",
            if unp {
                Bron::basis("EC3", "6.2.6 (6.18), A_v 6.2.6(3)b")
            } else {
                Bron::basis("EC3", "6.2.6 (6.18)")
            },
        )
        .waarden(ugt.v_ed, v_rd, "kN")
        .uc_uit_waarden(),
    );

    // Lijfplooi (EC3 6.2.6(6), formule 6.22): toetsen volgens EN 1993-1-5 nodig als
    // h_w/t_w > 72 eps/eta. Die toets zit niet in v1.
    let hw_tw = h_w / p.t_w;
    let grens_plooi = 72.0 * eps / eta;
    tw.insert("h_w_t_w".into(), hw_tw);
    let plooi = Toets::nieuw(
        "lijfplooi",
        "Plooi van het lijf",
        "h_w/t_w <= 72 eps/eta",
        Bron::basis("EC3", "6.2.6(6) (6.22)"),
    )
    .waarden(hw_tw, grens_plooi, "-");
    toetsen.push(if hw_tw > grens_plooi {
        plooi.niet_getoetst(
            "h_w/t_w > 72 eps/eta: plooitoets (EN 1993-1-5) buiten scope v1 - constructeur",
            true,
        )
    } else {
        plooi
    });

    // Kip (EC3 6.3.2.1(2)).
    let kip = Toets::nieuw(
        "kip",
        "Kip",
        "drukflens zijdelings gesteund",
        Bron::basis("EC3", "6.3.2.1(2)"),
    );
    toetsen.push(if invoer.bovenflens_gesteund {
        kip.aanname("kip niet getoetst: bovenflens doorgaand zijdelings gesteund aangenomen")
    } else {
        kip.niet_getoetst("bovenflens niet gesteund: kip buiten scope v1", true)
    });

    // Doorbuiging (EC0 NB A1.4.3, k_def staal = 0).
    let c = 5.0 * l_mm.powi(4) / (384.0 * E_STAAL * p.i_y);
    let w_g = g * c;
    let w_q = q * c;
    let eis = bel.doorbuigingseis();
    let w3 = if eis.w3_met_psi1 { bel.psi1 * w_q } else { w_q };
    let w2 = 0.0;
    let w_max = w_g + bel.psi2 * w_q;
    tw.insert("w_g".into(), w_g);
    tw.insert("w_q".into(), w_q);
    tw.insert("w_3".into(), w3);
    tw.insert("w_max".into(), w_max);
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
    if alg.uiterlijk_van_belang {
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

    // Trilling (alleen vloeren) en afschot (alleen daken).
    if bel.vloergedrag() {
        let trilling_bron = Bron::nb("EC0", "A1.4.4");
        if let Some((waarde, via)) = bel.trilling_massa(g, q) {
            let (grens, eenheid) = if via.contains("150") {
                (150.0, "kN")
            } else {
                (5.0, "kN/m2")
            };
            toetsen.push(
                Toets::nieuw(
                    "trilling",
                    "Trilling",
                    "g_k + psi_2 q_k >= 5 kN/m2 of totaal >= 150 kN",
                    trilling_bron,
                )
                .waarden(waarde, grens, eenheid),
            );
            meldingen.push(Melding::info(&format!(
                "Trilling: massacriterium gehaald ({via})."
            )));
        } else {
            tw.insert("w_qp".into(), w_max);
            toetsen.push(
                Toets::nieuw("trilling", "Trilling", "w_inst,qp <= 34 mm", trilling_bron)
                    .waarden(w_max, TRILLING_GRENS_MM, "mm")
                    .uc_uit_waarden(),
            );
        }
        meldingen.push(
            Melding::signalering(
                "Lichte vloer op stalen ligger: SBR-richtlijn trillingen, constructeur.",
            )
            .met_bron(Bron::nb("EC3", "7.2.3")),
        );
    }
    if bel.toepassing == crate::model::Toepassing::Dak {
        meldingen.push(bel.afschot_melding(w_g + w_q));
    }

    Kandidaat {
        naam: p.naam.clone(),
        hoogte_mm: p.h,
        eigen_gewicht_kn_m: eigen_gewicht,
        gewicht_kg_m: Some(p.a * 1e-6 * DICHTHEID_STAAL),
        toetsen,
        meldingen,
        afgewezen: None,
        tussenwaarden: tw,
        beton: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toets::ToetsStatus;

    fn invoer(json: &str) -> StaalInvoer {
        serde_json::from_str(json).unwrap()
    }

    const BASIS: &str = r#"{ "algemeen": { "overspanning_m": 5.4, "permanent_kn_m2": 0.75 },
        "belastingbreedte_m": 3.6, "staalsoort": "S235", "reeksen": ["IPE","HEB"] }"#;

    #[test]
    fn reeks_zonder_profielen_wordt_gemeld() {
        let r = bereken_staal(&invoer(BASIS)).unwrap();
        assert!(r
            .meldingen
            .iter()
            .any(|m| m.tekst.contains("HEB") && m.tekst.contains("geen profielen")));
        assert!(r.advies.is_some());
    }

    #[test]
    fn niet_gesteunde_bovenflens_blokkeert_advies() {
        let mut i = invoer(BASIS);
        i.bovenflens_gesteund = false;
        let r = bereken_staal(&i).unwrap();
        assert!(r.advies.is_none());
        assert!(r.laagste_bouwhoogte.is_none());
    }

    #[test]
    fn vloeigrens_neemt_plaatdikte_mee() {
        assert_eq!(vloeigrens(Staalsoort::S235, 40.0), 235.0);
        assert_eq!(vloeigrens(Staalsoort::S355, 41.0), 335.0);
    }

    #[test]
    fn dak_geeft_afschotsignalering_en_geen_trilling() {
        let i = invoer(
            r#"{ "algemeen": { "overspanning_m": 5.0, "permanent_kn_m2": 0.5, "toepassing": "dak" },
            "belastingbreedte_m": 3.0, "staalsoort": "S235", "reeksen": ["IPE"] }"#,
        );
        let r = bereken_staal(&i).unwrap();
        let a = r.advies.as_ref().unwrap();
        assert!(a.toetsen.iter().all(|t| t.id != "trilling"));
        assert!(r
            .meldingen
            .iter()
            .any(|m| m.tekst.contains("afwatering") || m.tekst.contains("Wateraccumulatie")));
    }

    #[test]
    fn eigen_profieldata_kan_worden_meegegeven() {
        let mut p = standaard_profielen().unwrap();
        p.truncate(1);
        let r = bereken_staal_met_profielen(&invoer(BASIS), &p).unwrap();
        assert_eq!(r.kandidaten.len(), 1);
    }

    fn toets<'a>(k: &'a Kandidaat, id: &str) -> &'a Toets {
        k.toetsen.iter().find(|t| t.id == id).unwrap()
    }

    #[test]
    fn gewalste_profielen_zijn_klasse_1_en_lijfplooi_voldoet() {
        let r = bereken_staal(&invoer(BASIS)).unwrap();
        for k in &r.kandidaten {
            assert_eq!(k.tussenwaarden["klasse_lijf"], 1.0, "{}", k.naam);
            assert_eq!(k.tussenwaarden["klasse_flens"], 1.0, "{}", k.naam);
            assert_eq!(toets(k, "lijfplooi").status, ToetsStatus::Voldoet);
        }
    }

    #[test]
    fn slanke_flens_is_klasse_4_en_blokkeert() {
        let mut p = standaard_profielen().unwrap();
        p.truncate(1);
        p[0].b = 300.0;
        let r = bereken_staal_met_profielen(&invoer(BASIS), &p).unwrap();
        let k = &r.kandidaten[0];
        assert_eq!(k.tussenwaarden["klasse_flens"], 4.0);
        assert!(toets(k, "klasse_flens").blokkeert());
        assert!(r.advies.is_none());
    }

    #[test]
    fn unp_flens_ct_vanaf_lijf_en_av_voor_u_profielen() {
        let json = r#"{ "algemeen": { "overspanning_m": 4.2, "permanent_kn_m2": 0.75 },
            "belastingbreedte_m": 2.4, "staalsoort": "S235", "reeksen": ["UNP"] }"#;
        let r = bereken_staal(&invoer(json)).unwrap();
        let k = r.kandidaten.iter().find(|k| k.naam == "UNP 200").unwrap();
        // UNP 200: b 75, t_w 8,5, t_f 11,5, r 11,5, A 3220.
        assert!((k.tussenwaarden["c_t_flens"] - (75.0 - 8.5 - 11.5) / 11.5).abs() < 1e-12);
        let a_v = 3220.0 - 2.0 * 75.0 * 11.5 + (8.5 + 11.5) * 11.5;
        assert!((k.tussenwaarden["a_v"] - a_v).abs() < 1e-9);
        assert!(toets(k, "dwarskracht").bron.artikel.contains("6.2.6(3)b"));
        assert!(toets(k, "buiging").formule.contains("M_el,Rd"));
    }

    #[test]
    fn hoge_dwarskracht_reduceert_unp_conservatief() {
        let json = r#"{ "algemeen": { "overspanning_m": 1.0, "permanent_kn_m2": 15.0 },
            "belastingbreedte_m": 10.0, "staalsoort": "S235", "reeksen": ["UNP"] }"#;
        let r = bereken_staal(&invoer(json)).unwrap();
        let k = r.kandidaten.iter().find(|k| k.naam == "UNP 80").unwrap();
        let t = &k.tussenwaarden;
        let ratio = t["v_ed"] / t["v_pl_rd"];
        assert!(ratio > 0.5, "ratio {ratio}");
        let rho = (2.0 * ratio - 1.0).powi(2);
        assert!((t["m_rd"] - (1.0 - rho) * t["m_el_rd"]).abs() < 1e-9);
    }

    #[test]
    fn unp_zonder_w_el_y_is_een_datafout() {
        let mut p: Vec<Profiel> = standaard_profielen()
            .unwrap()
            .into_iter()
            .filter(|p| p.reeks == Reeks::Unp)
            .collect();
        p[0].w_el_y = None;
        let json = r#"{ "algemeen": { "overspanning_m": 4.0, "permanent_kn_m2": 0.75 },
            "belastingbreedte_m": 2.0, "staalsoort": "S235", "reeksen": ["UNP"] }"#;
        assert!(matches!(
            bereken_staal_met_profielen(&invoer(json), &p),
            Err(ConstructieFout::Data(_))
        ));
    }

    #[test]
    fn i_profielen_krijgen_geen_unp_waarschuwing() {
        let r = bereken_staal(&invoer(BASIS)).unwrap();
        assert!(r.meldingen.iter().all(|m| m.tekst != UNP_WAARSCHUWING));
    }

    #[test]
    fn klasse_uit_grenzen_randen() {
        assert_eq!(klasse_uit_grenzen(9.0, [9.0, 10.0, 14.0]), 1.0);
        assert_eq!(klasse_uit_grenzen(9.5, [9.0, 10.0, 14.0]), 2.0);
        assert_eq!(klasse_uit_grenzen(14.0, [9.0, 10.0, 14.0]), 3.0);
        assert_eq!(klasse_uit_grenzen(14.1, [9.0, 10.0, 14.0]), 4.0);
    }

    #[test]
    fn hoge_dwarskracht_reduceert_buigweerstand() {
        let json = r#"{ "algemeen": { "overspanning_m": 1.0, "permanent_kn_m2": 15.0 },
            "belastingbreedte_m": 10.0, "staalsoort": "S235", "reeksen": ["IPE"] }"#;
        let r = bereken_staal(&invoer(json)).unwrap();
        let k = r.kandidaten.iter().find(|k| k.naam == "IPE 180").unwrap();
        let t = &k.tussenwaarden;
        let ratio = t["v_ed"] / t["v_pl_rd"];
        assert!(ratio > 0.5, "ratio {ratio}");
        let rho = (2.0 * ratio - 1.0).powi(2);
        assert!((t["rho_mv"] - rho).abs() < 1e-12);
        // (6.30) met h_w = h - 2 t_f, A_w = h_w t_w; IPE 180: h_w 164, t_w 5,3.
        let a_w: f64 = 164.0 * 5.3;
        let verwacht = (166.4e3 - rho * a_w * a_w / (4.0 * 5.3)) * 235.0 / 1e6;
        assert!((t["m_rd"] - verwacht).abs() < 1e-9);
        assert!(t["m_rd"] < t["m_pl_rd"]);
        assert!(matches!(
            toets(k, "interactie_mv").status,
            ToetsStatus::Aanname { .. }
        ));
        assert_eq!(toets(k, "buiging").grens, Some(t["m_rd"]));
    }
}
