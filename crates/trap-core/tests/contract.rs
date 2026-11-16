//! Headless trapontwerper volgens het assemblagecontract (O-2026-11-15-05, S1).
//!
//! Golden: de referentietrap (Voorstraat 172, geanonimiseerd in
//! `fixtures/invoer_kwartdraai_referentie.json`) via de contract-invoer
//! (`fixtures/contract/referentie_trap.json` + `referentie_project.json`) geeft
//! EXACT hetzelfde trapmodel als via de gewone invoer; dat model is in
//! `golden_referentie.rs` tegen de Python-referentie v5 getoetst (0,1 mm). Met de
//! lokale v5-fixture vergelijkt deze test ook zelf.
//!
//! Verder: GUID-regel (gouden vectoren gedeeld met de Python-referentie),
//! revisie = zelfde GUID's, sparing = minimaal trapgat, weigeringen.

mod common;

use std::collections::BTreeSet;

use trap_core::contract::{contract_guid, ContractInvoer, ProjectOorsprong};
use trap_core::model::{Prisma, Trapmodel};
use trap_core::onderdelen::volume;
use trap_core::{ontwerp, ontwerp_contract, Fout, Invoer};

const REF_GEWOON: &str = include_str!("fixtures/invoer_kwartdraai_referentie.json");
const REF_TRAP: &str = include_str!("fixtures/contract/referentie_trap.json");
const REF_PROJECT: &str = include_str!("fixtures/contract/referentie_project.json");
const TRAP01: &str = include_str!("fixtures/contract/trap-01.json");
const TRAP01_REV1: &str = include_str!("fixtures/contract/trap-01_rev1.json");
const PROJECT_TEST: &str = include_str!("fixtures/contract/project_test.json");
const TIJD: &str = "2026-11-15T20:00:00";

fn ci(json: &str) -> ContractInvoer {
    serde_json::from_str(json).expect("contract-invoer")
}

fn project(json: &str) -> ProjectOorsprong {
    serde_json::from_str(json).expect("project.json")
}

fn draai(trap: &str, proj: &str) -> trap_core::ContractUitvoer {
    ontwerp_contract(&ci(trap), &project(proj), "invoer.json", trap.as_bytes(), TIJD).expect("contract")
}

/// GlobalId's met entiteitnaam uit de STEP-tekst.
fn guids(ifc: &str) -> Vec<(String, String)> {
    ifc.lines()
        .filter_map(|l| {
            let (_, rest) = l.split_once('=')?;
            let (ent, args) = rest.split_once('(')?;
            let g = args.strip_prefix('\'')?.split('\'').next()?;
            // alleen IfcRoot-objecten: een eigenschapsnaam kan ook 22 tekens zijn
            (g.len() == 22 && ent != "IFCPROPERTYSINGLEVALUE").then(|| (ent.to_string(), g.to_string()))
        })
        .collect()
}

fn geometrie(m: &Trapmodel) -> serde_json::Value {
    serde_json::json!({
        // palen zonder eigenschappen: daar staat het materiaal (vurenhout -> NAA.K.T.)
        "treden": m.treden, "borden": m.borden, "bomen": m.bomen,
        "palen": m.palen.iter().map(|p| serde_json::json!([p.paal, p.nesten])).collect::<Vec<_>>(),
        "clearance": m.clearance, "pset_trap": m.pset_trap, "trap": {
            "vorm": m.trap.vorm, "steek": m.trap.steek, "optreden": m.trap.optreden,
            "aantreden": m.trap.aantreden, "optrede": m.trap.optrede, "aantrede": m.trap.aantrede
        }
    })
}

#[test]
fn golden_voorstraat_contract_geeft_zelfde_trapmodel() {
    let gewoon: Invoer = serde_json::from_str(REF_GEWOON).unwrap();
    let a = ontwerp(&gewoon).unwrap();
    let inv = ci(REF_TRAP).naar_invoer(&project(REF_PROJECT)).unwrap();
    let b = ontwerp(&inv).unwrap();
    // niveaus en vloerdikte komen nu uit project.json
    assert_eq!((inv.niveaus.onder, inv.niveaus.boven), (gewoon.niveaus.onder, gewoon.niveaus.boven));
    assert_eq!(inv.niveaus.vloerdikte_boven, gewoon.niveaus.vloerdikte_boven);
    assert_eq!(geometrie(&a.model), geometrie(&b.model), "geometrie wijkt af");
    let mat = |m: &Trapmodel| -> Vec<String> {
        m.palen.iter().flat_map(|p| p.eigenschappen.iter().filter(|(k, _)| k == "Materiaal").map(|(_, w)| format!("{w:?}"))).collect()
    };
    assert_eq!(mat(&a.model), vec!["Label(\"vurenhout\")".to_string()]);
    assert_eq!(mat(&b.model), vec!["Label(\"hout_vuren_generiek\")".to_string()]);
    assert_eq!(
        serde_json::to_value(&a.rapport.regels).unwrap(),
        serde_json::to_value(&b.rapport.regels).unwrap(),
        "toetsing wijkt af"
    );
    assert_eq!(a.rapport.trapgat_minimaal, b.rapport.trapgat_minimaal);
}

fn vergelijk(soort: &str, k: &[Prisma], r: &[Prisma]) {
    assert_eq!(k.len(), r.len(), "{soort}: aantal");
    for (a, b) in k.iter().zip(r) {
        assert_eq!(a.naam, b.naam);
        assert!((volume(a) - volume(b)).abs() < 0.1, "{}: volume", a.naam);
        assert!((a.z0 - b.z0).abs() < 0.1 && (a.z1 - b.z1).abs() < 0.1, "{}: z", a.naam);
    }
}

#[test]
fn golden_voorstraat_lokaal_tegen_v5() {
    let Some(v5) = common::lokaal("referentie_kwartdraai_v5.json") else { return };
    let r: Trapmodel = serde_json::from_str(&v5).unwrap();
    let inv = ci(REF_TRAP).naar_invoer(&project(REF_PROJECT)).unwrap();
    let k = ontwerp(&inv).unwrap().model;
    vergelijk("treden", &k.treden, &r.treden);
    vergelijk("stootborden", &k.borden, &r.borden);
    assert_eq!((k.trap.optreden, k.trap.aantreden), (r.trap.optreden, r.trap.aantreden));
}

#[test]
fn contract_ifc_ruimtelijke_guids_volgens_python_vectoren() {
    let u = draai(TRAP01, PROJECT_TEST);
    let g: BTreeSet<String> = guids(&u.ifc).into_iter().map(|(_, g)| g).collect();
    // gouden vectoren uit ref/tests/test_contract_guid.py
    assert!(g.contains("1GTcDAeNfIgRaWIqTGi90h"), "IfcSite");
    assert!(g.contains("2lDP_OR2XUCPKlhRig6pwi"), "IfcBuildingStorey 00");
    assert!(g.contains("0SFDsN65bISe_k$pWngdAB"), "IfcStair trap-01");
    assert!(g.contains("3KH5pO4orPpg_X3PNaTWIo"), "trede 01");
    assert_eq!(g.len(), guids(&u.ifc).len(), "GUID's uniek");
    assert!(u.ifc.contains("IFCMAPCONVERSION("));
    assert!(u.ifc.contains("'EPSG:28992'"));
    assert!(u.ifc.contains("IFCSTAIRFLIGHT(") && u.ifc.contains(",'steek',"), "staptitel 'steek'");
    assert!(!u.ifc.contains("vurenhout"), "alleen NAA.K.T.-materiaal");
    assert!(u.ifc.contains("'hout_vuren_generiek'"));
}

#[test]
fn revisie_zelfde_guids_andere_maat() {
    let a = draai(TRAP01, PROJECT_TEST);
    let b = draai(TRAP01_REV1, PROJECT_TEST);
    assert_ne!(a.ifc, b.ifc);
    assert_eq!(guids(&a.ifc), guids(&b.ifc));
}

#[test]
fn trapnaam_wijzigt_guids_niet() {
    let a = draai(TRAP01, PROJECT_TEST);
    let b = draai(&TRAP01.replace("\"naam\": \"trap 01\"", "\"naam\": \"hoofdtrap\""), PROJECT_TEST);
    assert!(b.ifc.contains("'hoofdtrap'"));
    assert_eq!(guids(&a.ifc), guids(&b.ifc));
}

#[test]
fn ander_onderdeel_andere_element_guids_zelfde_ruimte() {
    let a = draai(TRAP01, PROJECT_TEST);
    let b = draai(&TRAP01.replace("\"trap-01\"", "\"trap-02\""), PROJECT_TEST);
    let ga: BTreeSet<_> = guids(&a.ifc).into_iter().collect();
    let gb: BTreeSet<_> = guids(&b.ifc).into_iter().collect();
    let gedeeld: BTreeSet<&str> = ga.intersection(&gb).map(|(e, _)| e.as_str()).collect();
    // gedeeld: alleen project, site, gebouw, bouwlaag en hun aggregaties
    assert_eq!(
        gedeeld,
        ["IFCBUILDING", "IFCBUILDINGSTOREY", "IFCPROJECT", "IFCRELAGGREGATES", "IFCSITE"].into_iter().collect()
    );
    assert_eq!(ga.intersection(&gb).count(), 7);
}

#[test]
fn sparing_is_minimaal_trapgat_en_ligt_in_de_vloer() {
    let u = draai(TRAP01, PROJECT_TEST);
    assert_eq!(u.contract.sparing, u.rapport.trapgat_minimaal);
    assert!(u.contract.sparing.len() >= 4);
    let sp = u.ifc.lines().find(|l| l.contains("'SPARINGSVOLUME'")).expect("sparing");
    assert!(sp.contains("'sparing 01 trap-01'"));
    // vloerpakket 270 onder peil 3000: extrusie 270 vanaf z = 2730 (bouwlaag 00 op 0)
    assert!(u.ifc.contains("IFCCARTESIANPOINT((0.,0.,2730.))"));
    assert_eq!(
        u.contract.sparing_bron,
        "minimaal trapgat (trap-core: clearance boven o.k. bovenvloer)"
    );
}

#[test]
fn trapgat_uit_invoer_wordt_de_sparing() {
    let gat = "[[1600,1900],[4100,1900],[4100,2900],[2600,2900],[2600,3800],[1600,3800]]";
    let json = TRAP01.replace("\"trap\": {", &format!("\"trap\": {{\n    \"trapgat\": {gat},"));
    let u = draai(&json, PROJECT_TEST);
    assert_eq!(u.contract.sparing.len(), 6);
    assert_eq!(u.contract.sparing[0], [1600.0, 1900.0]);
    assert_eq!(u.contract.sparing_bron, "trapgat uit de invoer");
}

#[test]
fn deterministisch() {
    assert_eq!(draai(TRAP01, PROJECT_TEST).ifc, draai(TRAP01, PROJECT_TEST).ifc);
}

fn fout(json: &str, proj: &str) -> String {
    match ontwerp_contract(&ci(json), &project(proj), "x", b"", TIJD) {
        Err(Fout::Invoer(s)) => s,
        Err(e) => panic!("verkeerde fout: {e}"),
        Ok(_) => panic!("niet geweigerd"),
    }
}

#[test]
fn weigert_dubbele_bronnen_en_ongeldige_invoer() {
    let met_niveaus = TRAP01.replace("\"trap\": {", "\"trap\": {\"niveaus\": {\"onder\": 0, \"boven\": 3000},");
    assert!(fout(&met_niveaus, PROJECT_TEST).contains("trap.niveaus"));
    let met_mat = TRAP01.replace("\"langs_in\": 90,", "\"langs_in\": 90, \"materiaal\": \"vurenhout\",");
    assert!(fout(&met_mat, PROJECT_TEST).contains("houtsoort"));
    assert!(fout(&TRAP01.replace("\"trap-01\"", "\"Trap 01\""), PROJECT_TEST).contains("onderdeel"));
    assert!(fout(&TRAP01.replace("\"bouwlaag\": \"00\"", "\"bouwlaag\": \"07\""), PROJECT_TEST).contains("07"));
    let zonder_pakket = PROJECT_TEST.replace(", \"vloerpakket\": 270 }", " }");
    assert!(fout(TRAP01, &zonder_pakket).contains("vloerpakket"));
    let fout_schema = PROJECT_TEST.replace("\"kba-assemblage/1\"", "\"kba-assemblage/2\"");
    assert!(fout(TRAP01, &fout_schema).contains("schema"));
    assert!(fout(&TRAP01.replace("\"doorbreekt\": \"01\"", "\"nlsfb\": {\"code\": \"21.10\", \"omschrijving\": \"x\"}"), PROJECT_TEST).contains("NL-SfB"));
}

#[test]
fn guid_weigert_pipe_in_sleutel() {
    assert!(contract_guid("TEST|1", "trap-01", "IfcStair").is_err());
}

const TRAP02: &str = include_str!("fixtures/contract/trap-02.json");

fn oordelen(u: &trap_core::ContractUitvoer) -> Vec<(String, trap_core::toets::Oordeel)> {
    u.rapport.regels.iter().map(|g| (g.id.clone(), g.oordeel)).collect()
}

/// trap-01 is het NEGATIEVE geval (aantrede 208,5 < 220), trap-02 het
/// positieve: geen enkele Bbl-regel voldoet_niet (besluit planner 16-11).
#[test]
fn trap01_voldoet_niet_trap02_voldoet() {
    use trap_core::toets::Oordeel;
    let o1 = oordelen(&draai(TRAP01, PROJECT_TEST));
    let niet1: Vec<&String> = o1.iter().filter(|(_, o)| *o == Oordeel::VoldoetNiet).map(|(i, _)| i).collect();
    assert_eq!(niet1, vec!["bbl-4.26-aantrede"]);
    let u2 = draai(TRAP02, PROJECT_TEST);
    let o2 = oordelen(&u2);
    assert!(o2.iter().all(|(_, o)| *o != Oordeel::VoldoetNiet), "{o2:?}");
    assert!(o2.iter().any(|(i, o)| i == "bbl-4.26-aantrede" && *o == Oordeel::Voldoet));
    assert!((u2.rapport.resultaat.aantrede_looplijn - 240.0).abs() < 1e-9);
}

/// Teksten uit de inhoudsstroom (ongecomprimeerd, Helvetica).
fn pdf_teksten(pdf: &[u8]) -> Vec<String> {
    // geescapete haakjes eerst wegzetten, daarna terugzetten
    let s = String::from_utf8_lossy(pdf).replace("\\(", "\u{1}").replace("\\)", "\u{2}");
    s.split(") Tj")
        .filter_map(|d| d.rsplit_once('(').map(|(_, t)| t.replace('\u{1}', "(").replace('\u{2}', ")")))
        .collect()
}

fn pdf(trap: &str) -> Vec<String> {
    let c = ci(trap);
    let u = draai(trap, PROJECT_TEST);
    let k = trap_core::pdf::Kader { titel: "t".into(), regels: vec![] };
    pdf_teksten(&trap_core::pdf::tekening_pdf(&u.tekening, &u.rapport, Some(&u.contract.sparing), &k, c.referentie.as_ref()))
}

/// Review Jochem 15-11: legenda klopt met wat getekend is, maatvoering van
/// trapgat / loopbreedte / positie / aantrede, decimalen met komma.
#[test]
fn pdf_maatvoering_en_legenda() {
    let t1 = pdf(TRAP01);
    // trap-01: looplijn op 300 = klimlijn: een lijn, legenda zegt dat
    assert!(t1.iter().any(|t| t.starts_with("looplijn = klimlijn")), "{t1:?}");
    assert!(!t1.iter().any(|t| t.contains("klimlijn (groen")));
    assert!(t1.iter().any(|t| t == "aantrede 15 x 208,5 op de looplijn"), "{t1:?}");
    assert!(t1.iter().any(|t| t == "loopbreedte 850"));
    // trapgat 2030 x 1680 (L-vorm, omhullende) als keten en in het kader
    let u = draai(TRAP01, PROJECT_TEST);
    let xs: Vec<f64> = u.contract.sparing.iter().map(|p| p[0]).collect();
    let lx = xs.iter().copied().fold(f64::MIN, f64::max) - xs.iter().copied().fold(f64::MAX, f64::min);
    assert!(t1.iter().any(|t| *t == format!("{lx:.0}")), "trapgatmaat {lx}");
    assert!(t1.iter().any(|t| t.starts_with("trapgat ") && t.ends_with(" mm")));
    // zonder referentie: maat naar de projectoorsprong
    assert!(t1.iter().any(|t| t == "projectoorsprong"));
    // decimalen met komma, nooit een punt in een getal
    assert!(t1.iter().any(|t| t.contains("187,5")));
    // (regel-id's als bbl-4.26-aantrede zijn artikelnummers, geen getallen)
    assert!(!t1.iter().filter(|t| !t.contains("bbl-")).any(|t| t.chars().collect::<Vec<_>>().windows(3).any(|w| w[0].is_ascii_digit() && w[1] == '.' && w[2].is_ascii_digit())), "punt-decimaal: {t1:?}");

    // trap-02 met referentie: ketens naar ruwe wand en stramien, geen oorsprong
    let t2 = pdf(TRAP02);
    assert!(t2.iter().any(|t| t == "ruwe wand"));
    assert!(t2.iter().any(|t| t == "stramien"));
    assert!(!t2.iter().any(|t| t == "projectoorsprong"));
    assert!(t2.iter().any(|t| t == "aantrede 15 x 240 op de looplijn"), "{t2:?}");
}
