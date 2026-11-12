//! Treden lopen door tot het stootbord (O-2026-11-12-01, werk 2).
//!
//! Aanleiding: in een eerdere nabouw buiten trap-core liep de achterkant van
//! een trede niet door tot het stootbord erboven (kier, ook in de verdreven
//! treden). Conventie trap-core (`onderdelen.rs`): stootbord j staat OP trede j
//! (j = 0 op de vloer) en loopt tot de onderkant van trede j+1; trede j loopt
//! door tot het ACHTERvlak van stootbord j. Gemeten per stootbord:
//! - niet gedragen: oppervlak van de voetafdruk van het stootbord buiten de
//!   trede eronder (mm2), moet 0 zijn;
//! - kier: grootste afstand van een hoekpunt van het stootbord tot de trede
//!   eronder (mm), moet 0 zijn;
//! - spleet z: onderkant stootbord tegen bovenkant trede eronder, bovenkant
//!   stootbord tegen onderkant trede erboven (mm), moet 0 zijn.
//!
//! Positieve controle: hetzelfde stootbord tegen de verkeerde trede gemeten
//! geeft een kier > 0, dus de meting kan iets vinden.
//!
//! Daarnaast: de standaardtrap van het scherm (kwartdraai, alle standaardwaarden)
//! komt zonder Wattjes-afwijking door de toets (besluit Jochem 12-11-2026).

use trap_core::invoer::Invoer;
use trap_core::model::Prisma;
use trap_core::toets::Oordeel;
use trap_core::vlak::{afstand_tot_veelhoek, doorsnede_oppervlak, oppervlak, punt_in_veelhoek};
use trap_core::ontwerp;

const TOL: f64 = 1e-6;

struct Kier {
    niet_gedragen: f64,
    kier: f64,
    spleet_z: f64,
}

fn meet(bord: &Prisma, onder: &Prisma, boven: Option<&Prisma>) -> Kier {
    let a = oppervlak(&bord.veelhoek).abs();
    let gedragen = doorsnede_oppervlak(&bord.veelhoek, &onder.veelhoek).unwrap();
    let kier = bord
        .veelhoek
        .iter()
        .map(|&p| {
            if punt_in_veelhoek(p, &onder.veelhoek) {
                0.0
            } else {
                afstand_tot_veelhoek(p, &onder.veelhoek)
            }
        })
        .fold(0.0, f64::max);
    let mut spleet = (bord.z0 - onder.z1).abs();
    if let Some(b) = boven {
        spleet = spleet.max((bord.z1 - b.z0).abs());
    }
    Kier { niet_gedragen: (a - gedragen).max(0.0), kier, spleet_z: spleet }
}

/// Alle stootborden 1..n tegen de trede eronder; geeft het aantal gemeten.
fn geen_kier(inv: &Invoer) -> usize {
    let o = ontwerp(inv).expect("ontwerp");
    let (treden, borden) = (&o.model.treden, &o.model.borden);
    assert_eq!(borden.len(), treden.len(), "{}: een stootbord per trede (incl. welstuk)", inv.naam);
    let mut n = 0;
    for j in 1..borden.len() {
        // stootbord j staat op trede j = treden[j - 1]; erboven treden[j]
        let k = meet(&borden[j], &treden[j - 1], treden.get(j));
        let wie = format!("{}: stootbord {j:02} op {}", inv.naam, treden[j - 1].naam);
        assert!(k.niet_gedragen < 1e-3, "{wie}: niet gedragen {:.3} mm2", k.niet_gedragen);
        assert!(k.kier < TOL * 1e3, "{wie}: kier {:.4} mm", k.kier);
        assert!(k.spleet_z < TOL, "{wie}: spleet z {:.6} mm", k.spleet_z);
        n += 1;
    }
    // positieve controle: stootbord 2 tegen trede 4 (twee hoger) ligt er niet
    // op en geeft een kier
    let fout = meet(&borden[2], &treden[3], None);
    assert!(fout.kier > 10.0 && fout.niet_gedragen > 1000.0, "{}: positieve controle", inv.naam);
    n
}

fn invoer(json: &str) -> Invoer {
    serde_json::from_str(json).expect("invoer")
}

#[test]
fn recht_geen_kier() {
    let inv = invoer(
        r#"{"naam":"recht","niveaus":{"onder":0,"boven":2800},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[1,0],"binnenzijde":"links",
                           "armlengtes":[3300],"breedte":850}}"#,
    );
    assert!(geen_kier(&inv) >= 14);
}

#[test]
fn kwart_verdreven_geen_kier() {
    let inv = invoer(
        r#"{"naam":"kwart","niveaus":{"onder":0,"boven":2800},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[0,1],"binnenzijde":"links",
                           "armlengtes":[2400,1600],"breedte":850},"trappaal":{}}"#,
    );
    let o = ontwerp(&inv).unwrap();
    assert!(o.rapport.resultaat.voorkanten.iter().any(|v| v.verdreven), "er moet verdreven zijn");
    assert!(geen_kier(&inv) >= 14);
}

#[test]
fn half_verdreven_geen_kier() {
    let inv = invoer(
        r#"{"naam":"half","niveaus":{"onder":0,"boven":2800},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[0,1],"binnenzijde":"rechts",
                           "armlengtes":[1500,2200,1500],"breedte":850},"trappaal":{}}"#,
    );
    assert!(geen_kier(&inv) >= 14);
}

#[test]
fn onderkwart_contour_geen_kier() {
    let inv = invoer(include_str!("fixtures/invoer_kwartdraai_onderkwart.json"));
    assert_eq!(geen_kier(&inv), 15);
}

/// De standaardtrap van het scherm (frontend `formulier.ts`, STANDAARD):
/// kwartdraai links 2400 + 1600, loopbreedte 850, 2800, trappaal, verder
/// alles standaard. Geen Wattjes-advies mag afwijken, geen Bbl-regel falen.
#[test]
fn standaardtrap_zonder_wattjes_afwijking() {
    let inv = invoer(
        r#"{"naam":"standaard","niveaus":{"onder":0,"boven":2800},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[0,1],"binnenzijde":"links",
                           "armlengtes":[2400,1600],"breedte":850},"trappaal":{}}"#,
    );
    let o = ontwerp(&inv).unwrap();
    for r in &o.rapport.regels {
        assert!(
            r.oordeel != Oordeel::VoldoetNiet && r.oordeel != Oordeel::AdviesAfwijking,
            "standaardtrap: {} = {:?} ({:?}; {})",
            r.id,
            r.oordeel,
            r.waarde,
            r.toelichting
        );
    }
    assert!(o.rapport.waarschuwingen.iter().all(|w| !w.contains("achterhout") && !w.contains("nest steekt")),
        "{:?}", o.rapport.waarschuwingen);
    // standaardwaarden besluit Jochem 11/12-11-2026
    assert_eq!(inv.boom.plankbreedte, 320.0);
    assert_eq!(inv.trede.dikte, 38.0);
    assert!((15.0..=30.0).contains(&inv.boom.voorhout));
    assert_eq!(inv.looplijn.afstand_buitenkant_boom, 300.0);
}
