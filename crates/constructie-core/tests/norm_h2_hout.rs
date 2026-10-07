//! Golden H2: houten balklaag met vloerlagen, C24, vrije breedte 75 mm, hoh 400, l = 6,0 m,
//! g_k = 0,60 kN/m2 (eigen gewicht niet automatisch), CC2, klimaatklasse 1, drukrand
//! gesteund. Vloerlagen: vloerplaat 18 mm (E 4000) en plafond 12,5 mm (E 2000),
//! vloerbreedte 4,0 m. De golden komen uit een onafhankelijke controleberekening buiten de kern.
//!
//! Tolerantie: 0,005 op UC; 0,5 % op overige waarden, aangevuld met de halve eenheid
//! van het laatst genoteerde cijfer in de golden.

use constructie_core::model::Kandidaat;
use constructie_core::toets::ToetsStatus;
use constructie_core::{bereken_hout, HoutInvoer};

fn golden(naam: &str, actual: f64, expected: f64, decimalen: i32) {
    let tol = (0.005 * expected.abs()).max(0.5 * 10f64.powi(-decimalen));
    assert!(
        (actual - expected).abs() <= tol,
        "{naam}: berekend {actual}, golden {expected}, tolerantie {tol}"
    );
}

fn uc(naam: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 0.005,
        "{naam}: UC {actual}, golden {expected}"
    );
}

fn invoer(hoogte: Option<f64>, trilling: bool, dwars: Option<f64>) -> HoutInvoer {
    let dwars_deel = match dwars {
        Some(e) => format!(r#""dwarsverbinding":{{"ei_nm2":{e}}},"#),
        None => String::new(),
    };
    let hoogte_deel = match hoogte {
        Some(h) => format!(r#""hoogte_automatisch":false,"hoogte_mm":{h},"#),
        None => String::new(),
    };
    serde_json::from_str(&format!(
        r#"{{"algemeen":{{"overspanning_m":6.0,"permanent_kn_m2":0.60,
            "eigen_gewicht_automatisch":false,"gevolgklasse":"CC2"}},
          "element":{{"type":"balklaag","hoh_mm":400}},
          "sterkteklasse":"C24","breedte_mm":75,{hoogte_deel}{dwars_deel}
          "trillingstoets":{trilling},
          "vloerlagen":[{{"soort":"vloerplaat","dikte_mm":18,"e_mean_n_mm2":4000}},
                        {{"soort":"plafond","dikte_mm":12.5,"e_mean_n_mm2":2000}}],
          "vloerbreedte_m":4.0}}"#
    ))
    .unwrap()
}

fn uc_van(k: &Kandidaat, id: &str) -> f64 {
    k.toetsen
        .iter()
        .find(|t| t.id == id)
        .and_then(|t| t.uc)
        .unwrap_or_else(|| panic!("UC van {id} ontbreekt"))
}

fn waarde_van(k: &Kandidaat, id: &str) -> f64 {
    k.toetsen
        .iter()
        .find(|t| t.id == id)
        .and_then(|t| t.waarde)
        .unwrap_or_else(|| panic!("waarde van {id} ontbreekt"))
}

#[test]
fn h2_vaste_hoogte_275_met_trilling() {
    let r = bereken_hout(&invoer(Some(275.0), true, None)).unwrap();
    let k = &r.kandidaten[0];
    assert_eq!(k.naam, "75 x 275");
    golden("ei_t", k.tussenwaarden["ei_t"], 2269.5, 1);
    golden("f_1", k.tussenwaarden["f_1"], 10.55, 2);
    golden("b_ef", k.tussenwaarden["b_ef"], 0.905, 3);
    golden("w_1kN", k.tussenwaarden["w_1kn"], 1.391, 3);
    uc("UC w_1kN", uc_van(k, "trilling_w1kn"), 1.391);
    golden("n_40", k.tussenwaarden["n_40"], 8.03, 2);
    uc("UC v", uc_van(k, "trilling_v"), 0.907);
    uc("UC buiging", uc_van(k, "buiging"), 0.431);
    golden("w2+w3", waarde_van(k, "doorbuiging_w2w3"), 7.32, 2);
    golden("w_max", waarde_van(k, "doorbuiging_wmax"), 8.50, 2);
    // w_1kN > 1 mm/kN: voldoet niet, dus geen advies.
    assert!(!k.voldoet());
    assert!(r.advies.is_none());
}

#[test]
fn h2_trilling_uit_automatisch_geeft_75x221() {
    let r = bereken_hout(&invoer(None, false, None)).unwrap();
    let a = r.advies.as_ref().expect("advies");
    assert_eq!(a.naam, "75 x 221");
    let k = r.kandidaten.iter().find(|k| k.naam == a.naam).unwrap();
    uc("UC buiging", uc_van(k, "buiging"), 0.668);
    uc("UC w2+w3", uc_van(k, "doorbuiging_w2w3"), 0.783);
    for id in ["trilling_f1", "trilling_w1kn", "trilling_v"] {
        let t = k.toetsen.iter().find(|t| t.id == id).unwrap();
        assert!(matches!(
            &t.status,
            ToetsStatus::NietGetoetst { reden, blokkeert_advies: false } if reden == "uitgezet door gebruiker"
        ));
    }
    assert!(r
        .meldingen
        .iter()
        .any(|m| m.tekst == constructie_core::hout::TRILLING_UIT_ZIN));
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn h2_trilling_aan_automatisch_huidige_stand_geen_advies() {
    // Huidige stand (F1_LAAG_BLOKKEERT = true): 221 heeft f_1 < 8 Hz en wordt op f_1
    // geblokkeerd; de hogere kandidaten falen op w_1kN. Geen golden: het besluit volgt.
    assert!(constructie_core::hout::F1_LAAG_BLOKKEERT);
    let r = bereken_hout(&invoer(None, true, None)).unwrap();
    let k221 = r.kandidaten.iter().find(|k| k.hoogte_mm == 221.0).unwrap();
    let f1 = k221.toetsen.iter().find(|t| t.id == "trilling_f1").unwrap();
    assert!(f1.blokkeert());
    assert!(matches!(
        f1.status,
        ToetsStatus::NietGetoetst {
            blokkeert_advies: true,
            ..
        }
    ));
    assert!(r.advies.is_none());
}

#[test]
fn h2_trilling_aan_k221_f1_blokkeert_en_w1kn_informatief() {
    let r = bereken_hout(&invoer(None, true, None)).unwrap();
    let k = r.kandidaten.iter().find(|k| k.hoogte_mm == 221.0).unwrap();
    golden("f_1", k.tussenwaarden["f_1"], 7.60, 2);
    golden("w_1kN", waarde_van(k, "trilling_w1kn"), 2.275, 3);
    let t = k.toetsen.iter().find(|t| t.id == "trilling_w1kn").unwrap();
    assert!(matches!(
        &t.status,
        ToetsStatus::NietGetoetst {
            blokkeert_advies: false,
            ..
        }
    ));
}

const EI_DWARS: f64 = 1.4298e6;

#[test]
fn h2_dwarsverbinding_vaste_hoogte_275() {
    let r = bereken_hout(&invoer(Some(275.0), true, Some(EI_DWARS))).unwrap();
    let k = &r.kandidaten[0];
    golden("b_ef", k.tussenwaarden["b_ef"], 3.282, 3);
    golden("w_1kN", k.tussenwaarden["w_1kn"], 0.384, 3);
    let t = k.toetsen.iter().find(|t| t.id == "trilling_w1kn").unwrap();
    assert_eq!(t.bron.tekst(), "EN 1995-1-1:2026 9.3.2.5 (9.19)");
    assert!(matches!(&t.status, ToetsStatus::Aanname { tekst } if tekst.contains("1/3-punten")));
}

#[test]
fn h2_dwarsverbinding_automatisch_geeft_75x246() {
    let r = bereken_hout(&invoer(None, true, Some(EI_DWARS))).unwrap();
    let a = r.advies.as_ref().expect("advies");
    assert_eq!(a.naam, "75 x 246");
    let k = r.kandidaten.iter().find(|k| k.naam == a.naam).unwrap();
    golden("b_ef", k.tussenwaarden["b_ef"], 3.568, 3);
    golden("w_1kN", k.tussenwaarden["w_1kn"], 0.493, 3);
    uc("UC v", uc_van(k, "trilling_v"), 0.985);
    // 221: f_1 < 8 Hz blokkeert.
    assert!(!r
        .kandidaten
        .iter()
        .find(|k| k.hoogte_mm == 221.0)
        .unwrap()
        .voldoet());
}
