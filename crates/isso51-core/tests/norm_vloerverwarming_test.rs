//! Norm-fixtures voor ingebouwde verwarming (fk = 0) en systeemverliezen
//! (ISSO 51:2023 §2.9.1), plus een gebouwniveau-regressiewacht voor
//! ventilatiesysteem D.
//!
//! Provenance: invoer en verwachte uitkomst zijn met de hand afgeleid uit
//! ISSO 51:2023 (editie met erratum 01-09-2023), niet uit de code. Corpus:
//! `%KBA_SHARED%\kennis-v2\isso\`:
//!
//! - fk = 0 voor het verwarmde deel van wand/vloer bij wand-/vloerverwarming:
//!   §2.5.1 bij formule 2.6 (buitenlucht), §2.5.3 fia,k (aangrenzend vertrek),
//!   §2.5.4 bij formule 2.22 (onverwarmde ruimte), §2.5.5 fig,k (grond);
//!   bestand `isso-51__2-1-tm-2-5__2-5.md`.
//! - Onderburen bij vloerverwarming: "vanuit het vertrek gezien geen
//!   warmteverlies (cz=0)", §2.8.1; bestand `isso-51__2-8-2-9-h3-afd-2-8-2-9.md`.
//! - Φverlies1 = (1/n) · fvlw · ΦHL,i, n = 1 bij vertrekberekening
//!   (formule 2.61); verdiepingsvloer binnen de woning: Φverlies1 = 0;
//!   tabel 2.17 (p. 55): Rc > 3,0 → fvlw 0,10 (verdiepingsvloer) resp.
//!   0,15 (begane grondvloer); zelfde bestand, §2.9.1.
//! - Systeemverlies niet in het vertrekvermogen: §4.1 Opmerking (p. 63),
//!   ΦHL,i = Φbasis + Φextra (formule 4.23) en ΦHL,verdeler = Φbasis +
//!   Φextra + ΣΦadd,i (formule 4.24); bestanden `isso-51__h4-4-1-4-2.md`,
//!   `isso-51__h4-4-3-4-5.md`. Gebouw: formule 3.12 / 3.13.
//! - Systeem D op gebouwniveau: Φbasis bevat Φi (formule 3.10), Φvent = Φv
//!   (formule 3.4), Φextra formule 3.11, ΦHL,build formule 3.12;
//!   bestand `isso-51__2-8-2-9-h3-schilmethode.md`.
//!
//! Aanleiding: controle van een warmteverliesrapport (projectnr. 3076,
//! 02-10-2026), bevindingen 1, 2 en de gebouwsom bij systeem D.

use isso51_core::calculate;
use isso51_core::model::building::Project;
use serde_json::{json, Value};

const EPS: f64 = 1e-6;

/// Een vertrek van 20 m² op θi = 20 °C (θe = −10 °C) met:
/// - één wand naar een onverwarmde ruimte, A = 10 m², U = 0,4, fk = 0,5
///   (expliciet) → H = 2,0 W/K → Φ_T = 2,0 · 30 = 60 W;
/// - één vloer met vloerverwarming (`floor`), variant per test.
///
/// Geen buitenlucht-oppervlak → q_i = 0 (tabel 4.3 is per m² buitenschil),
/// ventilatie q_v = 0, geen nachtverlaging → Φhu = 0, cz = 0 (klasse A).
/// Daarmee is ΦHL,i = 60 W voor elke variant waarin de verwarmde vloer
/// volgens de norm niets bijdraagt.
fn project_met_vloer(floor: Value, security_class: &str) -> Project {
    let p = json!({
        "info": { "name": "norm-vloerverwarming" },
        "building": {
            "building_type": "detached",
            "qv10": 0.0,
            "total_floor_area": 20.0,
            "security_class": security_class,
            "has_night_setback": false,
            "warmup_time": 2.0,
            "num_floors": 1,
            "infiltration_method": "per_exterior_area",
            "aggregation_method": "norm_strict",
            "built_after_2015": true
        },
        "climate": {
            "theta_e": -10.0,
            "theta_b_residential": 17.0,
            "theta_b_non_residential": 14.0,
            "wind_factor": 1.0,
            "theta_water": 5.0,
            "theta_ground": 10.0
        },
        "ventilation": { "system_type": "system_c", "has_heat_recovery": false, "has_preheating": false },
        "rooms": [{
            "id": "r1",
            "name": "Vertrek",
            "function": "living_room",
            "floor_area": 20.0,
            "height": 2.6,
            "custom_temperature": 20.0,
            "heating_system": "floor_heating_main_low",
            "ventilation_rate": 0.0,
            "has_mechanical_exhaust": false,
            "has_mechanical_supply": false,
            "fraction_outside_air": 1.0,
            "clamp_positive": true,
            "constructions": [
                {
                    "id": "wand",
                    "description": "Wand naar onverwarmde ruimte",
                    "area": 10.0,
                    "u_value": 0.4,
                    "boundary_type": "unheated_space",
                    "material_type": "masonry",
                    "temperature_factor": 0.5,
                    "vertical_position": "wall",
                    "use_forfaitaire_thermal_bridge": false,
                    "has_embedded_heating": false
                },
                floor
            ]
        }]
    });
    serde_json::from_value(p).expect("fixture moet deserialiseren")
}

/// Vloer met vloerverwarming, A = 20 m², U = 0,25 → Rc ≈ 1/0,25 − Rsi − Rse
/// ≈ 3,6 à 3,8 m²K/W > 3,0 (rij "Rc > 3,0" van tabel 2.17).
fn verwarmde_vloer(boundary: &str, extra: Value) -> Value {
    let mut v = json!({
        "id": "vloer",
        "description": "Vloer met vloerverwarming",
        "area": 20.0,
        "u_value": 0.25,
        "boundary_type": boundary,
        "material_type": "masonry",
        "vertical_position": "floor",
        "use_forfaitaire_thermal_bridge": false,
        "has_embedded_heating": true
    });
    if let (Some(obj), Some(add)) = (v.as_object_mut(), extra.as_object()) {
        for (k, val) in add {
            obj.insert(k.clone(), val.clone());
        }
    }
    v
}

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < EPS,
        "{what}: verwacht {expected}, kreeg {actual}"
    );
}

/// Gemeenschappelijke verwachting: de verwarmde vloer telt niet in Φ_T
/// (fk = 0), ΦHL,i = 60 W, Φverlies1 = fvlw · 60 W buiten het vertrek-
/// vermogen, en op gebouwniveau 3.13 = 3.12 + ΣΦadd,i.
fn check(project: &Project, fvlw: f64) {
    let res = calculate(project).expect("berekening");
    let r = &res.rooms[0];
    let theta_diff = 30.0;

    // fk = 0: alleen de wand (H = 2,0 W/K) draagt bij aan de transmissie.
    let h_t_total = r.transmission.h_t_exterior
        + r.transmission.h_t_adjacent_rooms
        + r.transmission.h_t_unheated
        + r.transmission.h_t_adjacent_buildings
        + r.transmission.h_t_ground
        + r.transmission.h_t_water;
    assert_close(h_t_total, 2.0, "H_T vertrek (verwarmde vloer fk = 0)");
    assert_close(r.transmission.phi_t, 2.0 * theta_diff, "Φ_T vertrek");

    // Formule 4.23: ΦHL,i = Φbasis + Φextra, zonder systeemverlies (§4.1).
    assert_close(r.total_heat_loss, 60.0, "ΦHL,i (formule 4.23)");
    assert_close(r.basis_heat_loss, 60.0, "Φbasis (formule 4.21)");

    // Formule 2.61, n = 1: Φverlies1 = fvlw · ΦHL,i.
    let phi_verlies1 = fvlw * 60.0;
    assert_close(r.system_losses.phi_floor_loss, phi_verlies1, "Φverlies1 (formule 2.61)");
    assert_close(r.system_losses.phi_system_total, phi_verlies1, "ΣΦadd,i vertrek");

    // Gebouw (aggregatie norm_strict): 3.12 zonder, 3.13 met systeemverlies.
    let s = &res.summary;
    assert_close(s.phi_hl_build, 60.0, "ΦHL,build (formule 3.12)");
    assert_close(s.phi_hl_verdeler, 60.0 + phi_verlies1, "ΦHL,verdeler (formule 3.13)");
}

/// Begane grondvloer op de grond: fig,k = 0 (§2.5.5) en tabel 2.17 kolom
/// "begane grondvloer", Rc > 3,0 → fvlw = 0,15 → Φverlies1 = 9,0 W.
#[test]
fn norm_vloerverwarming_op_grond() {
    let floor = verwarmde_vloer(
        "ground",
        json!({ "ground_params": { "u_equivalent": 0.2, "ground_water_factor": 1.0, "fg2": 0.5 } }),
    );
    check(&project_met_vloer(floor, "a"), 0.15);
}

/// Vloer boven een kruipruimte (onverwarmde ruimte): fk = 0 (§2.5.4 bij
/// formule 2.22) en §2.9.1 "naar de bodem/kruipruimte" → kolom begane
/// grondvloer, fvlw = 0,15 → Φverlies1 = 9,0 W.
#[test]
fn norm_vloerverwarming_boven_kruipruimte() {
    let floor = verwarmde_vloer("unheated_space", json!({ "temperature_factor": 0.6 }));
    check(&project_met_vloer(floor, "a"), 0.15);
}

/// Vloer boven onderburen (gestapelde bouw), cz = 1 (klasse C): vanuit het
/// vertrek geen verlies (§2.8.1, fk = 0); systeemverlies naar beneden met
/// kolom "verdiepingsvloer", fvlw = 0,10 → Φverlies1 = 6,0 W.
#[test]
fn norm_vloerverwarming_boven_onderburen() {
    let floor = verwarmde_vloer("adjacent_building", json!({}));
    check(&project_met_vloer(floor, "c"), 0.10);
}

/// Verdiepingsvloer binnen de woning boven een kouder vertrek (15 °C):
/// fia,k = 0 (§2.5.3) en "voor verdiepingsvloeren binnen de woning
/// Φverlies1 = 0" (§2.9.1).
#[test]
fn norm_vloerverwarming_verdiepingsvloer_binnen_woning() {
    let floor = verwarmde_vloer("adjacent_room", json!({ "adjacent_temperature": 15.0 }));
    check(&project_met_vloer(floor, "a"), 0.0);
}

/// Gebouwniveau-regressiewacht systeem D (mechanische toevoer):
/// Φbasis bevat ΣΦi volledig (formule 3.10) en Φvent = ΣΦv zonder aftrek
/// (formule 3.4). Zonder Φhu en ΦT,iaBE is Φextra = Φvent (formule 3.11),
/// dus ΦHL,build = ΣΦT,ie + ΣΦi + ΣΦv (formule 3.12).
#[test]
fn norm_systeem_d_infiltratie_in_gebouwtotaal() {
    let p = json!({
        "info": { "name": "norm-systeem-d" },
        "building": {
            "building_type": "detached",
            "qv10": 0.0,
            "total_floor_area": 20.0,
            "security_class": "a",
            "has_night_setback": false,
            "warmup_time": 2.0,
            "num_floors": 1,
            "infiltration_method": "per_exterior_area",
            "aggregation_method": "norm_strict",
            "built_after_2015": true
        },
        "climate": {
            "theta_e": -10.0,
            "theta_b_residential": 17.0,
            "theta_b_non_residential": 14.0,
            "wind_factor": 1.0,
            "theta_water": 5.0,
            "theta_ground": 10.0
        },
        "ventilation": { "system_type": "system_d", "has_heat_recovery": false, "has_preheating": false },
        "rooms": [{
            "id": "r1",
            "name": "Vertrek",
            "function": "living_room",
            "floor_area": 20.0,
            "height": 2.6,
            "custom_temperature": 20.0,
            "heating_system": "radiator_lt",
            "ventilation_rate": 10.0,
            "supply_air_temperature": -10.0,
            "has_mechanical_exhaust": true,
            "has_mechanical_supply": true,
            "fraction_outside_air": 1.0,
            "clamp_positive": true,
            "constructions": [{
                "id": "gevel",
                "description": "Buitenwand",
                "area": 10.0,
                "u_value": 0.5,
                "boundary_type": "exterior",
                "material_type": "masonry",
                "vertical_position": "wall",
                "use_forfaitaire_thermal_bridge": false,
                "custom_delta_u_tb": 0.0,
                "has_embedded_heating": false
            }]
        }]
    });
    let project: Project = serde_json::from_value(p).expect("fixture moet deserialiseren");
    let res = calculate(&project).expect("berekening");
    let r = &res.rooms[0];
    let s = &res.summary;

    // Tabel 4.3 (qv10 ≤ 50): q_i = 0,08 · 10 m² = 0,8 dm³/s → Φi = 0,96 · 30 = 28,8 W.
    assert_close(r.infiltration.phi_i, 28.8, "Φi (tabel 4.3, H_i = 1,2 · q_i)");
    // Formule 2.6: Φ_T,ie = 10 · 0,5 · 30 = 150 W.
    assert_close(r.transmission.h_t_exterior * 30.0, 150.0, "Φ_T,ie");
    assert!(r.ventilation.phi_v > 0.0, "Φv moet > 0 zijn bij q_v = 10 dm³/s");

    // Formule 3.4: systeem D → Φvent = ΣΦv (geen aftrek van Φi).
    assert_close(s.phi_vent_building, r.ventilation.phi_v, "Φvent gebouw (formule 3.4)");
    // Formule 3.10: Φbasis bevat Φi.
    assert_close(s.phi_basis_build, 150.0 + 28.8, "Φbasis gebouw (formule 3.10)");
    // Formule 3.11 + 3.12.
    assert_close(
        s.phi_hl_build,
        150.0 + 28.8 + r.ventilation.phi_v,
        "ΦHL,build (formule 3.12)",
    );
}
