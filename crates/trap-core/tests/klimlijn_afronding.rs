//! O-2026-10-30-07: klimlijn met afgeronde hoek. De looplijn ligt 300 uit de
//! buitenzijde; de hoek van die offsetlijn wordt afgerond met straal rho
//! (rho = breedte - 300 is de boog uit de binnenhoek, de standaard). Gemeten:
//! aantrede B (langs de lijn tussen opvolgende neuspunten) per trede voor een
//! reeks rho, bij de leverancierstrap (vaste voorkanten uit het bron-IFC, via
//! de lokale fixture), de kern-nabouw en de referentietrap (v6-hoogte). Uitvoer:
//! `target/trap-core/klimlijn_afronding.json` en `klimlijn_afronding_leverancier.svg`.

mod common;

use std::path::PathBuf;

use serde_json::{json, Value};
use trap_core::invoer::{Invoer, Kant, Looplijnhoek};
use trap_core::model::P2;
use trap_core::verdrijving::{meet_op_lijn, Contour, Loodrecht, Verdeling, Voorkant};
use trap_core::vlak::{afstand_tot_veelhoek, eenheid, plus, sub, Polylijn};
use trap_core::ontwerp;

const LEVERANCIER: &str = include_str!("fixtures/invoer_kwartdraai_leverancier.json");
const REF_INVOER: &str = include_str!("fixtures/invoer_kwartdraai_referentie.json");

const RHO: [f64; 7] = [0.0, 25.0, 50.0, 100.0, 200.0, 300.0, 572.0];
const RHO_REF: [f64; 7] = [0.0, 25.0, 50.0, 100.0, 200.0, 300.0, 500.0];
const NEUS_LEVERANCIER: f64 = 40.0;
const NEUS_REF: f64 = 42.0;

fn r1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

fn leverancier_contour(rho: f64) -> Contour {
    Contour::nieuw(cv_binnen(), cv_buiten(), Kant::Links, 300.0, Looplijnhoek::Boog, Some(rho)).unwrap()
}

/// De leverancier-voorkanten (neusranden uit het bron-IFC) als `Verdeling`.
fn leverancier_verdeling(c: &Contour, lijnen: &[[P2; 2]]) -> Verdeling {
    let mut vk = Vec::new();
    for (k, l) in lijnen.iter().enumerate() {
        let d = eenheid(sub(l[1], l[0])).unwrap();
        let ver = plus(l[0], d, -5000.0);
        let (u, pb) = c.binnen.snijd(ver, d).expect("binnenzijde");
        let (w, pu) = c.buiten.snijd(ver, d).expect("buitenzijde");
        let (s, pl) = c.looplijn.snijd(ver, d).expect("looplijn");
        vk.push(Voorkant { nummer: k, s_loop: s, u_binnen: u, w_buiten: w, p_binnen: pb, p_loop: pl, p_buiten: pu, verdreven: false });
    }
    Verdeling { aantrede_loop: 0.0, voorkanten: vk, vakken: vec![], waarschuwingen: vec![] }
}

/// (kleinste, trede) met trede 1-gebaseerd: de trede waarvan de neus voorkant k is.
fn minimum(b: &[f64]) -> (f64, usize) {
    b.iter().enumerate().fold((f64::INFINITY, 0), |m, (k, &x)| if x < m.0 { (x, k + 1) } else { m })
}

fn leverancier_b(cv: &Verdeling, rho: f64) -> (Vec<f64>, Contour) {
    let k = leverancier_contour(rho);
    (meet_op_lijn(&k, cv, Loodrecht::VanafVolgende).unwrap().langs, k)
}

/// Kleinste rho (0,1 mm) waarbij de kleinste aantrede B >= `eis`; None als
/// geen rho in (0, max] het haalt. B neemt af met rho (gecontroleerd).
fn kleinste_rho(cv: &Verdeling, eis: f64, max: f64) -> Option<f64> {
    let haalt = |rho: f64| minimum(&leverancier_b(cv, rho).0).0 >= eis;
    if haalt(max) {
        return Some(max);
    }
    let (mut lo, mut hi) = (0.1, max);
    if !haalt(lo) {
        return None;
    }
    while hi - lo > 0.05 {
        let m = 0.5 * (lo + hi);
        if haalt(m) { lo = m } else { hi = m }
    }
    Some((lo * 10.0).floor() / 10.0)
}

/// Grootste rho (0,1 mm) waarbij de kern zelf (herverdeeld) B >= `eis` haalt;
/// bij de kern neemt B af met rho (kortere looplijn).
fn kern_grootste_rho(inv: &Invoer, eis: f64, max: f64) -> Option<f64> {
    let b = |rho: f64| {
        let mut i = inv.clone();
        i.looplijn.straal = Some(rho);
        ontwerp(&i).map(|o| o.rapport.resultaat.aantrede_looplijn).unwrap_or(0.0)
    };
    if b(max) >= eis {
        return Some(max);
    }
    if b(0.1) < eis {
        return None;
    }
    let (mut lo, mut hi) = (0.1, max);
    while hi - lo > 0.05 {
        let m = 0.5 * (lo + hi);
        if b(m) >= eis { lo = m } else { hi = m }
    }
    Some((lo * 10.0).floor() / 10.0)
}

fn kern_rij(inv: &Invoer, rho: f64, neus: f64) -> Value {
    let mut inv = inv.clone();
    inv.looplijn.straal = Some(rho);
    match ontwerp(&inv) {
        Ok(o) => {
            let res = &o.rapport.resultaat;
            let b: Vec<f64> = res.voorkanten.windows(2).map(|w| w[1].s_loop - w[0].s_loop).collect();
            let (min, trede) = minimum(&b);
            let regel = |id: &str| o.rapport.regels.iter().find(|r| r.id == id).unwrap().oordeel;
            json!({
                "rho": rho, "B_min": r1(min), "tredevlak": r1(min + neus), "trede": trede,
                "looplijn_lengte": r1(res.looplijn_lengte),
                "tot_binnenzijde": r1(res.meting.klimlijn_tot_binnenzijde),
                "tot_buitenzijde": r1(res.meting.klimlijn_tot_buitenzijde),
                "grootste_knik_graden": r1(res.meting.looplijn_grootste_knik),
                "vakken": res.vakken.iter().map(|v| [v.van, v.tot]).collect::<Vec<_>>(),
                "binnenzijde_min": r1(res.aantreden_binnenzijde.iter().copied().fold(f64::INFINITY, f64::min)),
                "oordeel_aantrede": regel("bbl-4.26-aantrede"),
                "oordeel_tredevlak_klimlijn": regel("bbl-4.26-tredevlak-klimlijn"),
                "B": b.iter().map(|x| r1(*x)).collect::<Vec<_>>()
            })
        }
        Err(e) => json!({"rho": rho, "fout": e.to_string()}),
    }
}

fn svg(cv: &Verdeling, lijnen: &[[P2; 2]], spil: &[P2], rhos: &[(f64, &str)]) -> String {
    // plattegrond: x naar links (klimrichting arm 1), y omhoog; schaal 1:10 in px
    let (x0, y0, sch) = (2774.1, 1521.0, 0.25);
    let p = |q: P2| format!("{:.1},{:.1}", (x0 - q[0]) * sch + 10.0, (y0 - q[1]) * sch + 10.0);
    let pad = |pl: &[P2]| pl.iter().map(|&q| p(q)).collect::<Vec<_>>().join(" ");
    let c = leverancier_contour(572.0);
    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" font-family=\"sans-serif\" font-size=\"10\">\n",
        2800.0 * sch + 20.0, 1600.0 * sch + 90.0
    ));
    s.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n");
    for pl in [&c.binnen.punten, &c.buiten.punten] {
        s.push_str(&format!("<polyline points=\"{}\" fill=\"none\" stroke=\"black\" stroke-width=\"2\"/>\n", pad(pl)));
    }
    s.push_str(&format!("<polygon points=\"{}\" fill=\"#999\" stroke=\"black\"/>\n", pad(spil)));
    for l in lijnen {
        s.push_str(&format!("<polyline points=\"{}\" fill=\"none\" stroke=\"#888\" stroke-width=\"0.8\"/>\n", pad(l)));
    }
    // de maatgevende treden (op de rechte armen, waar rho niets verandert)
    let k0 = leverancier_contour(0.0);
    let m = meet_op_lijn(&k0, cv, Loodrecht::VanafVolgende).unwrap();
    for (i, &b) in m.langs.iter().enumerate() {
        if b < 185.0 {
            let d = |l: &[P2; 2]| eenheid(sub(l[1], l[0])).unwrap();
            let q0 = k0.looplijn.snijd(plus(lijnen[i][0], d(&lijnen[i]), -1.0), d(&lijnen[i])).unwrap().1;
            let q1 = k0.looplijn.snijd(plus(lijnen[i + 1][0], d(&lijnen[i + 1]), -1.0), d(&lijnen[i + 1])).unwrap().1;
            s.push_str(&format!(
                "<polyline points=\"{} {}\" stroke=\"#c00\" stroke-width=\"4\" opacity=\"0.5\"/>\n",
                p(q0), p(q1)
            ));
            let t = p([0.5 * (q0[0] + q1[0]) + 40.0, 0.5 * (q0[1] + q1[1]) - 60.0]);
            let (tx, ty) = t.split_once(',').unwrap();
            s.push_str(&format!(
                "<text x=\"{tx}\" y=\"{ty}\" fill=\"#c00\">trede {}: B {:.1}</text>\n",
                i + 1,
                b
            ));
        }
    }
    for (k, &(rho, kleur)) in rhos.iter().enumerate() {
        let k_c = leverancier_contour(rho);
        s.push_str(&format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"{kleur}\" stroke-width=\"1.5\"/>\n",
            pad(&k_c.looplijn.punten)
        ));
        s.push_str(&format!(
            "<text x=\"10\" y=\"{:.0}\" fill=\"{kleur}\">looplijn 300 uit buitenzijde, rho {rho:.0}</text>\n",
            1600.0 * sch + 30.0 + 14.0 * k as f64
        ));
    }
    s.push_str(&format!(
        "<text x=\"{:.0}\" y=\"{:.0}\">Leverancierstrap kwartdraai, plattegrond (grijs: neusranden uit het bron-IFC)</text>\n",
        2800.0 * sch * 0.35, 1600.0 * sch + 30.0
    ));
    s.push_str("</svg>\n");
    s
}

#[test]
fn klimlijn_afronding() {
    let inv_c: Invoer = serde_json::from_str(LEVERANCIER).unwrap();
    let Some(r) = common::lokaal("leverancier_kwartdraai_maten.json") else { return };
    let r: Value = serde_json::from_str(&r).unwrap();
    let lijnen: Vec<[P2; 2]> = serde_json::from_value(r["voorkanten"].clone()).unwrap();
    let cv = leverancier_verdeling(&leverancier_contour(572.0), &lijnen);
    let spil: Vec<P2> = vec![[907.0, 495.5], [1022.0, 495.5], [1022.0, 563.5], [907.0, 563.5]];

    // 1. de lijn blijft >= 300 van beide zijkanten en is vloeiend (rho > 0)
    let mut leverancier = Vec::new();
    let mut vorige = f64::INFINITY;
    for &rho in &RHO {
        let (b, k) = leverancier_b(&cv, rho);
        let (min, trede) = minimum(&b);
        let tot = trap_core::verdrijving::looplijn_tot_zijkanten(&k);
        let tot_spil = k.looplijn.punten.iter().map(|&q| afstand_tot_veelhoek(q, &spil)).fold(f64::INFINITY, f64::min);
        let knik = k.looplijn.grootste_knik();
        assert!(tot.0 >= 300.0 - 1e-6 && tot.1 >= 300.0 - 1e-6, "rho {rho}: {tot:?}");
        if rho > 0.0 {
            assert!(knik < 1.0, "rho {rho}: knik {knik}");
            assert!((k.afronding[0] - rho).abs() < 1e-6);
        }
        // B in de hoek neemt af met rho
        assert!(min <= vorige + 1e-9 || rho == 0.0, "rho {rho}: {min} > {vorige}");
        vorige = min;
        leverancier.push(json!({
            "rho": rho, "B_min": r1(min), "tredevlak": r1(min + NEUS_LEVERANCIER), "trede": trede,
            "looplijn_lengte": r1(k.looplijn.lengte()),
            "tot_binnenzijde": r1(tot.0), "tot_buitenzijde": r1(tot.1), "tot_spil": r1(tot_spil),
            "grootste_knik_graden": r1(knik),
            "B": b.iter().map(|x| r1(*x)).collect::<Vec<_>>()
        }));
    }
    // de standaard (rho weg) is de boog uit de binnenhoek = rho 572
    let std_c = Contour::nieuw(cv_binnen(), cv_buiten(), Kant::Links, 300.0, Looplijnhoek::Boog, None).unwrap();
    assert!((std_c.afronding[0] - 572.0).abs() < 1e-6);
    let b_std = meet_op_lijn(&std_c, &cv, Loodrecht::VanafVolgende).unwrap().langs_min;
    assert!((b_std - 162.8).abs() < 0.05, "standaard {b_std}");

    let rho_185 = kleinste_rho(&cv, 185.0, 572.0);
    let rho_190 = kleinste_rho(&cv, 230.0 - NEUS_LEVERANCIER, 572.0);
    // grootste rho die het haalt, met de afstand tot de buitenzijde als vrije parameter erbij
    let afstanden: Vec<Value> = [300.0, 280.0, 250.0, 200.0, 175.0]
        .iter()
        .map(|&a| {
            let pl_b = cv_binnen();
            let pl_u = cv_buiten();
            let sc = Contour::nieuw(pl_b, pl_u, Kant::Links, a, Looplijnhoek::Scherp, None).unwrap();
            json!({"afstand": a, "rho_0_B_min": r1(meet_op_lijn(&sc, &cv, Loodrecht::VanafVolgende).unwrap().langs_min)})
        })
        .collect();

    // 2. kern-nabouw en referentietrap (v6: vloer -50 tot +3130, 17 x 187,06)
    let kern: Vec<Value> = RHO.iter().map(|&rho| kern_rij(&inv_c, rho, NEUS_LEVERANCIER)).collect();
    let mut inv_ref: Invoer = serde_json::from_str(REF_INVOER).unwrap();
    inv_ref.niveaus.onder = -50.0;
    let ref_rijen: Vec<Value> = RHO_REF.iter().map(|&rho| kern_rij(&inv_ref, rho, NEUS_REF)).collect();
    assert!((ref_rijen[6]["B_min"].as_f64().unwrap() - 188.3).abs() < 0.05, "referentie standaard {}", ref_rijen[6]);

    let kern_rho = json!({
        "B_185": kern_grootste_rho(&inv_c, 185.0, 572.0),
        "tredevlak_230_B_190": kern_grootste_rho(&inv_c, 230.0 - NEUS_LEVERANCIER, 572.0),
    });
    let ref_rho = json!({
        "B_220_woon": kern_grootste_rho(&inv_ref, 220.0, 500.0),
        "tredevlak_230_B_188": kern_grootste_rho(&inv_ref, 230.0 - NEUS_REF, 500.0),
    });
    let uit = json!({
        "opdracht": "O-2026-10-30-07",
        "meting": "B = afstand tussen opvolgende neuspunten langs de lijn; lijn 300 uit de buitenzijde, hoek afgerond met rho",
        "leverancier_vaste_voorkanten": leverancier,
        "leverancier_kleinste_rho": {"B_185": rho_185, "tredevlak_230_B_190": rho_190},
        "leverancier_scherpe_lijn_op_andere_afstand_info": afstanden,
        "kern_nabouw_herverdeeld": kern,
        "kern_nabouw_grootste_rho": kern_rho,
        "referentie_v6_grootste_rho": ref_rho,
        "referentie_v6_herverdeeld": ref_rijen,
    });
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/trap-core");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("klimlijn_afronding.json"), serde_json::to_string_pretty(&uit).unwrap()).unwrap();
    let kleuren = [(0.0, "#c00"), (50.0, "#e80"), (200.0, "#090"), (572.0, "#06c")];
    std::fs::write(dir.join("klimlijn_afronding_leverancier.svg"), svg(&cv, &lijnen, &spil, &kleuren)).unwrap();
}

fn cv_binnen() -> Polylijn {
    Polylijn::nieuw(vec![[2674.1, 549.0], [921.0, 549.0], [921.0, 53.0]]).unwrap()
}
fn cv_buiten() -> Polylijn {
    Polylijn::nieuw(vec![[2674.1, 1421.0], [49.0, 1421.0], [49.0, 53.0]]).unwrap()
}

/// Een straal die niet in de hoek past (raakpunt buiten de arm) wordt geweigerd.
#[test]
fn te_grote_straal_geweigerd() {
    assert!(Contour::nieuw(cv_binnen(), cv_buiten(), Kant::Links, 300.0, Looplijnhoek::Boog, Some(5000.0)).is_err());
    assert!(Contour::nieuw(cv_binnen(), cv_buiten(), Kant::Links, 300.0, Looplijnhoek::Boog, Some(-1.0)).is_err());
    let s = Contour::nieuw(cv_binnen(), cv_buiten(), Kant::Links, 300.0, Looplijnhoek::Boog, Some(0.0)).unwrap();
    assert!((s.looplijn.grootste_knik() - 90.0).abs() < 1e-6);
}
