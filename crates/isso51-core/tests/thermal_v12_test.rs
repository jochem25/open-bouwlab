//! thermal.json v1.2: `pair_id` op ruimte-aan-ruimte-scheidingen die van
//! BEIDE kanten gemeten zijn. Elke ruimte rekent met zijn eigen kant (eigen
//! ISSO 51-vertrekmaat); de import spiegelt zo'n constructie niet.
//!
//! Synthetische fixture: twee verwarmde ruimten met ongelijke vrije hoogte
//! (2,6 en 3,0 m) en één binnenwand van 4,0 m lang tussen beide. Van ruimte A
//! gezien is de wand 4,0 · 2,6 = 10,4 m², van ruimte B 4,0 · 3,0 = 12,0 m².
//! In de wand zit een deur van 0,9 × 2,1 m = 1,89 m², per kant geleverd.
//!
//! Verwachting met de hand (U = 1,0 voor alle elementen, θ_A = 20 °C,
//! θ_B = 16 °C, wand: f_ia = (θi − θa)/(θi − θe), ISSO 51:2023 form. 2.17):
//! - A: wand netto 10,4 − 1,89 = 8,51 m² + deur 1,89 m² = 10,4 m² naar B;
//!   H_T,ia,A = 10,4 · 1 · (20 − 16)/(20 − θe).
//! - B: 12,0 − 1,89 = 10,11 m² + 1,89 m² = 12,0 m² naar A;
//!   H_T,ia,B = 12,0 · 1 · (16 − 20)/(16 − θe).
//! In v1.1 (zonder pair_id, beide kanten geleverd) spiegelt de import en
//! krijgt elke ruimte 10,4 + 12,0 = 22,4 m²: dat is de dubbeltelling die
//! v1.2 oplost.
//!
//! Spec: `docs/thermal-import-construction-catalog-spec.md` (sectie v1.2).

use isso51_core::calculate;
use isso51_core::import::{map_thermal_import, ThermalImport, ThermalImportResult};
use isso51_core::model::enums::BoundaryType;
use serde_json::{json, Value};

const PAIR: &str = "p-0123456789abcdef";
const EPS: f64 = 1e-6;

fn layers() -> Value {
    json!([{ "material": "Kalkzandsteen", "thickness_mm": 100.0, "distance_from_interior_mm": 0.0, "type": "solid" }])
}

fn wall(id: &str, a: &str, b: &str, area: f64, pair: Option<&str>) -> Value {
    let mut v = json!({ "id": id, "room_a": a, "room_b": b, "orientation": "wall",
                        "gross_area_m2": area, "layers": layers() });
    if let Some(p) = pair {
        v["pair_id"] = json!(p);
    }
    v
}

fn door(id: &str, construction_id: &str) -> Value {
    json!({ "id": id, "construction_id": construction_id, "type": "door",
            "width_mm": 900.0, "height_mm": 2100.0 })
}

fn import(version: &str, constructions: Vec<Value>, openings: Vec<Value>) -> ThermalImportResult {
    let t = json!({
        "version": version, "source": "ifc", "exported_at": "2026-10-06T00:00:00Z",
        "rooms": [
            { "id": "a", "name": "A", "type": "heated", "area_m2": 12.0, "height_m": 2.6 },
            { "id": "b", "name": "B", "type": "heated", "area_m2": 12.0, "height_m": 3.0 }
        ],
        "constructions": constructions,
        "openings": openings
    });
    let ti: ThermalImport = serde_json::from_value(t).expect("fixture moet deserialiseren");
    map_thermal_import(ti)
}

fn v12_beide_kanten() -> ThermalImportResult {
    import(
        "1.2",
        vec![wall("wa", "a", "b", 10.4, Some(PAIR)), wall("wb", "b", "a", 12.0, Some(PAIR))],
        vec![door("da", "wa"), door("db", "wb")],
    )
}

/// Som van het oppervlak naar de buurruimte en het aantal deuren.
fn adjacent(r: &ThermalImportResult, room: &str) -> (f64, usize) {
    let room = r.project.rooms.iter().find(|x| x.id == room).expect("ruimte");
    let adj: Vec<_> = room
        .constructions
        .iter()
        .filter(|c| c.boundary_type == BoundaryType::AdjacentRoom)
        .collect();
    let doors = adj.iter().filter(|c| c.description.contains("deur")).count();
    (adj.iter().map(|c| c.area).sum(), doors)
}

#[test]
fn v12_elke_ruimte_eigen_kant() {
    let r = v12_beide_kanten();
    let (area_a, doors_a) = adjacent(&r, "a");
    let (area_b, doors_b) = adjacent(&r, "b");
    assert!((area_a - 10.4).abs() < EPS, "A: {area_a}");
    assert!((area_b - 12.0).abs() < EPS, "B: {area_b}");
    assert_eq!(doors_a, 1, "A: een deur (eigen kant)");
    assert_eq!(doors_b, 1, "B: een deur (eigen kant)");
}

#[test]
fn v12_transmissie_per_ruimte_uit_eigen_kant() {
    let mut r = v12_beide_kanten();
    for room in &mut r.project.rooms {
        room.custom_temperature = Some(if room.id == "a" { 20.0 } else { 16.0 });
        for c in &mut room.constructions {
            c.u_value = 1.0;
        }
    }
    let theta_e = r.project.climate.theta_e;
    let res = calculate(&r.project).expect("berekening");
    let h = |id: &str| {
        res.rooms
            .iter()
            .find(|x| x.room_id == id)
            .expect("resultaat")
            .transmission
            .h_t_adjacent_rooms
    };
    let verwacht_a = 10.4 * (20.0 - 16.0) / (20.0 - theta_e);
    let verwacht_b = 12.0 * (16.0 - 20.0) / (16.0 - theta_e);
    assert!((h("a") - verwacht_a).abs() < 1e-9, "A: {} vs {}", h("a"), verwacht_a);
    assert!((h("b") - verwacht_b).abs() < 1e-9, "B: {} vs {}", h("b"), verwacht_b);
}

/// Dezelfde levering zonder pair_id (v1.1) wordt gespiegeld: de dubbeltelling
/// die v1.2 oplost. Oppervlakken verschillen > 5 %, dus geen
/// dubbel-geleverd-waarschuwing; dat is precies waarom v1.2 nodig is.
#[test]
fn v11_zelfde_levering_wordt_gespiegeld() {
    let r = import(
        "1.1",
        vec![wall("wa", "a", "b", 10.4, None), wall("wb", "b", "a", 12.0, None)],
        vec![door("da", "wa"), door("db", "wb")],
    );
    let (area_a, doors_a) = adjacent(&r, "a");
    assert!((area_a - 22.4).abs() < EPS, "A: {area_a}");
    assert_eq!(doors_a, 2);
}

#[test]
fn v12_qc_oppervlak_per_kant_verschilt() {
    let r = v12_beide_kanten();
    let qc: Vec<_> = r.warnings.iter().filter(|w| w.contains(PAIR) && w.contains("verschilt")).collect();
    assert_eq!(qc.len(), 1, "{:?}", r.warnings);
    assert!(qc[0].contains("13 %"), "{}", qc[0]);
    assert!(!qc[0].contains("  "), "dubbele spatie: {}", qc[0]);
    assert!(!r.warnings.iter().any(|w| w.contains("van beide kanten")), "geen v1.1-melding bij pair_id");
}

#[test]
fn v12_qc_een_kant_ontbreekt() {
    let r = import("1.2", vec![wall("wa", "a", "b", 10.4, Some(PAIR))], vec![]);
    assert!(r.warnings.iter().any(|w| w.contains(PAIR) && w.contains("alleen de kant van 'a'")), "{:?}", r.warnings);
    let (area_b, _) = adjacent(&r, "b");
    assert_eq!(area_b, 0.0, "constructie met pair_id wordt niet gespiegeld");
}

#[test]
fn v12_gelijke_kanten_geen_qc() {
    let r = import(
        "1.2",
        vec![wall("wa", "a", "b", 10.0, Some(PAIR)), wall("wb", "b", "a", 10.2, Some(PAIR))],
        vec![],
    );
    assert!(!r.warnings.iter().any(|w| w.contains(PAIR)), "{:?}", r.warnings);
}

/// pair_id in een v1.1-bestand: genegeerd met waarschuwing, v1.1-gedrag.
#[test]
fn pair_id_in_v11_genegeerd() {
    let r = import("1.1", vec![wall("wa", "a", "b", 10.4, Some(PAIR))], vec![]);
    assert!(r.warnings.iter().any(|w| w.contains("pair_id vereist 1.2")), "{:?}", r.warnings);
    let (area_b, _) = adjacent(&r, "b");
    assert!((area_b - 10.4).abs() < EPS, "gespiegeld zoals v1.1: {area_b}");
}

/// Mengen mag: een paar met pair_id naast een (holte-)scheiding zonder.
#[test]
fn v12_gemengd_met_en_zonder_pair_id() {
    let r = import(
        "1.2",
        vec![
            wall("wa", "a", "b", 10.4, Some(PAIR)),
            wall("wb", "b", "a", 12.0, Some(PAIR)),
            wall("w3", "a", "b", 2.0, None),
        ],
        vec![],
    );
    let (area_a, _) = adjacent(&r, "a");
    let (area_b, _) = adjacent(&r, "b");
    assert!((area_a - 12.4).abs() < EPS, "A: 10,4 eigen + 2,0 zonder pair_id: {area_a}");
    assert!((area_b - 14.0).abs() < EPS, "B: 12,0 eigen + 2,0 gespiegeld: {area_b}");
}

/// Afwijkend pair_id-patroon: gebruikt, met waarschuwing.
#[test]
fn v12_afwijkend_pair_id_patroon() {
    let r = import(
        "1.2",
        vec![wall("wa", "a", "b", 10.0, Some("paar-1")), wall("wb", "b", "a", 10.0, Some("paar-1"))],
        vec![],
    );
    assert!(r.warnings.iter().any(|w| w.contains("patroon")), "{:?}", r.warnings);
    let (area_a, _) = adjacent(&r, "a");
    assert!((area_a - 10.0).abs() < EPS, "{area_a}");
}
