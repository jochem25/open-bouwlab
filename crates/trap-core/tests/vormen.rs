//! Trapvormen en invoervarianten buiten de golden cases: rechte trap,
//! halfdraai, open trap, buitenwerkse maten, automatisch aantal optreden en
//! foutgevallen.

use trap_core::clearance::open_randen;
use trap_core::invoer::Invoer;
use trap_core::onderdelen::overlap_onderling;
use trap_core::toets::Oordeel;
use trap_core::{ontwerp, schrijf_ifc, Fout, IfcOpties};

fn invoer(json: &str) -> Invoer {
    serde_json::from_str(json).expect("invoer")
}

fn controleer(inv: &Invoer) -> trap_core::Ontwerp {
    let o = ontwerp(inv).expect("ontwerp");
    let m = &o.model;
    let alle: Vec<_> = m.treden.iter().chain(&m.borden).cloned().collect();
    assert!(overlap_onderling(&alle).unwrap() < 1e-3, "overlap treden/stootborden");
    assert_eq!(open_randen(m.clearance.as_ref().unwrap()), 0, "clearance open");
    let ifc = schrijf_ifc(m, &IfcOpties::default()).unwrap();
    assert!(ifc.contains("IFCSTAIR("));
    o
}

#[test]
fn rechte_trap_automatisch_optreden() {
    let inv = invoer(
        r#"{"naam":"recht","niveaus":{"onder":0,"boven":2800},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[1,0],"binnenzijde":"links",
                           "armlengtes":[3300],"breedte":850}}"#,
    );
    let o = controleer(&inv);
    let r = &o.rapport.resultaat;
    // 2800/188 = 14,9 -> minstens 15; stapmaat dicht bij 630
    assert!(r.optreden >= 15, "{}", r.optreden);
    assert!(r.optrede <= 188.0 + 1e-9);
    assert_eq!(o.model.trap.vorm, "STRAIGHT_RUN_STAIR");
    assert_eq!(o.model.trap.steek, "STRAIGHT");
    assert!(o.model.palen.is_empty());
    assert_eq!(o.model.bomen[0].delen.len(), 1);
    assert!(r.vakken.is_empty());
    // rechte trap: alle aantreden gelijk, loodrecht = langs de looplijn
    for a in &r.aantreden_loodrecht {
        assert!((a - r.aantrede_looplijn).abs() < 1e-6);
    }
}

#[test]
fn halfdraai_met_twee_palen() {
    let inv = invoer(
        r#"{"naam":"halfdraai","niveaus":{"onder":0,"boven":3000},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[0,1],"binnenzijde":"rechts",
                           "armlengtes":[1500,2200,1500],"breedte":850},
            "optreden":16,"trappaal":{}}"#,
    );
    let o = controleer(&inv);
    assert_eq!(o.model.trap.vorm, "HALF_WINDING_STAIR");
    assert_eq!(o.model.palen.len(), 2);
    assert_eq!(o.model.bomen[0].delen.len(), 3);
    assert_eq!(o.model.bomen[1].delen.len(), 3);
    // per hoek een tusschenkwart; vakken van de twee hoeken overlappen niet
    let v = &o.rapport.resultaat.vakken;
    assert!(v.len() >= 3, "{v:?}");
    assert!(v.windows(2).all(|w| w[0].tot <= w[1].van), "{v:?}");
    // buitenboom: per hoek loopt het hogere deel door
    let d = &o.rapport.resultaat.boomdelen;
    let door: Vec<bool> = d.iter().filter(|b| b.naam.starts_with("buiten")).map(|b| b.doorlopend).collect();
    assert_eq!(door, vec![false, true, true]);
}

/// Middenarm van de halfdraai te kort (binnen 200 mm): geen automatische
/// verdrijving, duidelijke melding (vakken dan met de hand opgeven).
#[test]
fn halfdraai_te_korte_middenarm_meldt() {
    let inv = invoer(
        r#"{"naam":"halfdraai","niveaus":{"onder":0,"boven":3000},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[0,1],"binnenzijde":"rechts",
                           "armlengtes":[1500,1900,1500],"breedte":850},
            "optreden":16,"trappaal":{}}"#,
    );
    assert!(matches!(ontwerp(&inv), Err(Fout::Ontwerp(_))));
}

#[test]
fn open_trap_zonder_stootborden() {
    let inv = invoer(
        r#"{"naam":"open","niveaus":{"onder":0,"boven":2700},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[1,0],"binnenzijde":"links",
                           "armlengtes":[2400,1400],"breedte":800},
            "stootbord":{"soort":"open"},"trappaal":{}}"#,
    );
    let o = controleer(&inv);
    assert!(o.model.borden.is_empty());
    assert!(o.rapport.regels.iter().all(|r| r.id != "wattjes-stootbord-hoek"));
}

#[test]
fn buitenwerkse_maten_gelijk_aan_loopbreedte() {
    let a = invoer(
        r#"{"naam":"lb","niveaus":{"onder":0,"boven":2700},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[1,0],"binnenzijde":"links",
                           "armlengtes":[2400,1400],"breedte":800},"trappaal":{}}"#,
    );
    // buitenwerks: start op de buitenkant van de buitenboom, maten 40 per hoek groter
    let b = invoer(
        r#"{"naam":"lb","niveaus":{"onder":0,"boven":2700},
            "plattegrond":{"soort":"armen","start":[0,-40],"richting":[1,0],"binnenzijde":"links",
                           "armlengtes":[2440,1440],"breedte":880,"breedte_is":"buitenwerks"},
            "trappaal":{}}"#,
    );
    let oa = controleer(&a);
    let ob = controleer(&b);
    let (ra, rb) = (&oa.rapport.resultaat, &ob.rapport.resultaat);
    assert!((ra.loopbreedte - rb.loopbreedte).abs() < 1e-9);
    assert!((ra.aantrede_looplijn - rb.aantrede_looplijn).abs() < 1e-9);
}

#[test]
fn binnenhoek_zonder_paal_faalt() {
    let inv = invoer(
        r#"{"naam":"x","niveaus":{"onder":0,"boven":2700},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[1,0],"binnenzijde":"links",
                           "armlengtes":[2400,1400],"breedte":800}}"#,
    );
    assert!(matches!(ontwerp(&inv), Err(Fout::Invoer(_))));
}

#[test]
fn te_smalle_trap_voldoet_niet_bij_nieuwbouw() {
    let inv = invoer(
        r#"{"naam":"smal","niveaus":{"onder":0,"boven":2700},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[1,0],"binnenzijde":"links",
                           "armlengtes":[3600],"breedte":700}}"#,
    );
    let o = controleer(&inv);
    let r = o.rapport.regels.iter().find(|r| r.id == "bbl-4.26-breedte").unwrap();
    assert_eq!(r.oordeel, Oordeel::VoldoetNiet);
    // 700 breed: klimlijnband 300 .. 400 uit de buitenzijde
    assert_eq!(o.rapport.resultaat.meting.klimlijn_band, Some([300.0, 400.0]));
}

#[test]
fn trapgat_controle() {
    let basis = r#"{"naam":"gat","niveaus":{"onder":0,"boven":2800,"vloerdikte_boven":250},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[1,0],"binnenzijde":"links",
                           "armlengtes":[3300],"breedte":850}"#;
    let ruim = invoer(&format!(
        r#"{basis},"trapgat":[[-100,-100],[3500,-100],[3500,1000],[-100,1000]]}}"#
    ));
    let o = controleer(&ruim);
    let g = &o.rapport.trapgat_minimaal;
    assert!(g.len() >= 4);
    let r = |o: &trap_core::Ontwerp| o.rapport.regels.iter().find(|r| r.id == "bbl-4.26-vrije-hoogte-trapgat").unwrap().oordeel;
    assert_eq!(r(&o), Oordeel::Voldoet);
    let krap = invoer(&format!(r#"{basis},"trapgat":[[2500,-100],[3500,-100],[3500,1000],[2500,1000]]}}"#));
    assert_eq!(r(&controleer(&krap)), Oordeel::VoldoetNiet);
    // het minimale trapgat begint waar de clearance de onderkant van de vloer raakt:
    // bovenkant clearance = neus + 2300 >= 2800 - 250 -> neus >= 250
    let xmin = g.iter().map(|p| p[0]).fold(f64::MAX, f64::min);
    assert!(xmin > 0.0 && xmin < 3300.0, "{xmin}");
}
