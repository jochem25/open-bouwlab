//! Advieslogica: kandidaten, advies, maatgevende toets en alternatieven.

use crate::model::{
    Advies, Alternatief, AlternatiefPositie, Gevolgklasse, Kandidaat, Kengetallen, Maatgevend,
    Resultaat,
};
use crate::toets::{fmt_uc, Melding, ToetsStatus};
use crate::{DISCLAIMER, KERN_VERSIE, NORMEDITIES};

/// Index van het advies: de eerste kandidaat (oplopend) met alle UC <= 1,00 en zonder
/// blokkerende niet-getoetste toets.
pub fn kies_advies(kandidaten: &[Kandidaat]) -> Option<usize> {
    kandidaten.iter().position(Kandidaat::voldoet)
}

/// Index van de voldoende kandidaat met de laagste constructiehoogte.
///
/// Bij gelijke hoogte wint de eerste in de (op gewicht oplopende) volgorde.
pub fn laagste_bouwhoogte(kandidaten: &[Kandidaat]) -> Option<usize> {
    let mut beste: Option<usize> = None;
    for (i, k) in kandidaten.iter().enumerate() {
        if !k.voldoet() {
            continue;
        }
        match beste {
            Some(b) if kandidaten[b].hoogte_mm <= k.hoogte_mm => {}
            _ => beste = Some(i),
        }
    }
    beste
}

/// Toets met de hoogste UC van een kandidaat, met een korte uitleg.
pub fn maatgevend(k: &Kandidaat) -> Option<Maatgevend> {
    let toets = k.toetsen.iter().filter(|t| t.uc.is_some()).max_by(|a, b| {
        a.uc.unwrap_or(0.0)
            .partial_cmp(&b.uc.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;
    let uc = toets.uc?;
    Some(Maatgevend {
        toets_id: toets.id.clone(),
        naam: toets.naam.clone(),
        uc,
        uitleg: format!(
            "{} heeft de hoogste unity check ({}) en bepaalt daarmee de afmeting van {}.",
            toets.naam,
            fmt_uc(uc),
            k.naam
        ),
    })
}

/// Bouw een [`Advies`] uit een kandidaat.
pub fn bouw_advies(k: &Kandidaat, overspanning_m: f64) -> Advies {
    Advies {
        naam: k.naam.clone(),
        hoogte_mm: k.hoogte_mm,
        maatgevend: maatgevend(k),
        kengetallen: Kengetallen {
            constructiehoogte_mm: k.hoogte_mm,
            l_h: overspanning_m * 1000.0 / k.hoogte_mm,
            eigen_gewicht_kn_m: k.eigen_gewicht_kn_m,
            gewicht_kg_m: k.gewicht_kg_m,
            eigenfrequentie_hz: None,
        },
        toetsen: k.toetsen.clone(),
        beton: k.beton.clone(),
    }
}

fn alternatief(k: &Kandidaat, positie: AlternatiefPositie) -> Alternatief {
    let m = maatgevend(k);
    let voldoet = k.voldoet();
    let toelichting = if let Some(reden) = &k.afgewezen {
        format!("Afgewezen: {reden}.")
    } else if let Some(t) = k.toetsen.iter().find(|t| {
        matches!(
            t.status,
            ToetsStatus::NietGetoetst {
                blokkeert_advies: true,
                ..
            }
        )
    }) {
        format!("Niet getoetst: {}.", t.naam)
    } else if !voldoet {
        match &m {
            Some(m) => format!("Voldoet niet: {} (UC {}).", m.naam, fmt_uc(m.uc)),
            None => "Voldoet niet.".to_string(),
        }
    } else if positie == AlternatiefPositie::Boven {
        "Voldoet, maar zwaarder of hoger dan het advies.".to_string()
    } else {
        "Voldoet.".to_string()
    };
    Alternatief {
        naam: k.naam.clone(),
        positie,
        voldoet,
        max_uc: k.max_uc(),
        maatgevend: m.map(|m| m.naam),
        toelichting,
    }
}

/// Een kandidaat onder het advies en tot drie erboven.
pub fn alternatieven(kandidaten: &[Kandidaat], advies: Option<usize>) -> Vec<Alternatief> {
    let Some(i) = advies else {
        return Vec::new();
    };
    let mut uit = Vec::new();
    if i > 0 {
        uit.push(alternatief(&kandidaten[i - 1], AlternatiefPositie::Onder));
    }
    for k in kandidaten.iter().skip(i + 1).take(3) {
        uit.push(alternatief(k, AlternatiefPositie::Boven));
    }
    uit
}

/// Stel het resultaat samen uit de (al oplopend gesorteerde) kandidaten.
///
/// `met_laagste_hoogte` bepaalt of het tweede advies "laagste bouwhoogte" wordt gegeven (staal).
pub fn stel_resultaat_samen(
    kandidaten: Vec<Kandidaat>,
    gevolgklasse: Gevolgklasse,
    overspanning_m: f64,
    basis_meldingen: Vec<Melding>,
    met_laagste_hoogte: bool,
) -> Resultaat {
    let idx = kies_advies(&kandidaten);
    let advies = idx.map(|i| bouw_advies(&kandidaten[i], overspanning_m));
    let laagste = if met_laagste_hoogte {
        laagste_bouwhoogte(&kandidaten).map(|i| bouw_advies(&kandidaten[i], overspanning_m))
    } else {
        None
    };
    let alt = alternatieven(&kandidaten, idx);

    let mut meldingen = basis_meldingen;
    match idx {
        Some(i) => {
            for m in &kandidaten[i].meldingen {
                if !meldingen.contains(m) {
                    meldingen.push(m.clone());
                }
            }
        }
        None => meldingen.push(Melding::waarschuwing(
            "Geen advies binnen bereik - constructeur raadplegen.",
        )),
    }

    Resultaat {
        disclaimer: DISCLAIMER.to_string(),
        kern_versie: KERN_VERSIE.to_string(),
        normedities: NORMEDITIES
            .iter()
            .map(|(k, e)| (k.to_string(), e.to_string()))
            .collect(),
        gevolgklasse,
        kandidaten,
        advies,
        laagste_bouwhoogte: laagste,
        alternatieven: alt,
        meldingen,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Gevolgklasse;
    use crate::toets::{Bron, Toets};
    use std::collections::BTreeMap;

    fn kand(naam: &str, h: f64, uc: f64, blokkeert: bool) -> Kandidaat {
        let mut toetsen = vec![
            Toets::nieuw("a", "buiging", "x", Bron::basis("EC3", "6.2.5"))
                .waarden(uc, 1.0, "-")
                .uc_uit_waarden(),
        ];
        if blokkeert {
            toetsen.push(
                Toets::nieuw("b", "kip", "x", Bron::basis("EC3", "6.3.2"))
                    .niet_getoetst("test", true),
            );
        }
        Kandidaat {
            naam: naam.into(),
            hoogte_mm: h,
            eigen_gewicht_kn_m: 0.2,
            gewicht_kg_m: None,
            toetsen,
            meldingen: vec![],
            afgewezen: None,
            tussenwaarden: BTreeMap::new(),
            beton: None,
        }
    }

    #[test]
    fn eerste_voldoende_kandidaat_wint() {
        let k = vec![
            kand("a", 100.0, 1.2, false),
            kand("b", 120.0, 0.9, false),
            kand("c", 90.0, 0.5, false),
        ];
        assert_eq!(kies_advies(&k), Some(1));
        assert_eq!(laagste_bouwhoogte(&k), Some(2));
    }

    #[test]
    fn blokkerende_niet_getoetst_voorkomt_advies() {
        let k = vec![kand("a", 100.0, 0.5, true), kand("b", 120.0, 0.4, true)];
        assert_eq!(kies_advies(&k), None);
        let r = stel_resultaat_samen(k, Gevolgklasse::CC2, 5.0, vec![], true);
        assert!(r.advies.is_none());
        assert!(r.alternatieven.is_empty());
        assert!(r
            .meldingen
            .iter()
            .any(|m| m.tekst.contains("Geen advies binnen bereik")));
        assert_eq!(r.disclaimer, DISCLAIMER);
    }

    #[test]
    fn uc_precies_een_is_toegestaan_en_net_erboven_niet() {
        let k = vec![kand("a", 100.0, 1.0, false)];
        assert_eq!(kies_advies(&k), Some(0));
        let k = vec![kand("a", 100.0, 1.0000001, false)];
        assert_eq!(kies_advies(&k), None);
    }

    #[test]
    fn alternatieven_een_onder_drie_boven() {
        let k: Vec<_> = (0..6)
            .map(|i| {
                kand(
                    &format!("k{i}"),
                    100.0 + i as f64,
                    if i < 2 { 1.1 } else { 0.8 },
                    false,
                )
            })
            .collect();
        let r = stel_resultaat_samen(k, Gevolgklasse::CC2, 5.0, vec![], false);
        assert_eq!(r.advies.as_ref().map(|a| a.naam.as_str()), Some("k2"));
        let namen: Vec<_> = r.alternatieven.iter().map(|a| a.naam.as_str()).collect();
        assert_eq!(namen, ["k1", "k3", "k4", "k5"]);
        assert!(!r.alternatieven[0].voldoet);
        assert!(r.laagste_bouwhoogte.is_none());
    }

    #[test]
    fn afgewezen_kandidaat_is_nooit_advies() {
        let mut a = kand("a", 100.0, 0.1, false);
        a.afgewezen = Some("wapening past niet in een laag".into());
        assert_eq!(kies_advies(&[a]), None);
    }
}
