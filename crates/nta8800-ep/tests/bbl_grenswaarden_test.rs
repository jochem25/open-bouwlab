//! Tests BENG-grenswaarden per gebruiksfunctie, Bbl tabel 4.148A en
//! art. 4.149 lid 2. Alle verwachtingen zijn met de hand nagerekend.

use nta8800_ep::{
    beng1_limit_woonfunctie_grondgebonden, weighted_limits, BblGebruiksfunctie as F, BengLimits,
};

fn limits(f: F, x: f64) -> BengLimits {
    BengLimits::for_bbl_functie(f, x).expect("functie heeft een eis")
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "kreeg {a}, verwacht {b}");
}

#[test]
fn kantoor_bij_1_5_en_2_3() {
    // Bbl tabel 4.148A, rij 6: x <= 1,8 -> 90 (voetnoot 6).
    let l = limits(F::Kantoorfunctie, 1.5);
    close(l.beng1_max_kwh_per_m2, 90.0);
    close(l.beng2_max_kwh_per_m2, 40.0);
    close(l.beng3_min_pct, 30.0);
    // x = 2,3 > 1,8 -> 90 + 30 * (2,3 - 1,8) = 90 + 15 = 105 (voetnoot 7).
    close(limits(F::Kantoorfunctie, 2.3).beng1_max_kwh_per_m2, 105.0);
}

#[test]
fn bijeenkomst_overig_grens_1_8_inclusief() {
    // Bbl tabel 4.148A, rij 2b: x <= 1,8 -> 90 (voetnoot 6 is inclusief).
    let l = limits(F::BijeenkomstOverig, 1.8);
    close(l.beng1_max_kwh_per_m2, 90.0);
    close(l.beng2_max_kwh_per_m2, 60.0);
    close(l.beng3_min_pct, 30.0);
    // x = 2,0 -> 90 + 30 * 0,2 = 96.
    close(limits(F::BijeenkomstOverig, 2.0).beng1_max_kwh_per_m2, 96.0);
}

#[test]
fn bijeenkomst_kinderopvang() {
    // Bbl tabel 4.148A, rij 2a: 160 / 70 / 40; x = 2,3 -> 160 + 15 = 175.
    let l = limits(F::BijeenkomstKinderopvang, 2.3);
    close(l.beng1_max_kwh_per_m2, 175.0);
    close(l.beng2_max_kwh_per_m2, 70.0);
    close(l.beng3_min_pct, 40.0);
}

#[test]
fn sportfunctie() {
    // Bbl tabel 4.148A, rij 9: 40 + 15 * (x - 1,8); x = 2,8 -> 55.
    let l = limits(F::Sportfunctie, 2.8);
    close(l.beng1_max_kwh_per_m2, 55.0);
    close(l.beng2_max_kwh_per_m2, 90.0);
    close(l.beng3_min_pct, 30.0);
    close(limits(F::Sportfunctie, 1.0).beng1_max_kwh_per_m2, 40.0);
}

#[test]
fn onderwijsfunctie() {
    // Bbl tabel 4.148A, rij 8: 190 + 30 * (x - 1,8); x = 2,3 -> 205.
    let l = limits(F::Onderwijsfunctie, 2.3);
    close(l.beng1_max_kwh_per_m2, 205.0);
    close(l.beng2_max_kwh_per_m2, 70.0);
    close(l.beng3_min_pct, 40.0);
}

#[test]
fn celfunctie() {
    // Bbl tabel 4.148A, rij 3: 160 + 35 * (x - 1,8); x = 2,3 -> 177,5.
    let l = limits(F::Celfunctie, 2.3);
    close(l.beng1_max_kwh_per_m2, 177.5);
    close(l.beng2_max_kwh_per_m2, 120.0);
    close(l.beng3_min_pct, 30.0);
}

#[test]
fn winkelfunctie() {
    // Bbl tabel 4.148A, rij 10: 70 + 30 * (x - 1,8); x = 2,8 -> 100.
    let l = limits(F::Winkelfunctie, 2.8);
    close(l.beng1_max_kwh_per_m2, 100.0);
    close(l.beng2_max_kwh_per_m2, 60.0);
    close(l.beng3_min_pct, 30.0);
}

#[test]
fn gezondheidszorg_met_bedgebied_is_vlak() {
    // Bbl tabel 4.148A, rij 4a: 350 / 130 / 30 ongeacht vormfactor.
    for x in [0.5, 1.8, 3.0, 5.0] {
        let l = limits(F::GezondheidszorgMetBedgebied, x);
        close(l.beng1_max_kwh_per_m2, 350.0);
        close(l.beng2_max_kwh_per_m2, 130.0);
        close(l.beng3_min_pct, 30.0);
    }
}

#[test]
fn gezondheidszorg_overig() {
    // Bbl tabel 4.148A, rij 4b: 90 + 35 * (x - 1,8); x = 2,8 -> 125.
    let l = limits(F::GezondheidszorgOverig, 2.8);
    close(l.beng1_max_kwh_per_m2, 125.0);
    close(l.beng2_max_kwh_per_m2, 50.0);
    close(l.beng3_min_pct, 40.0);
}

#[test]
fn logies() {
    // Bbl tabel 4.148A, rij 7a: 100 + 35 * (x - 1,8); x = 2,8 -> 135; 130 / 40.
    let a = limits(F::LogiesInLogiesgebouw, 2.8);
    close(a.beng1_max_kwh_per_m2, 135.0);
    close(a.beng2_max_kwh_per_m2, 130.0);
    close(a.beng3_min_pct, 40.0);
    // Rij 7b: x = 2,0 -> 55 + 30 * 0,5 = 70; 40 / 50.
    let b = limits(F::LogiesOverig, 2.0);
    close(b.beng1_max_kwh_per_m2, 70.0);
    close(b.beng2_max_kwh_per_m2, 40.0);
    close(b.beng3_min_pct, 50.0);
    // x = 3,5 -> 100 + 50 * 0,5 = 125 (voetnoot 3).
    close(limits(F::LogiesOverig, 3.5).beng1_max_kwh_per_m2, 125.0);
}

#[test]
fn woongebouw() {
    // Bbl tabel 4.148A, rij 1a: x <= 1,83 -> 65 (voetnoot 1).
    let l = limits(F::Woongebouw, 1.83);
    close(l.beng1_max_kwh_per_m2, 65.0);
    close(l.beng2_max_kwh_per_m2, 50.0);
    close(l.beng3_min_pct, 40.0);
    // x = 2,0 -> 55 + 30 * 0,5 = 70 (voetnoot 2).
    close(limits(F::Woongebouw, 2.0).beng1_max_kwh_per_m2, 70.0);
    // x = 3,5 -> 100 + 50 * 0,5 = 125 (voetnoot 3).
    close(limits(F::Woongebouw, 3.5).beng1_max_kwh_per_m2, 125.0);
}

#[test]
fn woongebouw_sprong_op_1_83_zoals_in_het_bbl() {
    // Bbl rij 1a: net boven 1,83 geeft voetnoot (2) 55 + 30 * 0,33 = 64,9,
    // net eronder is het 65. Het Bbl publiceert deze sprong van 0,1.
    let onder = limits(F::Woongebouw, 1.83).beng1_max_kwh_per_m2;
    let boven = limits(F::Woongebouw, 1.83 + 1e-9).beng1_max_kwh_per_m2;
    assert!((onder - boven - 0.1).abs() < 1e-6, "{onder} vs {boven}");
}

#[test]
fn woongebouw_continu_op_3_0() {
    // Voetnoot (2) op 3,0 = 55 + 45 = 100 = voetnoot (3) op 3,0.
    let onder = limits(F::Woongebouw, 3.0).beng1_max_kwh_per_m2;
    let boven = limits(F::Woongebouw, 3.0 + 1e-9).beng1_max_kwh_per_m2;
    close(onder, 100.0);
    assert!((onder - boven).abs() < 1e-6);
}

#[test]
fn industrie_en_overig_geen_eis() {
    assert!(BengLimits::for_bbl_functie(F::Industriefunctie, 2.0).is_none());
    assert!(BengLimits::for_bbl_functie(F::OverigeGebruiksfunctie, 2.0).is_none());
}

#[test]
fn continuiteit_op_knik_1_8() {
    // Bbl tabel 4.148A: voetnoot (6) basis en (7) basis + helling * 0 sluiten aan.
    for f in [
        F::BijeenkomstKinderopvang,
        F::BijeenkomstOverig,
        F::Celfunctie,
        F::GezondheidszorgOverig,
        F::Kantoorfunctie,
        F::LogiesInLogiesgebouw,
        F::Onderwijsfunctie,
        F::Sportfunctie,
        F::Winkelfunctie,
    ] {
        let op = limits(f, 1.8).beng1_max_kwh_per_m2;
        let boven = limits(f, 1.8 + 1e-9).beng1_max_kwh_per_m2;
        assert!((op - boven).abs() < 1e-6, "{f:?}: sprong {op} vs {boven}");
    }
}

#[test]
fn andere_woonfunctie_gelijk_aan_for_woonfunctie() {
    // Bbl tabel 4.148A, rij 1e: zelfde curve, 30 en 50 %.
    let mut x = 0.5;
    while x <= 5.0 {
        let a = limits(F::AndereWoonfunctie, x);
        let b = BengLimits::for_woonfunctie(x);
        assert_eq!(a, b, "afwijking bij {x}");
        close(
            a.beng1_max_kwh_per_m2,
            beng1_limit_woonfunctie_grondgebonden(x),
        );
        x += 0.07;
    }
}

#[test]
fn gewogen_kantoor_en_bijeenkomst() {
    // x = 2,0. Kantoor (rij 6): 90 + 30 * 0,2 = 96; BENG 2 = 40; BENG 3 = 30.
    // Bijeenkomst overig (rij 2b): 96; 60; 30. Gewicht 100 : 300.
    // BENG 1: (100 * 96 + 300 * 96) / 400 = 96.
    // BENG 2: (100 * 40 + 300 * 60) / 400 = 22000 / 400 = 55.
    // BENG 3: (100 * 30 + 300 * 30) / 400 = 30.
    let l = weighted_limits(
        &[(F::Kantoorfunctie, 100.0), (F::BijeenkomstOverig, 300.0)],
        2.0,
    )
    .expect("eis");
    close(l.beng1_max_kwh_per_m2, 96.0);
    close(l.beng2_max_kwh_per_m2, 55.0);
    close(l.beng3_min_pct, 30.0);
}

#[test]
fn gewogen_kantoor_en_sport() {
    // x = 2,0. Kantoor 96 / 40 / 30; sport (rij 9): 40 + 15 * 0,2 = 43 / 90 / 30.
    // Gelijke gewichten: BENG 1 = (96 + 43) / 2 = 69,5; BENG 2 = 65; BENG 3 = 30.
    let l =
        weighted_limits(&[(F::Kantoorfunctie, 50.0), (F::Sportfunctie, 50.0)], 2.0).expect("eis");
    close(l.beng1_max_kwh_per_m2, 69.5);
    close(l.beng2_max_kwh_per_m2, 65.0);
    close(l.beng3_min_pct, 30.0);
}

#[test]
fn gewogen_functie_zonder_eis_telt_niet_mee() {
    // Industrie (rij 5) heeft geen eis: 100 m2 kantoor + 500 m2 industrie
    // geeft puur de kantoorgrenzen (x = 2,3 -> 105 / 40 / 30).
    let l = weighted_limits(
        &[(F::Kantoorfunctie, 100.0), (F::Industriefunctie, 500.0)],
        2.3,
    )
    .expect("eis");
    close(l.beng1_max_kwh_per_m2, 105.0);
    close(l.beng2_max_kwh_per_m2, 40.0);
    close(l.beng3_min_pct, 30.0);
}

#[test]
fn gewogen_randgevallen() {
    assert!(weighted_limits(&[], 2.0).is_none());
    assert!(weighted_limits(&[(F::Industriefunctie, 100.0)], 2.0).is_none());
    assert!(weighted_limits(&[(F::Kantoorfunctie, 0.0)], 2.0).is_none());
    // Eén functie == for_bbl_functie.
    let een = weighted_limits(&[(F::Winkelfunctie, 123.0)], 2.4).expect("eis");
    close(
        een.beng1_max_kwh_per_m2,
        limits(F::Winkelfunctie, 2.4).beng1_max_kwh_per_m2,
    );
    assert_eq!(een, limits(F::Winkelfunctie, 2.4));
}

#[test]
fn serde_namen_zijn_snake_case() {
    let j = serde_json::to_string(&F::GezondheidszorgMetBedgebied).expect("serialiseren");
    assert_eq!(j, "\"gezondheidszorg_met_bedgebied\"");
    let f: F = serde_json::from_str("\"logies_in_logiesgebouw\"").expect("deserialiseren");
    assert_eq!(f, F::LogiesInLogiesgebouw);
    let f: F = serde_json::from_str("\"bijeenkomst_kinderopvang\"").expect("deserialiseren");
    assert_eq!(f, F::BijeenkomstKinderopvang);
}
