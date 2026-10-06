//! thermal.json v1.2 uit een echte bronlevering: synthetische fixture van
//! ifc-ruimtebalans (`fixture_v12_ongelijke_hoogte`, bron d7378e2), byte-
//! identiek overgenomen in `tests/fixtures/thermal-import-v12-ongelijke-hoogte.json`
//! (sha256 82eadb10838e6dc7...).
//!
//! Geometrie: ruimte "hoog" 4,0 × 3,0 × 2,60 m en "laag" 4,0 × 3,0 × 2,40 m
//! naast elkaar, verspringend dak, binnenwand 0,20 m met een deur 0,80 × 2,10 m.
//!
//! Verwachting met de hand:
//! - laag → hoog: 3,0 · 2,40 = 7,20 m² (één deel).
//! - hoog → laag: 3,0 · 2,34 = 7,02 m² (drie delen 3,90 + 1,56 + 1,56, zelfde
//!   pair_id; de bron meet in rijen van 0,26 m, dus 2,34 i.p.v. 2,40). De strook
//!   boven het dak van laag (0,78 m²) gaat naar buiten, niet naar laag.
//! - deur 0,80 · 2,10 = 1,68 m² per kant, één per ruimte.
//! - paarverschil |7,20 − 7,02| / 7,20 = 2,5 % < 5 %: geen QC-melding.
//! - U = 1, θ_hoog = 20 °C, θ_laag = 16 °C (ISSO 51:2023 form. 2.17):
//!   H_T,ia,hoog = 7,02 · (20 − 16)/(20 − θe); H_T,ia,laag = 7,20 · (16 − 20)/(16 − θe).
//!   Gespiegeld (v1.1-gedrag) zou elke ruimte 14,22 m² krijgen.

use isso51_core::calculate;
use isso51_core::import::{map_thermal_import, ThermalImport, ThermalImportResult};
use isso51_core::model::enums::BoundaryType;

const HOOG: &str = "room-0000000000000000000008";
const LAAG: &str = "room-000000000000000000000A";
const EPS: f64 = 1e-6;

fn import() -> ThermalImportResult {
    let s = include_str!("../../../tests/fixtures/thermal-import-v12-ongelijke-hoogte.json");
    let ti: ThermalImport = serde_json::from_str(s).expect("fixture moet deserialiseren");
    map_thermal_import(ti)
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
fn fixture_elke_ruimte_eigen_kant() {
    let r = import();
    let (area_hoog, doors_hoog) = adjacent(&r, HOOG);
    let (area_laag, doors_laag) = adjacent(&r, LAAG);
    assert!((area_hoog - 7.02).abs() < EPS, "hoog: {area_hoog}");
    assert!((area_laag - 7.20).abs() < EPS, "laag: {area_laag}");
    assert_eq!(doors_hoog, 1, "hoog: een deur (eigen kant)");
    assert_eq!(doors_laag, 1, "laag: een deur (eigen kant)");
}

#[test]
fn fixture_geen_paar_qc_onder_drempel() {
    let r = import();
    let paar: Vec<_> = r.warnings.iter().filter(|w| w.contains("Paar")).collect();
    assert!(paar.is_empty(), "verwacht 0 paar-meldingen, kreeg {paar:?}");
}

#[test]
fn fixture_transmissie_per_ruimte_uit_eigen_kant() {
    let mut r = import();
    for room in &mut r.project.rooms {
        room.custom_temperature = Some(if room.id == HOOG { 20.0 } else { 16.0 });
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
    let verwacht_hoog = 7.02 * (20.0 - 16.0) / (20.0 - theta_e);
    let verwacht_laag = 7.20 * (16.0 - 20.0) / (16.0 - theta_e);
    assert!((h(HOOG) - verwacht_hoog).abs() < 1e-9, "hoog: {} vs {}", h(HOOG), verwacht_hoog);
    assert!((h(LAAG) - verwacht_laag).abs() < 1e-9, "laag: {} vs {}", h(LAAG), verwacht_laag);
}
