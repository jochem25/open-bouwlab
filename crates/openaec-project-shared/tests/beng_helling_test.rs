//! Helling bij BENG (12-10-2026): een dak met een kleine helling komt binnen
//! als `buitenlucht{orientatie: zuid}` met `helling_deg` (bv. 2°). De keten moet
//! die helling gebruiken voor `I_sol` (NTA 8800 tabel 17.2, lineair over de
//! hellingshoek) en voor de beschaduwing (§17.3.1: zichtveld Zuid bij een helling
//! onder 15°). Een dak van 2° mag niet rekenen als een verticale zuidgevel.
//!
//! Verwachtingswaarden met de hand uit het normcorpus (`kennis-v2/nen`):
//! `nta-8800__h17-klimaat-deel1.md` (tabel 17.2) en `-deel2.md` (tabel 17.13).

use nta8800_demand::calc::shading::{obstruction_g_factor, SolarBalance};
use nta8800_demand::calc::solar_gains::monthly_solar_gains;
use nta8800_model::time::Month;
use nta8800_model::Window;
use openaec_project_shared::beng::geometry_bridge::beng_geometry_to_shared;
use openaec_project_shared::beng_geometry::{
    BengAdjacency, BengBoundary, BengGeometry, BengWindowPlacement, BengZone, KozijnType,
    OpaqueConstructionDef, RcOrU, VlakType, WindowDef,
};
use openaec_project_shared::{geometry_to_nta8800, Obstruction, Orientation, SharedGeometry, SharedProject};

/// Rekenzone met een dak naar het zuiden met de gegeven helling en één raam
/// (2 m², g 0,6, F_fr 0,2, U 1,1) met de gegeven belemmering.
fn geo_dak(helling_deg: f64, belemmering: Obstruction) -> BengGeometry {
    BengGeometry {
        bron: None,
        opaque_defs: vec![OpaqueConstructionDef {
            id: "dak".into(),
            omschrijving: String::new(),
            kind: VlakType::Dak,
            thermal: RcOrU::Rc(6.3),
        }],
        window_defs: vec![WindowDef {
            id: "dakraam".into(),
            omschrijving: String::new(),
            kind: KozijnType::Raam,
            u_w_per_m2k: 1.1,
            ggl: Some(0.6),
            area_m2: 2.0,
            frame_fraction: Some(0.2),
        }],
        zones: vec![BengZone {
            id: "rz".into(),
            naam: String::new(),
            a_g_m2: 60.0,
            bouwwijze_vloer: None,
            bouwwijze_wand: None,
            woningtype: None,
            gebruiksfuncties: Vec::new(),
            gevels: vec![BengBoundary {
                id: "dak-z".into(),
                omschrijving: String::new(),
                vlak_type: VlakType::Dak,
                grenst_aan: BengAdjacency::Buitenlucht { orientatie: Orientation::Zuid },
                bruto_buiten_opp_m2: 60.0,
                helling_deg: Some(helling_deg),
                omtrek_p_m: None,
                constructie_ref: "dak".into(),
                ramen: vec![BengWindowPlacement {
                    kozijn_ref: "dakraam".into(),
                    aantal: 1,
                    belemmering,
                    zonwering: None,
                    zomernachtventilatie: false,
                }],
            }],
        }],
    }
}

/// Het raam zoals de BENG-rekenkern het ziet (brug + NTA 8800-view).
fn raam_in_kern(helling_deg: f64, belemmering: Obstruction) -> Window {
    let geo = geo_dak(helling_deg, belemmering);
    geo.validate().expect("geldige BENG-geometrie");
    let shared = beng_geometry_to_shared(&geo, &SharedGeometry::default()).unwrap();
    let view = geometry_to_nta8800(&SharedProject::new("helling"), &shared).unwrap();
    assert_eq!(view.windows.len(), 1);
    view.windows[0].clone()
}

#[test]
fn helling_reist_door_naar_het_raam() {
    let w = raam_in_kern(2.0, Obstruction::None);
    assert_eq!(w.orientation, Orientation::Zuid);
    assert!((w.tilt.degrees - 2.0).abs() < 1e-12, "tilt {}", w.tilt.degrees);
}

/// I_sol (tabel 17.2, ρ = 0,2), juni: β=0° = 209,3 W/m²; β=30° Z = 211,2 W/m²
/// (deel1, regel "Juni" van het deel β = 0° en β = 30°); β=90° Z = 112,7 W/m².
/// Lineair over β: I(2°) = 209,3 + (2/30)·(211,2 − 209,3) = 209,42667 W/m².
///
/// Q_sol (7.32), juni (t = 720 h), warmtebalans, geen belemmering:
///   g-werkend = A·g·F_w·(1 − F_F) = 2·0,6·0,9·0,8 = 0,864 m²
///   bruto     = 0,864 · 209,42667 · 720 · 0,0036 = 469,0085 MJ
///   Q_sky     = F_sky·R_se·U·A·h_lr·Δθ·t·0,0036
///             = 1,0·0,04·1,1·2·4,14·11·720·0,0036   = 10,3875 MJ (2° ≤ 5° → F_sky 1,0)
/// Ter vergelijking verticaal (F_sky 0,5): 0,864·112,7·720·0,0036 − 5,1937 = 247,197 MJ.
/// Netto dak 2°: 469,0085 − 10,3875 = 458,621 MJ.
#[test]
fn dak_2_graden_zuid_neemt_tabel_17_2_over_de_helling() {
    let climate = nta8800_tables::climate::de_bilt_climate_data();
    let q_sky = |f_sky: f64| f_sky * 0.04 * 1.1 * 2.0 * 4.14 * 11.0 * 720.0 * 0.0036;

    let dak = raam_in_kern(2.0, Obstruction::None);
    let q_dak = monthly_solar_gains(&[&dak], &climate, 1.0, SolarBalance::Heating)[Month::Juni];
    let i_2 = 209.3 + (2.0 / 30.0) * (211.2 - 209.3);
    let verwacht_dak = 0.864 * i_2 * 720.0 * 0.0036 - q_sky(1.0);
    assert!((q_dak - verwacht_dak).abs() < 1e-9, "dak 2°: {q_dak} != {verwacht_dak}");
    assert!((verwacht_dak - 458.621).abs() < 1e-3, "handgetal {verwacht_dak}");

    let gevel = raam_in_kern(90.0, Obstruction::None);
    let q_gevel = monthly_solar_gains(&[&gevel], &climate, 1.0, SolarBalance::Heating)[Month::Juni];
    let verwacht_gevel = 0.864 * 112.7 * 720.0 * 0.0036 - q_sky(0.5);
    assert!((q_gevel - verwacht_gevel).abs() < 1e-6, "gevel: {q_gevel} != {verwacht_gevel}");

    // Het dak van 2° rekent dus niet als verticale zuidgevel.
    assert!((q_dak - q_gevel).abs() > 100.0, "dak {q_dak} vs gevel {q_gevel}");
}

/// Beschaduwing (§17.3.1): bij een helling onder 15° geldt het zichtveld Zuid en
/// (dichtstbijzijnde kolom, §17.3.7) de kolom 0°. Tabel 17.13 Zuid, 0°:
/// januari 0,55, juni 0,37 (deel2:1246, 1251). Verticaal Zuid januari = 0,19.
#[test]
fn dak_2_graden_beschaduwing_volgt_zichtveld_zuid_kolom_0() {
    let dak = raam_in_kern(2.0, Obstruction::Full);
    let f = obstruction_g_factor(dak.obstruction, dak.orientation, dak.tilt, SolarBalance::Heating);
    assert!((f[Month::Januari] - 0.55).abs() < 1e-12, "jan {}", f[Month::Januari]);
    assert!((f[Month::Juni] - 0.37).abs() < 1e-12, "jun {}", f[Month::Juni]);

    // Zelfde dak op het noorden: zichtveld blijft Zuid.
    let noord = obstruction_g_factor(Obstruction::Full, Orientation::Noord, dak.tilt, SolarBalance::Heating);
    assert_eq!(noord[Month::Januari], f[Month::Januari]);

    let gevel = raam_in_kern(90.0, Obstruction::Full);
    let fg = obstruction_g_factor(gevel.obstruction, gevel.orientation, gevel.tilt, SolarBalance::Heating);
    assert!((fg[Month::Januari] - 0.19).abs() < 1e-12, "gevel jan {}", fg[Month::Januari]);
}
