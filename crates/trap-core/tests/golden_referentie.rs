//! Golden test 1 (O-2026-10-30-03): referentietrap (kwartdraai, fuik, looplijn
//! r 500, trappaal) uit de rekenkern tegen de Python-referentie IFC v5. De
//! referentiegeometrie komt uit een echt project en staat lokaal
//! (`tests/fixtures.local/trap/referentie_kwartdraai_v5.json`); zonder die
//! fixture slaan de vergelijkingen over. De invoer is geanonimiseerd.
//!
//! Gelijk binnen 0,1 mm3 / 0,1 mm: treden, welstuk, stootborden, binnenboom
//! (profiel en nesten), trappaal, Clearance. De buitenboom wijkt bewust af
//! (koude hoek i.p.v. verstek): daar eigen tests.
//!
//! Schrijft ook `target/trap-core/trap_referentie_kern.ifc` (+ toets-JSON) voor de
//! IFC-vergelijking met ifcopenshell (`tests/golden/vergelijk.py`).

mod common;

use std::path::PathBuf;

use trap_core::boom::{profiel_oppervlak, El};
use trap_core::clearance::{net_volume, open_randen};
use trap_core::model::{Prisma, Profielelement, Trapmodel, P2};
use trap_core::onderdelen::{overlap_onderling, volume};
use trap_core::toets::Oordeel;
use trap_core::vlak::doorsnede_oppervlak;
use trap_core::{ontwerp, schrijf_ifc, IfcOpties, Invoer, DISCLAIMER};

const INVOER: &str = include_str!("fixtures/invoer_kwartdraai_referentie.json");
const TOL_V: f64 = 0.1;
const TOL_B: f64 = 0.1;

fn kern() -> trap_core::Ontwerp {
    let inv: Invoer = serde_json::from_str(INVOER).expect("invoer");
    ontwerp(&inv).expect("ontwerp")
}

fn v5() -> Option<Trapmodel> {
    common::lokaal("referentie_kwartdraai_v5.json").map(|s| serde_json::from_str(&s).expect("v5-fixture"))
}

fn bbox2(p: &[P2]) -> [f64; 4] {
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for q in p {
        b[0] = b[0].min(q[0]);
        b[1] = b[1].min(q[1]);
        b[2] = b[2].max(q[0]);
        b[3] = b[3].max(q[1]);
    }
    b
}

fn dbbox(a: [f64; 4], b: [f64; 4]) -> f64 {
    (0..4).map(|i| (a[i] - b[i]).abs()).fold(0.0, f64::max)
}

fn vergelijk_prismas(soort: &str, kern: &[Prisma], ref_: &[Prisma]) -> (f64, f64) {
    assert_eq!(kern.len(), ref_.len(), "{soort}: aantal");
    let (mut dv, mut db) = (0.0f64, 0.0f64);
    for (k, r) in kern.iter().zip(ref_) {
        assert_eq!(k.naam, r.naam);
        let v = (volume(k) - volume(r)).abs();
        let b = dbbox(bbox2(&k.veelhoek), bbox2(&r.veelhoek))
            .max((k.z0 - r.z0).abs())
            .max((k.z1 - r.z1).abs());
        assert!(v < TOL_V && b < TOL_B, "{}: dV {v} mm3, dbbox {b} mm", k.naam);
        dv = dv.max(v);
        db = db.max(b);
    }
    (dv, db)
}

fn el(e: &Profielelement) -> El {
    match *e {
        Profielelement::Lijn { p0, p1 } => El::Lijn { p0, p1 },
        Profielelement::Boog { p0, p1, centrum, r, boven } => El::Boog { p0, p1, c: centrum, r, boven },
    }
}

/// (volume, bbox van het profiel in (s, z), bemonsterd over de bogen)
fn deelmaat(d: &trap_core::model::BoomdeelMetNesten) -> (f64, [f64; 4]) {
    let els: Vec<El> = d.deel.profiel.iter().map(el).collect();
    // alleen het deel van de nesten binnen het boomdeel (de kern laat nesten 1 mm
    // voorbij de einden doorlopen, zie boom::NEST_MARGE_EIND)
    let voet = trap_core::boom::deel_voetafdruk(&d.deel);
    let v = profiel_oppervlak(&els).abs() * d.deel.dikte
        - trap_core::onderdelen::nesten_in_vlak(&d.nesten, &voet, 1e-3).iter().map(volume).sum::<f64>();
    let mut pts = Vec::new();
    for e in &els {
        let (p, q) = (e.p0(), e.p1());
        for i in 0..=64 {
            let u = p[0] + (q[0] - p[0]) * i as f64 / 64.0;
            pts.push([u, e.z(u)]);
        }
    }
    (v, bbox2(&pts))
}

#[test]
fn treden_en_stootborden_gelijk_aan_v5() {
    let Some(r) = v5() else { return };
    let k = kern().model;
    let (dv1, db1) = vergelijk_prismas("treden", &k.treden, &r.treden);
    let (dv2, db2) = vergelijk_prismas("stootborden", &k.borden, &r.borden);
    println!("treden max dV {dv1:.2e} dbbox {db1:.2e}; stootborden max dV {dv2:.2e} dbbox {db2:.2e}");
}

#[test]
fn binnenboom_gelijk_aan_v5() {
    let Some(r) = v5() else { return };
    let k = kern().model;
    let (bk, br) = (&k.bomen[0], &r.bomen[0]);
    assert_eq!(bk.naam, "binnenboom");
    assert_eq!(bk.delen.len(), br.delen.len());
    for (dk, dr) in bk.delen.iter().zip(&br.delen) {
        assert_eq!(dk.deel.naam, dr.deel.naam);
        assert_eq!(dk.nesten.len(), dr.nesten.len(), "{}: nesten", dk.deel.naam);
        let ((vk, bbk), (vr, bbr)) = (deelmaat(dk), deelmaat(dr));
        let dorig = dk.deel.oorsprong.iter().zip(&dr.deel.oorsprong).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
        println!("{}: V {vr:.1} dV {:.2e} dbbox {:.2e}", dk.deel.naam, vk - vr, dbbox(bbk, bbr));
        assert!((vk - vr).abs() < TOL_V, "{}: dV {}", dk.deel.naam, vk - vr);
        assert!(dbbox(bbk, bbr) < TOL_B && dorig < TOL_B, "{}: bbox", dk.deel.naam);
        let voet = trap_core::boom::deel_voetafdruk(&dk.deel);
        for (nk, nr) in dk.nesten.iter().zip(&dr.nesten) {
            assert_eq!(nk.naam, nr.naam);
            let binnen: f64 = trap_core::onderdelen::nesten_in_vlak(std::slice::from_ref(nk), &voet, 1e-3).iter().map(volume).sum();
            assert!((binnen - volume(nr)).abs() < TOL_V, "{}: {} vs {}", nk.naam, binnen, volume(nr));
        }
    }
}

#[test]
fn trappaal_gelijk_aan_v5() {
    let Some(r) = v5() else { return };
    let k = kern().model;
    assert_eq!(k.palen.len(), 1);
    let (pk, pr) = (&k.palen[0], &r.palen[0]);
    vergelijk_prismas("paal", std::slice::from_ref(&pk.paal), std::slice::from_ref(&pr.paal));
    vergelijk_prismas("paalnesten", &pk.nesten, &pr.nesten);
}

#[test]
fn clearance_gelijk_aan_v5() {
    let Some(r) = v5() else { return };
    let k = kern().model;
    let (ck, cr) = (k.clearance.unwrap(), r.clearance.unwrap());
    let (vk, vr) = (net_volume(&ck), net_volume(&cr));
    assert_eq!(open_randen(&ck), 0, "clearance niet gesloten");
    assert_eq!(ck.vlakken.len(), cr.vlakken.len(), "aantal vlakken");
    let mut b6 = [f64::MAX, f64::MAX, f64::MAX, f64::MIN, f64::MIN, f64::MIN];
    let mut r6 = b6;
    for p in &ck.hoekpunten {
        for i in 0..3 {
            b6[i] = b6[i].min(p[i]);
            b6[i + 3] = b6[i + 3].max(p[i]);
        }
    }
    for p in &cr.hoekpunten {
        for i in 0..3 {
            r6[i] = r6[i].min(p[i]);
            r6[i + 3] = r6[i + 3].max(p[i]);
        }
    }
    let db = (0..6).map(|i| (b6[i] - r6[i]).abs()).fold(0.0, f64::max);
    println!("clearance V {vr:.0} mm3, dV {:.3e}, dbbox {db:.2e}", vk - vr);
    assert!((vk - vr).abs() < TOL_V && db < TOL_B);
}

/// De buitenboom: koude hoek. Gesloten profielen, geen overlap, het hogere deel
/// loopt `dikte` door en het lagere sluit er volledig tegen aan.
#[test]
fn buitenboom_koude_hoek() {
    let m = kern().model;
    let b = &m.bomen[1];
    assert_eq!(b.naam, "buitenboom");
    assert_eq!(b.delen.len(), 2);
    let (d1, d2) = (&b.delen[0].deel, &b.delen[1].deel);
    let dikte = d1.dikte;
    let s = |d: &trap_core::model::Boomdeel| {
        let s: Vec<f64> = d.profiel.iter().map(|e| e.p0()[0]).collect();
        (s.iter().copied().fold(f64::MAX, f64::min), s.iter().copied().fold(f64::MIN, f64::max))
    };
    // gesloten profiel, positief volume
    for d in [d1, d2] {
        let n = d.profiel.len();
        for i in 0..n {
            let (a, c) = (d.profiel[i].p1(), d.profiel[(i + 1) % n].p0());
            assert!((a[0] - c[0]).hypot(a[1] - c[1]) < 1e-9, "{}: profiel open", d.naam);
        }
        let els: Vec<El> = d.profiel.iter().map(el).collect();
        assert!(profiel_oppervlak(&els).abs() > 1e4, "{}", d.naam);
    }
    // lengtes: arm 1 = 1451,32 (stomp op de hoek); arm 2 = 2376,33 + welstuk 80 + dikte
    let (s1, s2) = (s(d1), s(d2));
    assert!((s1.0 - 0.0).abs() < 1e-6 && (s1.1 - 1451.32).abs() < 1e-6, "{s1:?}");
    assert!((s2.0 + dikte).abs() < 1e-6 && (s2.1 - (2376.33 + 80.0)).abs() < 1e-6, "{s2:?}");
    // geen overlap in plattegrond
    let voet = |d: &trap_core::model::Boomdeel| trap_core::boom::deel_voetafdruk(d);
    let overlap = doorsnede_oppervlak(&voet(d1), &voet(d2)).unwrap();
    assert!(overlap < 1e-6, "overlap boomdelen {overlap} mm2");
    // aansluiting: eindvlak deel 1 ligt in het binnenvlak van deel 2 (lijn y = hoek)
    let eind1 = [
        d1.oorsprong[0] + s1.1 * d1.richting[0],
        d1.oorsprong[1] + s1.1 * d1.richting[1],
    ];
    let afst = ((eind1[0] - d2.oorsprong[0]) * d2.naar_buiten[0] + (eind1[1] - d2.oorsprong[1]) * d2.naar_buiten[1]).abs();
    assert!(afst < 0.1, "aansluiting {afst} mm");
    // de borst van deel 1 ligt geheel tegen deel 2: z-bereik van deel 2 over s in
    // [-dikte, 0] omvat het eindvlak van deel 1
    let z_bij = |d: &trap_core::model::Boomdeel, s: f64| {
        let z: Vec<f64> = d
            .profiel
            .iter()
            .filter(|e| (e.p0()[0] - s).abs() < 1e-6)
            .flat_map(|e| [e.p0()[1]])
            .chain(d.profiel.iter().filter(|e| (e.p1()[0] - s).abs() < 1e-6).map(|e| e.p1()[1]))
            .collect();
        (z.iter().copied().fold(f64::MAX, f64::min), z.iter().copied().fold(f64::MIN, f64::max))
    };
    let (lo1, hi1) = z_bij(d1, s1.1);
    let els2: Vec<El> = d2.profiel.iter().map(el).collect();
    for i in 0..=40 {
        let sv = -dikte + dikte * f64::from(i) / 40.0;
        // boven- en onderrand van deel 2 bij sv
        let mut zs: Vec<f64> = Vec::new();
        for e in &els2 {
            let (a, c) = (e.p0()[0].min(e.p1()[0]), e.p0()[0].max(e.p1()[0]));
            if a - 1e-9 <= sv && sv <= c + 1e-9 && (c - a) > 1e-9 {
                zs.push(e.z(sv));
            }
        }
        let (lo2, hi2) = (zs.iter().copied().fold(f64::MAX, f64::min), zs.iter().copied().fold(f64::MIN, f64::max));
        assert!(hi2 >= hi1 - 0.1 && lo2 <= lo1 + 0.1, "s {sv}: deel 2 {lo2:.1}-{hi2:.1}, borst deel 1 {lo1:.1}-{hi1:.1}");
    }
    // geen overlap van boomdelen met treden/stootborden buiten de nesten: de nesten
    // zijn precies het overlappende deel
    for bd in &b.delen {
        for n in &bd.nesten {
            assert!(volume(n) > 0.0);
        }
    }
}

/// Negatieve controle: met neus 40 (Revit-type) i.p.v. 42 wijken de treden af.
#[test]
fn negatieve_controle_neus() {
    let mut inv: Invoer = serde_json::from_str(INVOER).unwrap();
    inv.trede.neus = 40.0;
    let k = ontwerp(&inv).unwrap().model;
    let Some(r) = v5() else { return };
    let max_dv = k
        .treden
        .iter()
        .zip(&r.treden)
        .map(|(a, b)| (volume(a) - volume(b)).abs())
        .fold(0.0, f64::max);
    assert!(max_dv > 1000.0, "negatieve controle faalt niet: {max_dv}");
}

#[test]
fn treden_en_stootborden_overlappen_niet() {
    let m = kern().model;
    let alle: Vec<Prisma> = m.treden.iter().chain(&m.borden).cloned().collect();
    let o = overlap_onderling(&alle).unwrap();
    assert!(o < 1e-3, "overlap {o} mm3");
}

/// Tot O-2026-11-12-01 koos de automatische verdrijving hier de vakken van v5
/// (0-5, 5-15). Die geven een zichtbare overgang (Wattjes: eerste verdreven
/// aantrede groter dan het reeksverschil). De keuze verkiest nu eerst vakken
/// zonder overgang; dat geeft kortere vakken, zoals de leverancier in golden 3
/// (`golden_onderkwart.rs`). De v5-vergelijking zelf gebruikt de vakken uit de
/// invoer en verandert niet.
#[test]
fn automatische_vakken_zonder_overgang() {
    let mut inv: Invoer = serde_json::from_str(INVOER).unwrap();
    inv.verdrijving.vakken = None;
    let o = ontwerp(&inv).unwrap();
    let v: Vec<(usize, usize)> = o.rapport.resultaat.vakken.iter().map(|v| (v.van, v.tot)).collect();
    println!("automatische vakken: {v:?}");
    assert_eq!(v, vec![(1, 6), (6, 9)]);
    let r = o.rapport.regels.iter().find(|x| x.id == "wattjes-verdrijving-overgang").unwrap();
    assert_eq!(r.oordeel, Oordeel::AdviesVoldoet);
    // de v5-vakken geven die overgang wel
    let v5 = kern();
    let r5 = v5.rapport.regels.iter().find(|x| x.id == "wattjes-verdrijving-overgang").unwrap();
    assert_eq!(r5.oordeel, Oordeel::AdviesAfwijking);
}

#[test]
fn toets_referentie_verbouw_meldt() {
    let o = kern();
    let r = &o.rapport;
    let get = |id: &str| r.regels.iter().find(|x| x.id == id).unwrap();
    // optrede 191,76 > 188 en aantrede < 220: verbouw -> melding, geen afkeur
    assert_eq!(get("bbl-4.26-optrede").oordeel, Oordeel::Melding);
    assert_eq!(get("bbl-4.26-aantrede").oordeel, Oordeel::Melding);
    assert_eq!(get("bbl-4.26-breedte").oordeel, Oordeel::Voldoet);
    assert!(r.regels.iter().all(|x| x.oordeel != Oordeel::VoldoetNiet), "verbouw keurt niet af");
    assert!((r.resultaat.aantrede_looplijn - 188.3154712725773).abs() < 1e-6);
}

#[test]
fn schrijft_ifc_met_disclaimer() {
    let o = kern();
    let opties = IfcOpties {
        tijd: "2026-10-30T00:00:00".into(),
        bestandsnaam: "trap_referentie_kern.ifc".into(),
        disclaimer: Some(DISCLAIMER.into()),
        ..Default::default()
    };
    let ifc = schrijf_ifc(&o.model, &opties).unwrap();
    assert!(ifc.contains("FILE_SCHEMA(('IFC4'));"));
    assert!(ifc.contains("'KBA_Status'"));
    assert!(ifc.contains("Opgesteld door Kolthof Bouwadvies."));
    assert!(!ifc.contains("IFCHALFSPACESOLID"), "geen verstek meer");
    assert!(!ifc.contains("IFCBOOLEANCLIPPINGRESULT"));
    assert!(ifc.contains(".QUARTER_WINDING_STAIR."));
    assert_eq!(ifc, schrijf_ifc(&o.model, &opties).unwrap(), "deterministisch");
    let uit = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/trap-core");
    std::fs::create_dir_all(&uit).unwrap();
    std::fs::write(uit.join("trap_referentie_kern.ifc"), &ifc).unwrap();
    std::fs::write(uit.join("trap_referentie_kern.toets.json"), serde_json::to_string_pretty(&o.rapport).unwrap()).unwrap();
}
