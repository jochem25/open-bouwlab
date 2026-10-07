//! Golden B1: betonbalk C30/37, b = 300 mm, l = 6,00 m, belastingbreedte 3,00 m, g_k = 6,00 kN/m2.
//!
//! Tolerantie: 0,005 op UC; 0,5 % op overige waarden, aangevuld met de halve eenheid
//! van het laatst genoteerde cijfer in de golden (de golden zijn afgerond genoteerd).

use constructie_core::{bereken_beton, BetonInvoer};

fn golden(naam: &str, actual: f64, expected: f64, decimalen: i32) {
    let tol = (0.005 * expected.abs()).max(0.5 * 10f64.powi(-decimalen));
    assert!(
        (actual - expected).abs() <= tol,
        "{naam}: berekend {actual}, golden {expected}, tolerantie {tol}"
    );
}

fn uc(naam: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 0.005,
        "{naam}: UC {actual}, golden {expected}"
    );
}

fn invoer() -> BetonInvoer {
    serde_json::from_str(
        r#"{
        "algemeen": { "overspanning_m": 6.0, "permanent_kn_m2": 6.0, "gevolgklasse": "CC2" },
        "belastingbreedte_m": 3.0, "sterkteklasse": "C30/37", "balkbreedte_mm": 300,
        "milieuklasse": "XC1" }"#,
    )
    .unwrap()
}

// h, d, q_d, M_Ed, xi, UC xi, As_req, n, phi, l/d, grens, UC l/d, V_Ed, V_Rd,max, V_Rd,c, s
type Rij = (
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    u32,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
);
const TABEL: [Rij; 4] = [
    (
        400.0, 365.0, 33.08, 148.8, 0.260, 0.49, 1047.0, 4, 20.0, 16.44, 15.71, 1.05, 99.2, 359.0,
        74.4, 270.0,
    ),
    (
        450.0, 415.0, 33.53, 150.9, 0.198, 0.37, 908.0, 3, 20.0, 14.46, 17.17, 0.84, 100.6, 408.0,
        71.7, 300.0,
    ),
    (
        500.0, 465.0, 33.98, 152.9, 0.157, 0.29, 807.0, 3, 20.0, 12.90, 18.78, 0.69, 101.9, 457.0,
        75.6, 300.0,
    ),
    (
        550.0, 515.0, 34.43, 154.9, 0.128, 0.24, 729.0, 4, 16.0, 11.65, 21.66, 0.54, 103.3, 506.0,
        75.2, 300.0,
    ),
];

#[test]
fn b1_per_hoogte() {
    let r = bereken_beton(&invoer()).unwrap();
    for (h, d, qd, m, xi, ucxi, asr, n, phi, ld, gr, ucld, v, vmax, vc, s) in TABEL {
        let k = r
            .kandidaten
            .iter()
            .find(|k| k.hoogte_mm == h)
            .unwrap_or_else(|| panic!("h {h} ontbreekt"));
        let t = &k.tussenwaarden;
        let n_ = format!("h {h}");
        golden(&format!("{n_} d"), t["d"], d, 0);
        golden(&format!("{n_} q_d"), t["q_d"], qd, 2);
        golden(&format!("{n_} M_Ed"), t["m_ed"], m, 1);
        golden(&format!("{n_} x_u/d"), t["xi"], xi, 3);
        golden(&format!("{n_} A_s,req"), t["as_req"], asr, 0);
        golden(&format!("{n_} l/d"), t["l_d"], ld, 2);
        golden(&format!("{n_} grens"), t["l_d_grens"], gr, 2);
        golden(&format!("{n_} V_Ed"), t["v_ed"], v, 1);
        golden(&format!("{n_} V_Rd,max"), t["v_rd_max"], vmax, 0);
        golden(&format!("{n_} V_Rd,c"), t["v_rd_c"], vc, 1);
        assert_eq!(t["beugel_s"], s, "{n_} beugelafstand");

        let uc_van = |id: &str| {
            k.toetsen
                .iter()
                .find(|t| t.id == id)
                .and_then(|t| t.uc)
                .unwrap()
        };
        uc(&format!("{n_} UC x_u/d"), uc_van("buiging_xu_d"), ucxi);
        uc(&format!("{n_} UC l/d"), uc_van("slankheid"), ucld);

        let keuze = k.beton.as_ref().expect("wapeningskeuze");
        assert_eq!(keuze.aantal, n, "{n_} aantal staven");
        assert_eq!(keuze.diameter_mm, phi, "{n_} diameter");
        assert_eq!(keuze.beugel_s_mm, s);
    }
}

#[test]
fn b1_detail_bij_450() {
    let r = bereken_beton(&invoer()).unwrap();
    let k = r.kandidaten.iter().find(|k| k.hoogte_mm == 450.0).unwrap();
    let t = &k.tussenwaarden;
    golden("A_s,min", t["as_min"], 165.0, 0);
    golden("A_s,max", t["as_max"], 5400.0, 0);
    golden("A_sw/s nodig", t["asw_s_req"], 0.248, 3);
    golden("A_sw/s minimum", t["asw_s_min"], 0.263, 3);
    assert_eq!(t["s_l_max"], 300.0);
    golden("x_u/d grens", t["xi_grens"], 0.535, 3);
    // Trilling: g_k + psi_2 q_k = 6,53 >= 5 kN/m2.
    let tr = k.toetsen.iter().find(|t| t.id == "trilling").unwrap();
    assert!(matches!(
        tr.status,
        constructie_core::toets::ToetsStatus::Voldoet
    ));
    golden("trilling waarde", tr.waarde.unwrap(), 6.525, 3);
}

#[test]
fn b1_advies() {
    let r = bereken_beton(&invoer()).unwrap();
    // 400 voldoet niet op slankheid.
    let k400 = r.kandidaten.iter().find(|k| k.hoogte_mm == 400.0).unwrap();
    assert!(!k400.voldoet());
    let a = r.advies.as_ref().expect("advies");
    assert_eq!(a.naam, "300 x 450");
    let m = a.maatgevend.as_ref().unwrap();
    assert_eq!(m.toets_id, "slankheid");
    uc("maatgevend", m.uc, 0.84);
    let b = a.beton.as_ref().unwrap();
    assert_eq!((b.aantal, b.diameter_mm), (3, 20.0));
    assert_eq!((b.beugel_diameter_mm, b.beugel_s_mm), (8.0, 300.0));
    assert!(r.laagste_bouwhoogte.is_none());
}
