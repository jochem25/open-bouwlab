//! Spiegelregel thermal-import: een scheiding tussen twee echte ruimten wordt
//! één keer geleverd, beschreven vanuit `room_a`; de ontvanger maakt zelf de
//! `room_b`-kant met gespiegelde orientatie (vloer <-> plafond). Een paar dat
//! van beide kanten binnenkomt geeft een waarschuwing.
//!
//! Synthetische fixtures: twee verwarmde ruimten boven elkaar, één
//! tussenvloer van 10 m². Spec: `docs/thermal-import-construction-catalog-spec.md`
//! (sectie Spiegelregel).

use isso51_core::import::thermal::ThermalOrientation;
use isso51_core::import::{map_thermal_import, ThermalImport};
use isso51_core::model::enums::{BoundaryType, VerticalPosition};
use serde_json::{json, Value};

fn layers() -> Value {
    json!([
        { "material": "Beton", "thickness_mm": 200.0, "distance_from_interior_mm": 0.0, "type": "solid" },
        { "material": "Isolatie", "thickness_mm": 100.0, "distance_from_interior_mm": 200.0, "type": "solid" }
    ])
}

fn construction(id: &str, a: &str, b: &str, orientation: &str, area: f64) -> Value {
    json!({ "id": id, "room_a": a, "room_b": b, "orientation": orientation,
            "gross_area_m2": area, "layers": layers() })
}

fn import(constructions: Vec<Value>) -> isso51_core::import::ThermalImportResult {
    import_rooms(constructions, false)
}

/// `onder_eerst` zet de onderruimte vóór de bovenruimte in `rooms[]`.
fn import_rooms(constructions: Vec<Value>, onder_eerst: bool) -> isso51_core::import::ThermalImportResult {
    let boven = json!({ "id": "boven", "name": "Boven", "type": "heated", "area_m2": 10.0, "height_m": 2.6 });
    let onder = json!({ "id": "onder", "name": "Onder", "type": "heated", "area_m2": 10.0, "height_m": 2.6 });
    let buiten = json!({ "id": "room-ground", "name": "Grond", "type": "ground" });
    let rooms = if onder_eerst { vec![onder, boven, buiten] } else { vec![boven, onder, buiten] };
    let t = json!({
        "version": "1.1", "source": "ifc", "exported_at": "2026-10-05T00:00:00Z",
        "rooms": rooms,
        "constructions": constructions
    });
    let ti: ThermalImport = serde_json::from_value(t).expect("fixture moet deserialiseren");
    map_thermal_import(ti)
}

fn position_in(result: &isso51_core::import::ThermalImportResult, room: &str) -> Vec<VerticalPosition> {
    result
        .project
        .rooms
        .iter()
        .find(|r| r.id == room)
        .expect("ruimte aanwezig")
        .constructions
        .iter()
        .filter(|c| c.boundary_type == BoundaryType::AdjacentRoom)
        .map(|c| c.vertical_position)
        .collect()
}

fn double_delivery_warnings(result: &isso51_core::import::ThermalImportResult) -> usize {
    result.warnings.iter().filter(|w| w.contains("van beide kanten")).count()
}

/// De vloer van de bovenruimte is het plafond van de onderruimte.
#[test]
fn tussenvloer_een_kant_geeft_plafond_aan_zijde_b() {
    let r = import(vec![construction("f1", "boven", "onder", "floor", 10.0)]);
    assert_eq!(position_in(&r, "boven"), vec![VerticalPosition::Floor]);
    assert_eq!(position_in(&r, "onder"), vec![VerticalPosition::Ceiling]);
    assert_eq!(double_delivery_warnings(&r), 0);
}

/// Omgekeerd beschreven (plafond van de onderruimte) levert hetzelfde beeld.
#[test]
fn plafond_een_kant_geeft_vloer_aan_zijde_b() {
    let r = import(vec![construction("c1", "onder", "boven", "ceiling", 10.0)]);
    assert_eq!(position_in(&r, "onder"), vec![VerticalPosition::Ceiling]);
    assert_eq!(position_in(&r, "boven"), vec![VerticalPosition::Floor]);
}

/// Een wand blijft een wand aan beide kanten.
#[test]
fn wand_blijft_wand() {
    let r = import(vec![construction("w1", "boven", "onder", "wall", 10.0)]);
    assert_eq!(position_in(&r, "boven"), vec![VerticalPosition::Wall]);
    assert_eq!(position_in(&r, "onder"), vec![VerticalPosition::Wall]);
}

/// Beide kanten geleverd (gespiegelde orientatie, oppervlak binnen 5 %) →
/// waarschuwing; de import ontdubbelt niet stil.
#[test]
fn dubbel_geleverd_paar_geeft_waarschuwing() {
    let r = import(vec![
        construction("f1", "boven", "onder", "floor", 10.0),
        construction("c1", "onder", "boven", "ceiling", 9.85),
    ]);
    assert_eq!(double_delivery_warnings(&r), 1, "warnings: {:?}", r.warnings);
}

/// Twee segmenten tussen hetzelfde paar vanaf dezelfde kant zijn legitiem:
/// geen waarschuwing.
#[test]
fn twee_segmenten_zelfde_kant_geen_waarschuwing() {
    let r = import(vec![
        construction("w1", "boven", "onder", "wall", 5.0),
        construction("w2", "boven", "onder", "wall", 5.0),
    ]);
    assert_eq!(double_delivery_warnings(&r), 0, "warnings: {:?}", r.warnings);
}

/// Verschillend oppervlak (> 5 %) is geen dubbele levering.
#[test]
fn ander_oppervlak_geen_waarschuwing() {
    let r = import(vec![
        construction("w1", "boven", "onder", "wall", 10.0),
        construction("w2", "onder", "boven", "wall", 4.0),
    ]);
    assert_eq!(double_delivery_warnings(&r), 0, "warnings: {:?}", r.warnings);
}

/// De waarschuwing is één leesbare zin (geen spatie-gaten uit een
/// string-afbreking).
#[test]
fn waarschuwingstekst_zonder_dubbele_spaties() {
    let r = import(vec![
        construction("f1", "boven", "onder", "floor", 10.0),
        construction("c1", "onder", "boven", "ceiling", 10.0),
    ]);
    let w = r.warnings.iter().find(|w| w.contains("van beide kanten")).expect("waarschuwing");
    assert!(!w.contains("  "), "dubbele spatie in: {w}");
}

/// `roof` spiegelt naar `floor`; de melding mag niet van de volgorde in het
/// bestand afhangen.
#[test]
fn roof_floor_paar_in_beide_volgordes_gemeld() {
    let a = import(vec![
        construction("r1", "onder", "boven", "roof", 10.0),
        construction("f1", "boven", "onder", "floor", 10.0),
    ]);
    let b = import(vec![
        construction("f1", "boven", "onder", "floor", 10.0),
        construction("r1", "onder", "boven", "roof", 10.0),
    ]);
    assert_eq!(double_delivery_warnings(&a), 1, "{:?}", a.warnings);
    assert_eq!(double_delivery_warnings(&b), 1, "{:?}", b.warnings);
}

/// De catalogus noemt de geleverde (room_a) orientatie eerst, ongeacht de
/// volgorde van de ruimten; de frontend gebruikt `used_for[0]` voor de
/// Rc-positie.
#[test]
fn catalogus_used_for_volgt_geleverde_kant() {
    for onder_eerst in [false, true] {
        let r = import_rooms(vec![construction("f1", "boven", "onder", "floor", 10.0)], onder_eerst);
        assert_eq!(r.construction_catalog.len(), 1);
        let used_for = &r.construction_catalog[0].used_for;
        assert_eq!(used_for[0], (BoundaryType::AdjacentRoom, ThermalOrientation::Floor), "onder_eerst={onder_eerst}: {used_for:?}");
        assert!(used_for.contains(&(BoundaryType::AdjacentRoom, ThermalOrientation::Ceiling)));
    }
}

/// Pseudo-room als room_a valt buiten het contract: dan geen spiegeling
/// (gedrag van voor 05-10).
#[test]
fn pseudo_room_a_niet_gespiegeld() {
    let r = import(vec![construction("g1", "room-ground", "onder", "floor", 10.0)]);
    let onder = r.project.rooms.iter().find(|x| x.id == "onder").expect("onder");
    assert_eq!(onder.constructions.len(), 1);
    assert_eq!(onder.constructions[0].vertical_position, VerticalPosition::Floor);
}
