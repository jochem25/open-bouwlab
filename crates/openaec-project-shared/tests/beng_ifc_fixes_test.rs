//! BENG-keten-fixes van 12-10-2026 (opdracht beng-ifc), met de hand nagerekend.
//!
//! - Fix 1: verliesoppervlakte `A_ls` gewogen met `f_ls` (NTA 8800 §6.7.2
//!   formule 6.3, §6.7.3).
//! - Fix 2: kozijnfractie `F_fr` per kozijnmerk reist door de brug naar de
//!   zontoetreding (NTA 8800 §7.6.6.2 methode A, formule 7.44; forfait 0,25 =
//!   methode B).
//!
//! Alle invoer is synthetisch, behalve de Aalten-fixture die al in de repo staat.

use std::collections::BTreeMap;

use nta8800_demand::calc::shading::SolarBalance;
use nta8800_demand::calc::solar_gains::monthly_solar_gains;
use nta8800_model::location::Tilt;
use nta8800_model::time::MonthlyProfile;
use nta8800_model::{ClimateData, Window};
use openaec_project_shared::beng::geometry_bridge::beng_geometry_to_shared;
use openaec_project_shared::beng_geometry::{
    BengAdjacency, BengBoundary, BengGeometry, BengWindowPlacement, BengZone, KozijnType,
    OpaqueConstructionDef, RcOrU, VlakType, WindowDef,
};
use openaec_project_shared::{Obstruction, Orientation, SharedGeometry};

fn def(id: &str, kind: VlakType) -> OpaqueConstructionDef {
    OpaqueConstructionDef {
        id: id.into(),
        omschrijving: String::new(),
        kind,
        thermal: RcOrU::Rc(4.7),
    }
}

fn vlak(
    id: &str,
    vlak_type: VlakType,
    grenst_aan: BengAdjacency,
    opp: f64,
    def_ref: &str,
) -> BengBoundary {
    let omtrek_p_m = grenst_aan.requires_omtrek().then_some(30.0);
    BengBoundary {
        id: id.into(),
        omschrijving: String::new(),
        vlak_type,
        grenst_aan,
        bruto_buiten_opp_m2: opp,
        helling_deg: None,
        omtrek_p_m,
        constructie_ref: def_ref.into(),
        ramen: Vec::new(),
    }
}

/// Eén vlak per weegfactor-categorie van §6.7.3.
fn geo_alle_begrenzingen() -> BengGeometry {
    let buiten = BengAdjacency::Buitenlucht {
        orientatie: Orientation::Zuid,
    };
    BengGeometry {
        opaque_defs: vec![
            def("v", VlakType::Vloer),
            def("g", VlakType::Gevel),
            def("k", VlakType::Kelderwand),
        ],
        window_defs: Vec::new(),
        zones: vec![BengZone {
            id: "rz".into(),
            naam: String::new(),
            a_g_m2: 100.0,
            bouwwijze_vloer: None,
            bouwwijze_wand: None,
            woningtype: None,
            gebruiksfuncties: Vec::new(),
            gevels: vec![
                vlak("gevel", VlakType::Gevel, buiten, 100.0, "g"),
                vlak(
                    "vloer-grond",
                    VlakType::Vloer,
                    BengAdjacency::VloerOpMaaiveldBovenGrond,
                    50.0,
                    "v",
                ),
                vlak(
                    "vloer-kruip",
                    VlakType::Vloer,
                    BengAdjacency::VloerOpMaaiveldBovenKruipruimte,
                    40.0,
                    "v",
                ),
                vlak(
                    "vloer-kelder",
                    VlakType::Vloer,
                    BengAdjacency::VloerOpMaaiveldBovenOnverwarmdeKelder,
                    30.0,
                    "v",
                ),
                vlak("kelderwand", VlakType::Kelderwand, buiten, 10.0, "k"),
                vlak(
                    "avr",
                    VlakType::Gevel,
                    BengAdjacency::AangrenzendeVerwarmdeRuimte,
                    20.0,
                    "g",
                ),
                vlak(
                    "aor",
                    VlakType::Gevel,
                    BengAdjacency::AorForfaitair,
                    5.0,
                    "g",
                ),
                vlak("water", VlakType::Gevel, BengAdjacency::Water, 2.0, "g"),
            ],
        }],
    }
}

/// NTA 8800 §6.7.3: a) AVR 0; b) buitenlucht, water, AOR, onverwarmde kelder
/// (een AOR) 1; c) grond en kruipruimte 0,7; een kelderwand grenst aan de grond.
///
/// A_ls = 1·100 + 0,7·50 + 0,7·40 + 1·30 + 0,7·10 + 0·20 + 1·5 + 1·2
///      = 100 + 35 + 28 + 30 + 7 + 0 + 5 + 2 = 207 m².
#[test]
fn a_ls_weegt_met_f_ls_per_begrenzing() {
    let geo = geo_alle_begrenzingen();
    geo.validate().unwrap();
    assert!(
        (geo.loss_surface_area_m2() - 207.0).abs() < 1e-9,
        "{}",
        geo.loss_surface_area_m2()
    );
}

/// Aalten-fixture (bestaand BENG-anker): grondvloer 67,0 m² weegt 0,7, de
/// overige vlakken 1. A_ls = 0,7·67,0 + (21,96 + 23,81 + 39,86 + 23,81) + 69,3
/// = 46,90 + 109,44 + 69,30 = 225,64 m² (was ongewogen 245,74).
#[test]
fn a_ls_aalten_fixture_gewogen() {
    let raw = std::fs::read_to_string(
        "../../tests/verification/beng_uniec_crosscheck/aalten-2522/beng_geometry.input.json",
    )
    .unwrap();
    let geo: BengGeometry = serde_json::from_str(&raw).unwrap();
    assert!(
        (geo.loss_surface_area_m2() - 225.64).abs() < 1e-9,
        "{}",
        geo.loss_surface_area_m2()
    );
}

fn geo_met_raam(frame_fraction: Option<f64>) -> BengGeometry {
    let mut geo = geo_alle_begrenzingen();
    geo.window_defs.push(WindowDef {
        id: "merk".into(),
        omschrijving: String::new(),
        kind: KozijnType::Raam,
        u_w_per_m2k: 1.2,
        ggl: Some(0.4),
        area_m2: 2.0,
        frame_fraction,
    });
    geo.zones[0].gevels[0].ramen.push(BengWindowPlacement {
        kozijn_ref: "merk".into(),
        aantal: 1,
        belemmering: Obstruction::None,
        zonwering: None,
        zomernachtventilatie: false,
    });
    geo
}

/// De kozijnfractie van het kozijnmerk komt 1-op-1 op de opening; zonder
/// waarde blijft `None` (de demand-keten valt dan terug op 0,25).
#[test]
fn frame_fraction_reist_door_de_brug() {
    for ff in [Some(0.12), None] {
        let shared =
            beng_geometry_to_shared(&geo_met_raam(ff), &SharedGeometry::default()).unwrap();
        let o = &shared.spaces[0].constructions[0].openings[0];
        assert_eq!(o.frame_fraction, ff);
    }
}

#[test]
fn frame_fraction_buiten_bereik_wordt_geweigerd() {
    assert!(geo_met_raam(Some(1.2)).validate().is_err());
    assert!(geo_met_raam(Some(-0.1)).validate().is_err());
    assert!(geo_met_raam(Some(1.0)).validate().is_ok());
}

/// Formule 7.32 bevat de factor (1 − F_fr) op de bruto zontoetreding; de
/// hemelstraling Q_sky hangt niet van F_fr af. Met een raam van F_fr = 1,0 als
/// nulpunt (alleen −Q_sky) geldt per maand:
///
/// (Q(0,10) − Q(1,0)) / (Q(0,25) − Q(1,0)) = (1 − 0,10) / (1 − 0,25) = 0,90 / 0,75 = 1,20.
#[test]
fn zontoetreding_schaalt_met_1_min_f_fr() {
    let mut solar = BTreeMap::new();
    solar.insert(Orientation::Zuid, MonthlyProfile::from_constant(100.0));
    let climate = ClimateData {
        outdoor_temperature: MonthlyProfile::from_constant(10.0),
        solar_irradiation: solar,
        cooling_reference_temperature: MonthlyProfile::from_constant(Some(16.0)),
        wind_speed: MonthlyProfile::from_constant(3.0),
        wtw_preheat_temperature: MonthlyProfile::from_constant(0.0),
    };
    let raam = |ff: f64| {
        Window::new(
            "w",
            "c",
            2.0,
            Orientation::Zuid,
            Tilt::VERTICAL,
            1.2,
            0.4,
            ff,
        )
        .unwrap()
    };
    let q = |ff: f64| monthly_solar_gains(&[&raam(ff)], &climate, 1.0, SolarBalance::Heating);
    let (q10, q25, q100) = (q(0.10), q(0.25), q(1.0));
    for m in nta8800_model::time::Month::all() {
        let ratio = (q10[m] - q100[m]) / (q25[m] - q100[m]);
        assert!((ratio - 1.2).abs() < 1e-9, "{m:?}: {ratio}");
    }
}
