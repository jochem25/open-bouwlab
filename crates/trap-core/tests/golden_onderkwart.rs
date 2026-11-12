//! Golden test 3 (O-2026-11-12-01): kwartdraai met onderkwart naar een
//! offertetekening van een leverancier (vector-pdf, uitgelezen). De invoer staat
//! geanonimiseerd in `fixtures/invoer_kwartdraai_onderkwart.json`; de maten van
//! de leverancier (neuslijnen, aantreden) staan lokaal in
//! `tests/fixtures.local/trap/onderkwart_leverancier_maten.json` (gitignored;
//! zonder die fixture slaat het vergelijkende deel over).
//!
//! Getoetst (moet gelijk): optreden en optrede, aantal treden en stootborden,
//! looplijn 300 uit de buitenkant van de boom (= looplijn leverancier), de
//! aantrede op die looplijn binnen 1 mm van de leverancier, de rechte treden
//! (onverdreven voorkanten) binnen 2 mm. Gemeten en vastgelegd (niet
//! weggepoetst): de verdreven voorkanten van trap-core tegen die van de
//! leverancier. Uitvoer: `target/trap-core/onderkwart_vergelijking.json`.

mod common;

use std::path::PathBuf;

use serde_json::{json, Value};
use trap_core::invoer::Invoer;
use trap_core::model::P2;
use trap_core::verdrijving::{Contour, Verdeling, Voorkant, Waar};
use trap_core::vlak::{eenheid, plus, sub, Polylijn};
use trap_core::{ontwerp, ontwerp_uitvoer};

const INVOER: &str = include_str!("fixtures/invoer_kwartdraai_onderkwart.json");

fn r1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// Voorkanten van de leverancier als `Verdeling` op dezelfde contour.
fn verdeling_van(c: &Contour, lijnen: &[[P2; 2]]) -> Verdeling {
    let mut vk = Vec::new();
    for (k, l) in lijnen.iter().enumerate() {
        let d = eenheid(sub(l[1], l[0])).unwrap();
        let ver = plus(l[0], d, -5000.0);
        let (u, pb) = snijd(&c.binnen, ver, d);
        let (w, pu) = snijd(&c.buiten, ver, d);
        let (s, pl) = snijd(&c.looplijn, ver, d);
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

/// Snijpunt van een lijn met een polylijn; op het begin- of eindpunt (eerste
/// trede, welstuk) geeft `snijd` niets, dan het eindpunt zelf.
fn snijd(pl: &Polylijn, p0: P2, d: P2) -> (f64, P2) {
    if let Some(x) = pl.snijd(p0, d) {
        return x;
    }
    let n = pl.punten.len();
    for (s, q) in [(0.0, pl.punten[0]), (pl.lengte(), pl.punten[n - 1])] {
        if trap_core::vlak::kruis(d, sub(q, p0)).abs() < 1.0 {
            return (s, q);
        }
    }
    panic!("voorkant snijdt de zijde niet");
}

fn contour(inv: &Invoer) -> Contour {
    let trap_core::invoer::Plattegrond::Contour(p) = &inv.plattegrond else { panic!("contour") };
    Contour::nieuw(
        Polylijn::nieuw(p.binnenzijde.clone()).unwrap(),
        Polylijn::nieuw(p.buitenzijde.clone()).unwrap(),
        p.kant_binnen,
        inv.looplijn.afstand_buitenkant_boom - inv.boom.dikte,
        inv.looplijn.hoek,
        inv.looplijn.straal,
    )
    .unwrap()
}

#[test]
fn onderkwart_zonder_lokale_fixture() {
    let inv: Invoer = serde_json::from_str(INVOER).unwrap();
    let o = ontwerp(&inv).expect("ontwerp");
    assert_eq!(o.model.trap.optreden, 16);
    assert!((o.model.trap.optrede - 198.75).abs() < 1e-9);
    assert_eq!(o.model.treden.len(), 16, "15 treden + welstuk");
    assert_eq!(o.model.borden.len(), 16);
    // looplijn 262 uit de rand loopbreedte = 300 uit de buitenkant van de boom 38
    let c = contour(&inv);
    let (_, d) = c.buiten.projecteer(c.looplijn.punten[0]);
    assert!((d - 262.0).abs() < 1e-6, "{d}");
    // uitvoer zoals het scherm (ook voor de screenshots)
    let u = ontwerp_uitvoer(&inv, "2026-11-12T00:00:00").unwrap();
    let uit = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/trap-core");
    std::fs::create_dir_all(&uit).unwrap();
    std::fs::write(uit.join("onderkwart_kern.ifc"), &u.ifc).unwrap();
}

#[test]
fn onderkwart_tegen_leverancier() {
    let inv: Invoer = serde_json::from_str(INVOER).unwrap();
    let Some(r) = common::lokaal("onderkwart_leverancier_maten.json") else { return };
    let r: Value = serde_json::from_str(&r).unwrap();
    let o = ontwerp(&inv).expect("ontwerp");
    let (m, t) = (&o.model, &o.rapport.resultaat);

    assert_eq!(m.trap.optreden, r["optreden"].as_u64().unwrap() as u32);
    assert!((m.trap.optrede - r["optrede"].as_f64().unwrap()).abs() < 1e-9);
    assert_eq!(m.treden.len() - 1, r["treden"].as_u64().unwrap() as usize);
    assert_eq!(m.borden.len(), r["stootborden"].as_u64().unwrap() as usize);

    let a_lev = r["aantrede_looplijn_leverancier"].as_f64().unwrap();
    assert!((t.aantrede_looplijn - a_lev).abs() < 1.0, "aantrede looplijn {} vs {a_lev}", t.aantrede_looplijn);

    // leverancier-voorkanten op dezelfde contour en looplijn
    let c = contour(&inv);
    let lijnen: Vec<[P2; 2]> = serde_json::from_value(r["voorkanten"].clone()).unwrap();
    assert_eq!(lijnen.len(), t.voorkanten.len());
    let lev = verdeling_van(&c, &lijnen);
    let a_lev_loop = lev.aantreden(Waar::Loop);
    // verschil per voorkant langs binnen- en buitenzijde (kern - leverancier)
    let d_binnen: Vec<f64> = t.voorkanten.iter().zip(&lev.voorkanten).map(|(a, b)| r1(a.u_binnen - b.u_binnen)).collect();
    let d_buiten: Vec<f64> = t.voorkanten.iter().zip(&lev.voorkanten).map(|(a, b)| r1(a.w_buiten - b.w_buiten)).collect();
    let d_loop: Vec<f64> = t.voorkanten.iter().zip(&lev.voorkanten).map(|(a, b)| r1(a.s_loop - b.s_loop)).collect();
    // onverdreven bij beide (recht in een arm): binnen 2 mm op de looplijn
    for (k, x) in t.voorkanten.iter().enumerate() {
        let lev_recht = {
            let l = lijnen[k];
            (l[0][0] - l[1][0]).abs() < 0.5 || (l[0][1] - l[1][1]).abs() < 0.5
        };
        if !x.verdreven && lev_recht {
            assert!(d_loop[k].abs() < 2.0, "voorkant {k}: looplijn {} mm", d_loop[k]);
        }
    }
    let rond = |v: &[f64]| v.iter().map(|x| r1(*x)).collect::<Vec<_>>();
    let vergelijking = json!({
        "gelijk": {
            "optreden": [m.trap.optreden, r["optreden"]],
            "optrede": [m.trap.optrede, r["optrede"]],
            "aantrede_looplijn": [r1(t.aantrede_looplijn), a_lev],
            "looplijn": "300 uit de buitenkant van de boom, boog r 500 (leverancier gemeten 299,8 / 300,6, r 499,7)"
        },
        "verdrijving": {
            "vakken_kern": t.vakken.iter().map(|v| [v.van, v.tot]).collect::<Vec<_>>(),
            "verdreven_kern": t.voorkanten.iter().filter(|v| v.verdreven).map(|v| v.nummer).collect::<Vec<_>>(),
            "verschil_voorkant_langs_binnenzijde_mm": d_binnen,
            "verschil_voorkant_langs_buitenzijde_mm": d_buiten,
            "verschil_voorkant_op_looplijn_mm": d_loop,
            "aantreden_binnenzijde_kern": rond(&t.aantreden_binnenzijde),
            "aantreden_binnenzijde_leverancier": rond(&lev.aantreden(Waar::Binnen)),
            "aantreden_buitenzijde_kern": rond(&t.aantreden_buitenzijde),
            "aantreden_buitenzijde_leverancier": rond(&lev.aantreden(Waar::Buiten)),
            "aantreden_looplijn_leverancier_op_kern_looplijn": rond(&a_lev_loop),
            "kleinste_binnen": [r1(t.aantreden_binnenzijde.iter().copied().fold(f64::MAX, f64::min)),
                                r1(lev.aantreden(Waar::Binnen).iter().copied().fold(f64::MAX, f64::min))]
        },
        "toets_kern": o.rapport.regels.iter().map(|x| json!([x.id, x.oordeel, x.waarde])).collect::<Vec<_>>(),
        "waarschuwingen_kern": o.rapport.waarschuwingen,
        "vrije_hoogte_leverancier": r["vrije_hoogte_leverancier"],
    });
    let uit = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/trap-core");
    std::fs::create_dir_all(&uit).unwrap();
    std::fs::write(uit.join("onderkwart_vergelijking.json"), serde_json::to_string_pretty(&vergelijking).unwrap()).unwrap();
}
