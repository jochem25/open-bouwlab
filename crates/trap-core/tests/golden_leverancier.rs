//! Golden test 2 (O-2026-10-30-03): nabouw van een kwartdraaitrap van een
//! leverancier. De invoer is afgeleid uit het coordinatie-IFC
//! (`fixtures/invoer_kwartdraai_leverancier.json`, geanonimiseerd); de gemeten
//! maten staan lokaal in `tests/fixtures.local/trap/leverancier_kwartdraai_maten.json`
//! (zonder die fixture slaat de test over).
//!
//! Getoetst: wat gelijk moet zijn (aantal optreden, optrede, dikten, spil).
//! Wat verschilt (aantrede, verdrijving, trededieptes) wordt gemeten en
//! verklaard in de levering, niet weggepoetst; de test legt de verschillen vast
//! zodat een wijziging opvalt. Uitvoer: `target/trap-core/leverancier_vergelijking.json`.

mod common;

use std::path::PathBuf;

use serde_json::{json, Value};
use trap_core::invoer::{Invoer, Kant, Looplijnhoek};
use trap_core::model::P2;
use trap_core::onderdelen::volume;
use trap_core::toets::Oordeel;
use trap_core::verdrijving::{aantreden_op_klimlijn, Contour, Loodrecht, Verdeling, Voorkant, Waar};
use trap_core::vlak::{eenheid, plus, sub, Polylijn};
use trap_core::{ontwerp, schrijf_ifc, IfcOpties, DISCLAIMER};

const INVOER: &str = include_str!("fixtures/invoer_kwartdraai_leverancier.json");

fn r1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// De leverancier-voorkanten als `Verdeling` op dezelfde contour, zodat de
/// leverancier-trap met dezelfde regels gemeten kan worden.
fn leverancier_verdeling(c: &Contour, lijnen: &[[P2; 2]]) -> Verdeling {
    let mut vk = Vec::new();
    for (k, l) in lijnen.iter().enumerate() {
        let d = eenheid(sub(l[1], l[0])).unwrap();
        let ver = plus(l[0], d, -5000.0);
        let (u, pb) = c.binnen.snijd(ver, d).expect("binnenzijde");
        let (w, pu) = c.buiten.snijd(ver, d).expect("buitenzijde");
        let (s, pl) = c.looplijn.snijd(ver, d).expect("looplijn");
        vk.push(Voorkant {
            nummer: k,
            s_loop: s,
            u_binnen: u,
            w_buiten: w,
            p_binnen: pb,
            p_loop: pl,
            p_buiten: pu,
            verdreven: false,
        });
    }
    Verdeling { aantrede_loop: 0.0, voorkanten: vk, vakken: vec![], waarschuwingen: vec![] }
}

#[test]
fn leverancier_nabouw() {
    let inv: Invoer = serde_json::from_str(INVOER).unwrap();
    let Some(r) = common::lokaal("leverancier_kwartdraai_maten.json") else { return };
    let r: Value = serde_json::from_str(&r).unwrap();
    let o = ontwerp(&inv).expect("ontwerp");
    let (m, t) = (&o.model, &o.rapport);

    // gelijk aan leverancier
    assert_eq!(m.trap.optreden, r["optreden"].as_u64().unwrap() as u32);
    assert!((m.trap.optrede - r["pset"]["optrede"].as_f64().unwrap()).abs() < 1e-9);
    assert_eq!(m.treden.len(), r["treden_incl_welstuk"].as_u64().unwrap() as usize);
    assert_eq!(m.borden.len(), r["stootborden"].as_u64().unwrap() as usize);
    for tr in &m.treden {
        assert!((tr.z1 - tr.z0 - 39.0).abs() < 1e-9);
    }
    let paal = &m.palen[0].paal;
    let xs: Vec<f64> = paal.veelhoek.iter().map(|p| p[0]).collect();
    let ys: Vec<f64> = paal.veelhoek.iter().map(|p| p[1]).collect();
    let span = |v: &[f64]| (v.iter().copied().fold(f64::MAX, f64::min), v.iter().copied().fold(f64::MIN, f64::max));
    let (sx, sy) = (span(&xs), span(&ys));
    assert!((sx.0 - 907.0).abs() < 1e-6 && (sx.1 - 1022.0).abs() < 1e-6, "spil x {sx:?}");
    assert!((sy.0 - 495.5).abs() < 1e-6 && (sy.1 - 563.5).abs() < 1e-6, "spil y {sy:?}");

    // eigen verdeling op de leverancier-contour vs de leverancier-voorkanten
    let pl_b = Polylijn::nieuw(vec![[2674.1, 549.0], [921.0, 549.0], [921.0, 53.0]]).unwrap();
    let pl_u = Polylijn::nieuw(vec![[2674.1, 1421.0], [49.0, 1421.0], [49.0, 53.0]]).unwrap();
    let c = Contour::nieuw(pl_b, pl_u, Kant::Links, 300.0, Looplijnhoek::Boog, None).unwrap();
    let lijnen: Vec<[P2; 2]> = serde_json::from_value(r["voorkanten"].clone()).unwrap();
    let cv = leverancier_verdeling(&c, &lijnen);
    let ref_buiten: Vec<f64> = serde_json::from_value(r["aantreden_buitenzijde"].clone()).unwrap();
    let c_buiten = cv.aantreden(Waar::Buiten);
    for (a, b) in c_buiten.iter().zip(&ref_buiten) {
        assert!((a - b).abs() < 0.15, "leverancier buitenzijde {a} vs gemeten {b}");
    }
    let band: Vec<Value> = (0..=27)
        .map(|i| {
            let d = 300.0 + 10.0 * f64::from(i);
            let k = aantreden_op_klimlijn(&c, &cv, d.min(572.0), Loodrecht::VanafVolgende).unwrap();
            let e = aantreden_op_klimlijn(&c, &cv, d.min(572.0), Loodrecht::VanafEigen).unwrap();
            json!({"afstand": d.min(572.0), "B_langs_min": r1(k.langs_min),
                   "A_min": r1(k.loodrecht.iter().copied().fold(f64::MAX, f64::min)),
                   "A_eigen_min": r1(e.loodrecht.iter().copied().fold(f64::MAX, f64::min))})
        })
        .collect();

    let kern = &t.resultaat;
    let regel = |id: &str| t.regels.iter().find(|x| x.id == id).unwrap();
    let rond = |v: &[f64]| v.iter().map(|x| r1(*x)).collect::<Vec<_>>();
    let trede_diepte_recht = {
        // trede 01 is recht (arm 1): diepte in looprichting (x)
        let xs: Vec<f64> = m.treden[0].veelhoek.iter().map(|p| p[0]).collect();
        let (a, b) = span(&xs);
        b - a
    };
    let boom_binnen_1 = kern.boomdelen.iter().find(|b| b.naam == "binnenboom deel 1").unwrap();
    let vergelijking = json!({
        "gelijk": {
            "optreden": [m.trap.optreden, r["optreden"]],
            "optrede": [m.trap.optrede, r["pset"]["optrede"]],
            "treden_incl_welstuk": [m.treden.len(), r["treden_incl_welstuk"]],
            "stootborden": [m.borden.len(), r["stootborden"]],
            "trede_dikte": [39.0, r["trede_dikte"]],
            "stootbord_dikte": [13.0, r["stootbord_dikte"]],
            "boom_dikte": [39.0, r["boom_dikte"]],
            "spil_plattegrond": [[sx.1 - sx.0, sy.1 - sy.0], [115.0, 68.0]]
        },
        "verschillend": {
            "aantrede_looplijn_300": [r1(kern.aantrede_looplijn), "leverancier recht 194"],
            "aantreden_looplijn_leverancier_op_onze_looplijn_300": rond(&cv.aantreden(Waar::Loop)),
            "aantreden_buitenzijde_kern": rond(&kern.aantreden_buitenzijde),
            "aantreden_buitenzijde_leverancier": rond(&c_buiten),
            "aantreden_binnenzijde_kern": rond(&kern.aantreden_binnenzijde),
            "aantreden_binnenzijde_leverancier": rond(&cv.aantreden(Waar::Binnen)),
            "vakken_kern": kern.vakken.iter().map(|v| [v.van, v.tot]).collect::<Vec<_>>(),
            "trede_recht_diepte": [r1(trede_diepte_recht), r["trede_recht_diepte"]],
            "spil_hoogte": [r1(paal.z1 - paal.z0), r["spil"]["hoogte"]],
            "binnenboom_arm1_hoogte_bereik": [r1(boom_binnen_1.z_min), r1(boom_binnen_1.z_max)],
            "boomdelen": [kern.boomdelen.len(), r["boomdelen"]],
            "nesten": [kern.boomdelen.iter().map(|b| b.nesten).sum::<usize>(), "leverancier: geen nesten gemodelleerd"]
        },
        "toets_kern": {
            "aantrede_klimlijn": [regel("bbl-4.26-aantrede").oordeel, regel("bbl-4.26-aantrede").waarde, &regel("bbl-4.26-aantrede").toelichting],
            "tredevlak_klimlijn": [regel("bbl-4.26-tredevlak-klimlijn").oordeel, regel("bbl-4.26-tredevlak-klimlijn").waarde],
            "optrede": [regel("bbl-4.26-optrede").oordeel, regel("bbl-4.26-optrede").waarde],
            "tredevlak": [regel("bbl-4.26-tredevlak").oordeel, regel("bbl-4.26-tredevlak").waarde],
            "stapmaat": regel("praktijk-stapmaat").waarde,
            "klimlijn_band_kern": kern.klimlijnen
        },
        "toets_leverancier_met_dezelfde_regels": {
            "klimlijnen_in_band": band,
        },
        "volume_treden_mm3": m.treden.iter().map(volume).sum::<f64>().round()
    });
    let uit = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/trap-core");
    std::fs::create_dir_all(&uit).unwrap();
    std::fs::write(uit.join("leverancier_vergelijking.json"), serde_json::to_string_pretty(&vergelijking).unwrap()).unwrap();
    let opties = IfcOpties {
        tijd: "2026-10-30T00:00:00".into(),
        bestandsnaam: "leverancier_nabouw_kern.ifc".into(),
        disclaimer: Some(DISCLAIMER.into()),
        ..Default::default()
    };
    std::fs::write(uit.join("leverancier_nabouw_kern.ifc"), schrijf_ifc(m, &opties).unwrap()).unwrap();
    std::fs::write(uit.join("leverancier_nabouw_kern.toets.json"), serde_json::to_string_pretty(&o.rapport).unwrap()).unwrap();

    // vastgelegde verschillen (verklaard in de levering)
    assert!(t.regels.iter().all(|x| x.oordeel != Oordeel::VoldoetNiet || x.id.starts_with("bbl-4.26")));
}
