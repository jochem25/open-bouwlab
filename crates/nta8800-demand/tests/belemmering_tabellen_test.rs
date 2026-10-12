//! Tests voor `obstruction_g_factor` met de nieuwe belemmeringsvarianten
//! (overstek, volledige belemmering, overig), NTA 8800 §17.3.
//!
//! Alle verwachtingswaarden zijn met de hand uit het normcorpus gelezen:
//! `nta-8800__h17-klimaat-deel1.md` (`deel1`) en `nta-8800__h17-klimaat-deel2.md`
//! (`deel2`) in `kennis-v2/nen`; regelnummers verwijzen naar de rij in die
//! bestanden.

use nta8800_demand::calc::shading::{obstruction_g_factor, SolarBalance};
use nta8800_model::location::{Orientation, Tilt};
use nta8800_model::time::Month;
use nta8800_model::Obstruction;

const EPS: f64 = 1e-12;

const ALLE_ORIENTATIES: [Orientation; 9] = [
    Orientation::Noord,
    Orientation::NoordOost,
    Orientation::Oost,
    Orientation::ZuidOost,
    Orientation::Zuid,
    Orientation::ZuidWest,
    Orientation::West,
    Orientation::NoordWest,
    Orientation::Horizontaal,
];

fn tilt(degrees: f64) -> Tilt {
    Tilt { degrees }
}

fn waarde(
    obstruction: Obstruction,
    orientation: Orientation,
    degrees: f64,
    balance: SolarBalance,
    month: Month,
) -> f64 {
    obstruction_g_factor(obstruction, orientation, tilt(degrees), balance)[month]
}

fn assert_gelijk(gevonden: f64, verwacht: f64, wat: &str) {
    assert!((gevonden - verwacht).abs() < EPS, "{wat}: {gevonden} != {verwacht}");
}

// --- Volledige belemmering: tabel 17.13 / 17.14 (§17.3.7) ---

#[test]
fn full_zuid_verticaal_heating_januari() {
    // Tabel 17.13, Zuid, kolom 90°, januari = 0,19 (deel2:1246).
    let f = waarde(Obstruction::Full, Orientation::Zuid, 90.0, SolarBalance::Heating, Month::Januari);
    assert_gelijk(f, 0.19, "17.13 Zuid 90° jan");
}

#[test]
fn full_cooling_is_tabel_17_14() {
    // Tabel 17.14, Zuid, kolom 90°, januari = 0,19 (deel2:1614-1662, celgelijk aan 17.13).
    let f = waarde(Obstruction::Full, Orientation::Zuid, 90.0, SolarBalance::Cooling, Month::Januari);
    assert_gelijk(f, 0.19, "17.14 Zuid 90° jan");
}

#[test]
fn full_helling_dichtstbijzijnde_kolom() {
    // 17.3.7: dichtstbijzijnde hellingkolom. Zuid januari (deel2:1246):
    // 90°=0,19, 75°=0,20, 60°=0,22, 45°=0,25. Helling 50° ligt dichter bij 45°
    // (afstand 5) dan bij 60° (afstand 10) -> 0,25.
    let f = waarde(Obstruction::Full, Orientation::Zuid, 50.0, SolarBalance::Heating, Month::Januari);
    assert_gelijk(f, 0.25, "50° -> 45°");
}

#[test]
fn full_helling_gelijke_afstand_neemt_grootste() {
    // 17.3.7: bij gelijke afstand mag de hoogste naastliggende waarde.
    // 52,5° ligt midden tussen 45° en 60°.
    // Zuid januari (deel2:1246): 45°=0,25, 60°=0,22 -> 0,25.
    let jan = waarde(Obstruction::Full, Orientation::Zuid, 52.5, SolarBalance::Heating, Month::Januari);
    assert_gelijk(jan, 0.25, "52,5° jan");
    // Zuid mei (deel2:1250): 45°=0,33, 60°=0,35 -> 0,35 (hier wint de andere kolom).
    let mei = waarde(Obstruction::Full, Orientation::Zuid, 52.5, SolarBalance::Heating, Month::Mei);
    assert_gelijk(mei, 0.35, "52,5° mei");
    // 97,5° ligt midden tussen 90° en 105° (kolommen niet aaneengesloten in de tabel).
    // Zuid mei (deel2:1250): 90°=0,46, 105°=0,61 -> 0,61.
    let mei = waarde(Obstruction::Full, Orientation::Zuid, 97.5, SolarBalance::Heating, Month::Mei);
    assert_gelijk(mei, 0.61, "97,5° mei");
}

#[test]
fn full_helling_boven_90_gebruikt_neerwaartse_kolommen() {
    // Zuid januari (deel2:1246): 105°=0,18.
    let f = waarde(Obstruction::Full, Orientation::Zuid, 105.0, SolarBalance::Heating, Month::Januari);
    assert_gelijk(f, 0.18, "105° jan");
}

#[test]
fn full_horizontaal_gebruikt_zuid_kolom_0_graden() {
    // 17.3.1: helling < 15° -> zichtveld Zuid; kolom 0°.
    // Zuid (deel2:1246, 1250, 1251): jan 0,55, mei 0,34, jun 0,37.
    for (month, verwacht) in [(Month::Januari, 0.55), (Month::Mei, 0.34), (Month::Juni, 0.37)] {
        let f = waarde(Obstruction::Full, Orientation::Horizontaal, 0.0, SolarBalance::Heating, month);
        assert_gelijk(f, verwacht, "Horizontaal 0°");
    }
}

#[test]
fn full_helling_onder_15_gebruikt_zuid_ongeacht_orientatie() {
    // 17.3.1: helling < 15° -> oriëntatie Zuid. 10° ligt het dichtst bij kolom 15°
    // (afstand 5): Zuid januari 15° = 0,39 (deel2:1246).
    let noord = waarde(Obstruction::Full, Orientation::Noord, 10.0, SolarBalance::Heating, Month::Januari);
    assert_gelijk(noord, 0.39, "Noord 10° -> Zuid 15°");
}

// --- Overstek: tabel 17.8 (warmte) en 17.9 (koeling), §17.3.5 ---

#[test]
fn overhang_heating_zuid_drie_klassen() {
    // Tabel 17.8 Zuid februari (deel1:1623): 0,30 | 0,60 | 0,85.
    let cases = [(0.4, 0.30), (0.7, 0.60), (1.5, 0.85)];
    for (h_o, verwacht) in cases {
        let f = waarde(
            Obstruction::Overhang { h_o },
            Orientation::Zuid,
            90.0,
            SolarBalance::Heating,
            Month::Februari,
        );
        assert_gelijk(f, verwacht, &format!("17.8 Zuid feb h_o={h_o}"));
    }
}

#[test]
fn overhang_klassegrenzen() {
    // Klassen: < 0,5 | 0,5 t/m < 1,0 | >= 1,0 (kolomkoppen 17.8).
    // Zuid februari (deel1:1623): 0,30 | 0,60 | 0,85.
    let f = |h_o| {
        waarde(Obstruction::Overhang { h_o }, Orientation::Zuid, 90.0, SolarBalance::Heating, Month::Februari)
    };
    assert_gelijk(f(0.499), 0.30, "0,499");
    assert_gelijk(f(0.5), 0.60, "0,5");
    assert_gelijk(f(0.999), 0.60, "0,999");
    assert_gelijk(f(1.0), 0.85, "1,0");
}

#[test]
fn overhang_cooling_zuid_is_tabel_17_9() {
    // Tabel 17.9 Zuid (deel1:1848-1850): jan 0,88|1,00|1,00; feb 0,68|0,93|1,00;
    // maart 0,46|0,78|1,00.
    let cases = [
        (0.4, Month::Februari, 0.68),
        (0.7, Month::Februari, 0.93),
        (1.5, Month::Februari, 1.00),
        (0.4, Month::Maart, 0.46),
        (0.7, Month::Maart, 0.78),
        (0.4, Month::Januari, 0.88),
    ];
    for (h_o, month, verwacht) in cases {
        let f = waarde(Obstruction::Overhang { h_o }, Orientation::Zuid, 90.0, SolarBalance::Cooling, month);
        assert_gelijk(f, verwacht, &format!("17.9 Zuid {month:?} h_o={h_o}"));
    }
}

#[test]
fn overhang_cooling_zuidwest_gesplitste_tabel() {
    // Tabel 17.9 Zuidwest is gesplitst: juni staat in deel1:1883 (0,50|0,64|1,00),
    // juli in deel2:68 (0,56|0,70|1,00).
    let juni = waarde(Obstruction::Overhang { h_o: 0.4 }, Orientation::ZuidWest, 90.0, SolarBalance::Cooling, Month::Juni);
    assert_gelijk(juni, 0.50, "17.9 ZW juni");
    let juli = waarde(Obstruction::Overhang { h_o: 0.7 }, Orientation::ZuidWest, 90.0, SolarBalance::Cooling, Month::Juli);
    assert_gelijk(juli, 0.70, "17.9 ZW juli");
}

#[test]
fn overhang_op_hellend_vlak_gedraagt_zich_als_other() {
    // 45° is geen verticaal vlak: 17.8/17.9 gelden niet, situatie g.
    // Warmte: tabel 17.13 Zuid 45° januari = 0,25 (deel2:1246).
    // Koeling: 1,0.
    let overhang = Obstruction::Overhang { h_o: 0.7 };
    let other = Obstruction::Other { overhang_h_o: Some(0.7) };
    for balance in [SolarBalance::Heating, SolarBalance::Cooling] {
        for month in Month::all() {
            let a = waarde(overhang, Orientation::Zuid, 45.0, balance, month);
            let b = waarde(other, Orientation::Zuid, 45.0, balance, month);
            assert_gelijk(a, b, "Overhang 45° == Other");
        }
    }
    let heating = waarde(overhang, Orientation::Zuid, 45.0, SolarBalance::Heating, Month::Januari);
    assert_gelijk(heating, 0.25, "Overhang 45° heating jan");
    let cooling = waarde(overhang, Orientation::Zuid, 45.0, SolarBalance::Cooling, Month::Januari);
    assert_gelijk(cooling, 1.0, "Overhang 45° cooling jan");
}

// --- Overige belemmering (§17.3.2 g) ---

#[test]
fn other_zonder_overstek() {
    // Warmte: gelijk aan Full (17.13, Zuid 90° jan = 0,19, deel2:1246).
    // Koeling: 1,0 (tabel 17.5 bij minimale belemmering is uniform 1,00).
    let other = Obstruction::Other { overhang_h_o: None };
    for month in Month::all() {
        let h = waarde(other, Orientation::Zuid, 90.0, SolarBalance::Heating, month);
        let full = waarde(Obstruction::Full, Orientation::Zuid, 90.0, SolarBalance::Heating, month);
        assert_gelijk(h, full, "Other heating == Full");
        let c = waarde(other, Orientation::Zuid, 90.0, SolarBalance::Cooling, month);
        assert_gelijk(c, 1.0, "Other cooling");
    }
    let jan = waarde(other, Orientation::Zuid, 90.0, SolarBalance::Heating, Month::Januari);
    assert_gelijk(jan, 0.19, "Other heating jan");
}

#[test]
fn other_met_overstek_cooling_is_tabel_17_9() {
    // Tabel 17.9 Zuid februari, klasse 0,5..1,0 = 0,93 (deel1:1849).
    let other = Obstruction::Other { overhang_h_o: Some(0.7) };
    let c = waarde(other, Orientation::Zuid, 90.0, SolarBalance::Cooling, Month::Februari);
    assert_gelijk(c, 0.93, "Other + overstek cooling");
    // Warmte blijft de tabel van situatie e (17.13), niet 17.8: Zuid 90° feb = 0,30 (deel2:1247).
    let h = waarde(other, Orientation::Zuid, 90.0, SolarBalance::Heating, Month::Februari);
    assert_gelijk(h, 0.30, "Other + overstek heating");
}

// --- Ongewijzigd gedrag ---

#[test]
fn minimal_en_none_ongewijzigd() {
    // Tabel 17.4 Zuid verticaal januari = 0,23; koeling (17.5) = 1,0; None = 1,0.
    let f = waarde(Obstruction::Minimal, Orientation::Zuid, 90.0, SolarBalance::Heating, Month::Januari);
    assert_gelijk(f, 0.23, "Minimal heating jan");
    let f = waarde(Obstruction::Minimal, Orientation::Zuid, 90.0, SolarBalance::Cooling, Month::Januari);
    assert_gelijk(f, 1.0, "Minimal cooling");
    let f = waarde(Obstruction::None, Orientation::Zuid, 90.0, SolarBalance::Heating, Month::Januari);
    assert_gelijk(f, 1.0, "None");
}

// --- Structuur: alle waarden in 0..=1, alle oriëntaties/maanden/kolommen aanwezig ---

#[test]
fn structuur_alle_waarden_binnen_0_en_1() {
    let hellingen = [0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0, 105.0, 120.0, 135.0, 150.0, 165.0, 180.0];
    let overstekken = [0.0, 0.4, 0.5, 0.7, 1.0, 1.5];
    for balance in [SolarBalance::Heating, SolarBalance::Cooling] {
        for orientation in ALLE_ORIENTATIES {
            for degrees in hellingen {
                let mut varianten = vec![
                    Obstruction::Full,
                    Obstruction::Other { overhang_h_o: None },
                ];
                for h_o in overstekken {
                    varianten.push(Obstruction::Overhang { h_o });
                    varianten.push(Obstruction::Other { overhang_h_o: Some(h_o) });
                }
                for obstruction in varianten {
                    let profiel = obstruction_g_factor(obstruction, orientation, tilt(degrees), balance);
                    for month in Month::all() {
                        let v = profiel[month];
                        assert!(
                            v.is_finite() && (0.0..=1.0).contains(&v),
                            "{obstruction:?} {orientation:?} {degrees}° {balance:?} {month:?} = {v}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn structuur_tabellen_volledig_en_niet_triviaal() {
    // Elke oriëntatie levert voor Full 12 maanden; en de tabellen zijn niet
    // uniform 1,0 (zou duiden op ontbrekende/lege tabeldelen).
    for orientation in ALLE_ORIENTATIES {
        let p = obstruction_g_factor(Obstruction::Full, orientation, tilt(90.0), SolarBalance::Heating);
        assert_eq!(Month::all().len(), 12);
        let som: f64 = Month::all().iter().map(|m| p[*m]).sum();
        assert!(som < 12.0 || orientation == Orientation::Horizontaal || orientation == Orientation::Noord);
    }
    for orientation in ALLE_ORIENTATIES {
        let p = obstruction_g_factor(
            Obstruction::Overhang { h_o: 0.4 },
            orientation,
            tilt(90.0),
            SolarBalance::Heating,
        );
        let som: f64 = Month::all().iter().map(|m| p[*m]).sum();
        assert!(som > 0.0, "{orientation:?}: lege overstektabel");
    }
}
