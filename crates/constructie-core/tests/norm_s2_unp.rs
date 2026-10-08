//! Golden S2: UNP-ligger, S235, l = 4,20 m, belastingbreedte 2,40 m, CC2, vloer woning.
//!
//! Golden-waarden uit het onafhankelijke controlescript `bouw_s2_unp_controle.py`
//! (deel 2). Tolerantie als S1: 0,005 op UC; 0,5 % op overige waarden, aangevuld met de
//! halve eenheid van het laatst genoteerde cijfer.

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
        "algemeen": { "overspanning_m": 4.2, "permanent_kn_m2": 0.75,
                      "gevolgklasse": "CC2" },
        "belastingbreedte_m": 2.4, "staalsoort": "S235", "reeksen": ["UNP"] }"#,
    )
    .unwrap()
}

// naam, q_d, M_Ed, M_el,Rd, UC M, V_Ed, A_v, V_pl,Rd, UC V, w_3, UC w_3, c/t flens
type Rij = (&'static str, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64);
const TABEL: [Rij; 5] = [
    ("UNP 120", 8.620, 19.007, 14.264, 1.332, 18.102, 854.0, 115.87, 0.156, 11.13, 0.883, 4.33),
    ("UNP 140", 8.652, 19.078, 20.304, 0.940, 18.170, 1010.0, 137.03, 0.133, 6.70, 0.532, 4.30),
    ("UNP 160", 8.686, 19.153, 27.260, 0.703, 18.241, 1224.0, 166.07, 0.110, 4.38, 0.348, 4.48),
    ("UNP 180", 8.724, 19.236, 35.250, 0.546, 18.320, 1469.0, 199.31, 0.092, 3.00, 0.238, 4.64),
    ("UNP 200", 8.763, 19.323, 44.885, 0.431, 18.403, 1725.0, 234.04, 0.079, 2.12, 0.168, 4.78),
];

#[test]
fn s2_per_profiel() {
    let r = bereken_staal(&invoer()).unwrap();
    for (naam, qd, m, mrd, ucm, v, av, vrd, ucv, w3, ucw3, ct) in TABEL {
        let k = r
            .kandidaten
            .iter()
            .find(|k| k.naam == naam)
            .unwrap_or_else(|| panic!("{naam} ontbreekt"));
        let t = &k.tussenwaarden;
        let toets = |id: &str| k.toetsen.iter().find(|x| x.id == id).unwrap();
        golden(&format!("{naam} q_d"), t["q_d"], qd, 3);
        golden(&format!("{naam} M_Ed"), t["m_ed"], m, 3);
        golden(&format!("{naam} M_el,Rd"), t["m_el_rd"], mrd, 3);
        golden(&format!("{naam} V_Ed"), t["v_ed"], v, 3);
        golden(&format!("{naam} A_v"), t["a_v"], av, 1);
        golden(&format!("{naam} V_pl,Rd"), t["v_pl_rd"], vrd, 2);
        golden(&format!("{naam} w_3"), t["w_3"], w3, 2);
        golden(&format!("{naam} c/t flens"), t["c_t_flens"], ct, 2);
        uc(&format!("{naam} buiging"), toets("buiging").uc.unwrap(), ucm);
        uc(&format!("{naam} dwarskracht"), toets("dwarskracht").uc.unwrap(), ucv);
        uc(&format!("{naam} w2+w3"), toets("doorbuiging_w2w3").uc.unwrap(), ucw3);
        // Geen plastische weerstand voor een enkel U-profiel.
        assert!(!t.contains_key("m_pl_rd"), "{naam}: m_pl_rd hoort niet bij UNP");
        assert_eq!(t["klasse_flens"], 1.0, "{naam}");
    }
}

#[test]
fn s2_advies_en_vaste_waarschuwing() {
    let r = bereken_staal(&invoer()).unwrap();
    let advies = r.advies.as_ref().expect("advies");
    assert_eq!(advies.naam, "UNP 140");
    assert!(r
        .meldingen
        .iter()
        .any(|m| m.tekst == constructie_core::staal::UNP_WAARSCHUWING));
    // Wringing/kip blokkeren niet: de waarschuwing staat er, het advies ook.
    assert!(r.meldingen.iter().any(|m| m.tekst.contains("W_el,y")));
}

#[test]
fn s2_alle_unp_maten_aanwezig() {
    let r = bereken_staal(&invoer()).unwrap();
    let namen: Vec<&str> = r.kandidaten.iter().map(|k| k.naam.as_str()).collect();
    for maat in [80, 100, 120, 140, 160, 180, 200, 220, 240, 260, 280, 300, 320, 350, 380, 400] {
        assert!(namen.contains(&format!("UNP {maat}").as_str()), "UNP {maat} ontbreekt");
    }
    assert_eq!(namen.len(), 16);
}

// naam, V_Ed, V_pl,Rd, rho, M_Ed, M_el,Rd, M_Rd, UC M
type RijB = (&'static str, f64, f64, f64, f64, f64, f64, f64);

/// Golden S2b: hoge dwarskracht (V_Ed > 0,5 V_pl,Rd), M_Rd = (1 - rho) M_el,Rd.
/// S235, l = 1,00 m, belastingbreedte 4,50 m, g_k 15 kN/m2. Waarden: controlescript deel 3.
#[test]
fn s2b_hoge_dwarskracht() {
    let invoer: StaalInvoer = serde_json::from_str(
        r#"{ "algemeen": { "overspanning_m": 1.0, "permanent_kn_m2": 15.0,
                            "gevolgklasse": "CC2" },
             "belastingbreedte_m": 4.5, "staalsoort": "S235", "reeksen": ["UNP"] }"#,
    )
    .unwrap();
    let r = bereken_staal(&invoer).unwrap();
    let tabel: [RijB; 3] = [
        ("UNP 80", 47.983, 66.75, 0.1915, 11.996, 6.228, 5.035, 2.383),
        ("UNP 100", 47.997, 84.56, 0.0183, 11.999, 9.682, 9.505, 1.262),
        ("UNP 120", 48.015, 115.87, 0.0, 12.004, 14.264, 14.264, 0.842),
    ];
    for (naam, v, vrd, rho, m, mel, mrd, ucm) in tabel {
        let k = r.kandidaten.iter().find(|k| k.naam == naam).unwrap();
        let t = &k.tussenwaarden;
        golden(&format!("{naam} V_Ed"), t["v_ed"], v, 3);
        golden(&format!("{naam} V_pl,Rd"), t["v_pl_rd"], vrd, 2);
        assert!((t["rho_mv"] - rho).abs() <= 0.0005, "{naam} rho {}", t["rho_mv"]);
        golden(&format!("{naam} M_Ed"), t["m_ed"], m, 3);
        golden(&format!("{naam} M_el,Rd"), t["m_el_rd"], mel, 3);
        golden(&format!("{naam} M_Rd"), t["m_rd"], mrd, 3);
        let buiging = k.toetsen.iter().find(|x| x.id == "buiging").unwrap();
        uc(&format!("{naam} buiging"), buiging.uc.unwrap(), ucm);
    }
}
