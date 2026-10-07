//! Golden H1: houten balklaag, C24, hoh 400, b = 71 mm, l = 4,20 m, g_k = 0,75 kN/m2
//! (inclusief eigen gewicht), categorie A, CC2, klimaatklasse 1, drukrand gesteund.
//!
//! Variant A: zonder vloerplaat (b_ef = hoh). Variant B: vloerplaat 18 mm met
//! E = 4000 N/mm2 en vloerbreedte 4,0 m. De golden komen uit een onafhankelijke
//! controleberekening buiten de kern.
//!
//! Tolerantie: 0,005 op UC; 0,5 % op overige waarden, aangevuld met de halve eenheid
//! van het laatst genoteerde cijfer in de golden.

use constructie_core::model::Kandidaat;
use constructie_core::{bereken_hout, HoutInvoer};

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

fn invoer(vloerplaat: bool) -> HoutInvoer {
    let extra = if vloerplaat {
        r#","vloerplaat":{"dikte_mm":18,"e_mean_n_mm2":4000},"vloerbreedte_m":4.0"#
    } else {
        ""
    };
    serde_json::from_str(&format!(
        r#"{{"algemeen":{{"overspanning_m":4.2,"permanent_kn_m2":0.75,
            "eigen_gewicht_automatisch":false,"gevolgklasse":"CC2"}},
          "element":{{"type":"balklaag","hoh_mm":400}},
          "sterkteklasse":"C24","breedte_mm":71{extra}}}"#
    ))
    .unwrap()
}

fn uc_van(k: &Kandidaat, id: &str) -> f64 {
    k.toetsen
        .iter()
        .find(|t| t.id == id)
        .and_then(|t| t.uc)
        .unwrap_or_else(|| panic!("UC van {id} ontbreekt"))
}

fn waarde_van(k: &Kandidaat, id: &str) -> f64 {
    k.toetsen
        .iter()
        .find(|t| t.id == id)
        .and_then(|t| t.waarde)
        .unwrap_or_else(|| panic!("waarde van {id} ontbreekt"))
}

// h, k_r, M_d, UC M, V_d, UC V, w2+w3, UC, w_max, UC, f_1, b_ef, w_1kN
type RijA = (
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
);
const A: [RijA; 7] = [
    (
        146.0, 0.690, 4.054, 1.082, 5.100, 0.300, 13.12, 1.041, 16.32, 0.972, 7.25, 0.400, 7.620,
    ),
    (
        171.0, 0.690, 4.054, 0.793, 5.073, 0.255, 8.17, 0.648, 10.16, 0.605, 9.19, 0.400, 4.743,
    ),
    (
        196.0, 0.690, 4.054, 0.604, 5.046, 0.221, 5.42, 0.430, 6.75, 0.402, 11.27, 0.400, 3.150,
    ),
    (
        221.0, 0.690, 4.054, 0.475, 5.019, 0.195, 3.78, 0.300, 4.71, 0.280, 13.50, 0.400, 2.197,
    ),
    (
        246.0, 0.690, 4.054, 0.383, 4.992, 0.174, 2.74, 0.218, 3.41, 0.203, 15.85, 0.400, 1.593,
    ),
    (
        271.0, 0.690, 4.054, 0.316, 4.966, 0.157, 2.05, 0.163, 2.55, 0.152, 18.33, 0.400, 1.192,
    ),
    (
        296.0, 0.690, 4.054, 0.265, 4.939, 0.143, 1.57, 0.125, 1.96, 0.117, 20.92, 0.400, 0.914,
    ),
];

// h, k_r, M_d, UC M, f_1, b_ef, w_1kN, n_40, v, v-grens, UC v
type RijB = (f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64);
const B: [RijB; 7] = [
    (
        146.0, 0.651, 3.870, 1.033, 7.25, 0.993, 3.069, 8.91, 0.01549, 0.01179, 1.314,
    ),
    (
        171.0, 0.651, 3.870, 0.757, 9.19, 0.882, 2.151, 8.87, 0.01542, 0.01294, 1.192,
    ),
    (
        196.0, 0.651, 3.870, 0.576, 11.27, 0.796, 1.582, 8.80, 0.01531, 0.01430, 1.071,
    ),
    (
        221.0, 0.651, 3.870, 0.453, 13.50, 0.728, 1.208, 8.72, 0.01518, 0.01590, 0.955,
    ),
    (
        246.0, 0.651, 3.870, 0.366, 15.85, 0.672, 0.949, 8.61, 0.01500, 0.01780, 0.843,
    ),
    (
        271.0, 0.651, 3.870, 0.302, 18.33, 0.625, 0.763, 8.47, 0.01478, 0.02004, 0.738,
    ),
    (
        296.0, 0.651, 3.870, 0.253, 20.92, 0.585, 0.626, 8.30, 0.01450, 0.02269, 0.639,
    ),
];

#[test]
fn h1_variant_a_zonder_vloerplaat() {
    let r = bereken_hout(&invoer(false)).unwrap();
    assert_eq!(r.kandidaten.len(), 7);
    for (k, rij) in r.kandidaten.iter().zip(A.iter()) {
        let (h, kr, m, ucm, v, ucv, w23, ucw, wmax, ucwm, f1, bef, w1) = *rij;
        assert_eq!(k.hoogte_mm, h);
        let n = |s: &str| format!("A {h}: {s}");
        golden(&n("k_r"), k.tussenwaarden["k_r"], kr, 3);
        golden(&n("M_d"), k.tussenwaarden["m_d"], m, 3);
        uc(&n("UC buiging"), uc_van(k, "buiging"), ucm);
        golden(&n("V_d"), k.tussenwaarden["v_d"], v, 3);
        uc(&n("UC afschuiving"), uc_van(k, "afschuiving"), ucv);
        golden(&n("w2+w3"), waarde_van(k, "doorbuiging_w2w3"), w23, 2);
        uc(&n("UC w2+w3"), uc_van(k, "doorbuiging_w2w3"), ucw);
        golden(&n("w_max"), waarde_van(k, "doorbuiging_wmax"), wmax, 2);
        uc(&n("UC w_max"), uc_van(k, "doorbuiging_wmax"), ucwm);
        golden(&n("f_1"), k.tussenwaarden["f_1"], f1, 2);
        golden(&n("b_ef"), k.tussenwaarden["b_ef"], bef, 3);
        golden(&n("w_1kN"), k.tussenwaarden["w_1kn"], w1, 3);
        uc(&n("UC w_1kN"), uc_van(k, "trilling_w1kn"), w1);
    }
    let a = r.advies.expect("advies");
    assert_eq!(a.naam, "71 x 296");
    assert!(a.kengetallen.eigenfrequentie_hz.is_some());
}

#[test]
fn h1_variant_b_met_vloerplaat() {
    let r = bereken_hout(&invoer(true)).unwrap();
    for (k, rij) in r.kandidaten.iter().zip(B.iter()) {
        let (h, kr, m, ucm, f1, bef, w1, n40, v, vgr, ucv) = *rij;
        assert_eq!(k.hoogte_mm, h);
        let n = |s: &str| format!("B {h}: {s}");
        golden(&n("k_r"), k.tussenwaarden["k_r"], kr, 3);
        golden(&n("M_d"), k.tussenwaarden["m_d"], m, 3);
        uc(&n("UC buiging"), uc_van(k, "buiging"), ucm);
        golden(&n("f_1"), k.tussenwaarden["f_1"], f1, 2);
        golden(&n("b_ef"), k.tussenwaarden["b_ef"], bef, 3);
        golden(&n("w_1kN"), k.tussenwaarden["w_1kn"], w1, 3);
        uc(&n("UC w_1kN"), uc_van(k, "trilling_w1kn"), w1);
        golden(&n("n_40"), k.tussenwaarden["n_40"], n40, 2);
        golden(&n("v"), k.tussenwaarden["v"], v, 5);
        golden(&n("v-grens"), k.tussenwaarden["v_grens"], vgr, 5);
        uc(&n("UC v"), uc_van(k, "trilling_v"), ucv);
    }
    let a = r.advies.expect("advies");
    assert_eq!(a.naam, "71 x 246");
}
