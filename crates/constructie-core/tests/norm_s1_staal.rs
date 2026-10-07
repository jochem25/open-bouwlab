//! Golden S1: stalen ligger, S235, IPE en HEA, l = 5,40 m, belastingbreedte 3,60 m.
//!
//! Tolerantie: 0,005 op UC; 0,5 % op overige waarden, aangevuld met de halve eenheid
//! van het laatst genoteerde cijfer in de golden (de golden zijn afgerond genoteerd).

use constructie_core::{bereken_staal, StaalInvoer};

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

fn invoer() -> StaalInvoer {
    serde_json::from_str(
        r#"{
        "algemeen": { "overspanning_m": 5.4, "permanent_kn_m2": 0.75,
                      "gevolgklasse": "CC2" },
        "belastingbreedte_m": 3.6, "staalsoort": "S235", "reeksen": ["IPE", "HEA"] }"#,
    )
    .unwrap()
}

// naam, q_d, M_Ed, M_pl,Rd, UC M, V_Ed, V_pl,Rd, UC V, w_3 (UC), w_max (UC), w_qp (UC)
type Rij = (
    &'static str,
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
const TABEL: [Rij; 7] = [
    (
        "IPE 180", 12.92, 47.08, 39.10, 1.20, 34.9, 152.7, 0.23, 12.6, 19.1, 19.1,
    ),
    // w_3 = 8,5: golden gecorrigeerd door de planner (was 8,6, afrondingsfout).
    (
        "IPE 200", 12.96, 47.23, 51.84, 0.91, 35.0, 189.9, 0.18, 8.5, 13.1, 13.1,
    ),
    (
        "IPE 220", 13.00, 47.40, 67.07, 0.71, 35.1, 215.5, 0.16, 6.0, 9.2, 9.2,
    ),
    (
        "IPE 240", 13.06, 47.60, 86.15, 0.55, 35.3, 259.8, 0.14, 4.3, 6.6, 6.6,
    ),
    (
        "HEA 140", 12.99, 47.33, 40.77, 1.16, 35.1, 137.4, 0.26, 16.1, 24.7, 24.7,
    ),
    (
        "HEA 160", 13.06, 47.59, 57.60, 0.83, 35.3, 179.2, 0.20, 9.9, 15.4, 15.4,
    ),
    (
        "HEA 180", 13.12, 47.81, 76.35, 0.63, 35.4, 196.3, 0.18, 6.6, 10.4, 10.4,
    ),
];

#[test]
fn s1_per_profiel() {
    let r = bereken_staal(&invoer()).unwrap();
    for (naam, qd, m, mrd, ucm, v, vrd, ucv, w3, wmax, wqp) in TABEL {
        let k = r
            .kandidaten
            .iter()
            .find(|k| k.naam == naam)
            .unwrap_or_else(|| panic!("{naam} ontbreekt"));
        let t = &k.tussenwaarden;
        golden(&format!("{naam} q_d"), t["q_d"], qd, 2);
        golden(&format!("{naam} M_Ed"), t["m_ed"], m, 2);
        golden(&format!("{naam} M_pl,Rd"), t["m_pl_rd"], mrd, 2);
        golden(&format!("{naam} V_Ed"), t["v_ed"], v, 1);
        golden(&format!("{naam} V_pl,Rd"), t["v_pl_rd"], vrd, 1);
        golden(&format!("{naam} w_3"), t["w_3"], w3, 1);
        golden(&format!("{naam} w_max"), t["w_max"], wmax, 1);
        golden(&format!("{naam} w_qp"), t["w_qp"], wqp, 1);

        let uc_van = |id: &str| {
            k.toetsen
                .iter()
                .find(|t| t.id == id)
                .and_then(|t| t.uc)
                .unwrap()
        };
        uc(&format!("{naam} UC M"), uc_van("buiging"), ucm);
        uc(&format!("{naam} UC V"), uc_van("dwarskracht"), ucv);
        assert!(t["v_ed"] / t["v_pl_rd"] < 0.5, "{naam}: M-V interactie");
        assert_eq!(t["klasse_lijf"], 1.0);
    }
}

#[test]
fn s1_uc_doorbuiging_en_trilling() {
    let r = bereken_staal(&invoer()).unwrap();
    // (naam, UC w_3, UC w_max, UC w_qp) uit de golden.
    let uc_tabel = [
        ("IPE 200", 0.53, 0.60, 0.38),
        ("IPE 220", 0.37, 0.43, 0.27),
        ("IPE 240", 0.26, 0.31, 0.20),
        ("HEA 140", 0.99, 1.14, 0.73),
        ("HEA 160", 0.61, 0.71, 0.45),
        ("HEA 180", 0.41, 0.48, 0.31),
    ];
    for (naam, a, b, c) in uc_tabel {
        let k = r.kandidaten.iter().find(|k| k.naam == naam).unwrap();
        let uc_van = |id: &str| {
            k.toetsen
                .iter()
                .find(|t| t.id == id)
                .and_then(|t| t.uc)
                .unwrap()
        };
        uc(&format!("{naam} w2+w3"), uc_van("doorbuiging_w2w3"), a);
        uc(&format!("{naam} w_max"), uc_van("doorbuiging_wmax"), b);
        uc(&format!("{naam} trilling"), uc_van("trilling"), c);
    }
}

#[test]
fn s1_advies_en_laagste_bouwhoogte() {
    let r = bereken_staal(&invoer()).unwrap();
    let a = r.advies.as_ref().expect("advies");
    assert_eq!(a.naam, "IPE 200");
    let m = a.maatgevend.as_ref().unwrap();
    assert_eq!(m.toets_id, "buiging");
    uc("maatgevend", m.uc, 0.91);

    let l = r.laagste_bouwhoogte.as_ref().expect("laagste bouwhoogte");
    assert_eq!(l.naam, "HEA 160");
    assert_eq!(l.hoogte_mm, 152.0);
    uc(
        "laagste bouwhoogte",
        l.maatgevend.as_ref().unwrap().uc,
        0.83,
    );

    assert_eq!(r.alternatieven.len(), 4);
    assert_eq!(r.alternatieven[0].naam, "IPE 180");
    assert!(!r.alternatieven[0].voldoet);
}
