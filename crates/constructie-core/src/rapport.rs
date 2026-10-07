//! Rapportbouwer: zet een resultaat om naar rapport-JSON voor openaec-reports.
//!
//! De functie is puur (geen klok, geen I/O): de datum wordt meegegeven. De tekst E1
//! staat altijd in het rapport en kan niet worden uitgeschakeld.

use serde::Serialize;
use serde_json::{json, Value};

use crate::belasting::{bepaal_belasting, combinatie};
use crate::beton::{dekking, materiaal};
use crate::hout::{k_def, k_mod, Duur, GAMMA_M, K_CR};
use crate::model::{
    Advies, BetonInvoer, BetonResultaat, Gebouwtype, HoutElement, HoutInvoer, HoutResultaat,
    Kandidaat, Resultaat, StaalInvoer, StaalResultaat,
};
use crate::toets::{fmt_getal, fmt_uc, Melding, MeldingSoort, Toets, ToetsStatus};
use crate::{DISCLAIMER, KERN_VERSIE, NORMEDITIES};

/// De berekening waarvoor een rapport wordt gemaakt.
#[derive(Debug, Clone, Copy)]
pub enum RapportBerekening<'a> {
    /// Stalen ligger.
    Staal {
        /// Invoer.
        invoer: &'a StaalInvoer,
        /// Resultaat.
        resultaat: &'a StaalResultaat,
    },
    /// Betonbalk.
    Beton {
        /// Invoer.
        invoer: &'a BetonInvoer,
        /// Resultaat.
        resultaat: &'a BetonResultaat,
    },
    /// Houten balk of balklaag.
    Hout {
        /// Invoer.
        invoer: &'a HoutInvoer,
        /// Resultaat.
        resultaat: &'a HoutResultaat,
    },
}

/// Invoer voor [`rapport_json`].
#[derive(Debug, Clone, Copy)]
pub struct RapportInvoer<'a> {
    /// Projectnaam.
    pub project: &'a str,
    /// Projectnummer.
    pub projectnummer: &'a str,
    /// Opdrachtgever.
    pub opdrachtgever: &'a str,
    /// Auteur.
    pub auteur: &'a str,
    /// Datum (ISO 8601).
    pub datum: &'a str,
    /// De berekening.
    pub berekening: RapportBerekening<'a>,
}

/// Maak een tekst veilig voor het opmaaksysteem en vervang vergelijkingstekens.
fn esc(tekst: &str) -> String {
    tekst
        .replace('&', "&amp;")
        .replace("<=", "\u{2264}")
        .replace(">=", "\u{2265}")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Leesbare tekst voor het gebouwtype.
fn gebouwtype_tekst(g: Gebouwtype) -> &'static str {
    match g {
        Gebouwtype::Eengezinswoning1Tot3 => "eengezinswoning, 1 tot 3 bouwlagen",
        Gebouwtype::Eengezinswoning4Plus => "eengezinswoning, 4 of meer bouwlagen",
        Gebouwtype::Woongebouw => "woongebouw",
        Gebouwtype::Kantoorgebouw => "kantoorgebouw",
    }
}

fn label<T: Serialize>(waarde: &T) -> String {
    match serde_json::to_value(waarde) {
        Ok(Value::String(s)) => s.replace('_', " "),
        Ok(v) => v.to_string(),
        Err(_) => String::new(),
    }
}

fn ja_nee(b: bool) -> &'static str {
    if b {
        "ja"
    } else {
        "nee"
    }
}

fn tabel(titel: &str, koppen: &[&str], rijen: Vec<Vec<String>>) -> Value {
    json!({
        "type": "table",
        "title": esc(titel),
        "headers": koppen.iter().map(|k| esc(k)).collect::<Vec<_>>(),
        "rows": rijen.iter().map(|r| r.iter().map(|c| esc(c)).collect::<Vec<_>>()).collect::<Vec<_>>(),
    })
}

fn alinea(tekst: &str) -> Value {
    json!({ "type": "paragraph", "text": esc(tekst) })
}

fn sectie(titel: &str, inhoud: Vec<Value>) -> Value {
    json!({ "title": titel, "level": 1, "content": inhoud })
}

fn spatie() -> Value {
    json!({ "type": "spacer", "height_mm": 4 })
}

fn getal_met_eenheid(w: Option<f64>, eenheid: &str) -> String {
    match w {
        Some(w) if eenheid.is_empty() || eenheid == "-" => fmt_getal(w, 2),
        Some(w) => format!("{} {}", fmt_getal(w, 2), eenheid),
        None => "-".to_string(),
    }
}

fn status_tekst(t: &Toets) -> String {
    match &t.status {
        ToetsStatus::Voldoet => "voldoet".to_string(),
        ToetsStatus::VoldoetNiet => "voldoet niet".to_string(),
        ToetsStatus::NietGetoetst { reden, .. } => format!("niet getoetst - {reden}"),
        ToetsStatus::Aanname { .. } => "aanname".to_string(),
    }
}

fn toetsrijen(toetsen: &[Toets]) -> Vec<Vec<String>> {
    toetsen
        .iter()
        .map(|t| {
            vec![
                format!("{} ({})", t.naam, t.formule),
                getal_met_eenheid(t.waarde, &t.eenheid),
                getal_met_eenheid(t.grens, &t.eenheid),
                t.uc.map_or_else(|| "-".to_string(), fmt_uc),
                t.bron.tekst(),
                status_tekst(t),
            ]
        })
        .collect()
}

fn melding_rij(m: &Melding) -> Vec<String> {
    let soort = match m.soort {
        MeldingSoort::Waarschuwing => "waarschuwing",
        MeldingSoort::Signalering => "signalering",
        MeldingSoort::Info => "info",
    };
    vec![
        soort.to_string(),
        m.tekst.clone(),
        m.bron
            .as_ref()
            .map_or_else(|| "-".to_string(), |b| b.tekst()),
    ]
}

/// Bouw de rapport-JSON (formaat van de openaec-reports `standaard_rapport`).
pub fn rapport_json(invoer: &RapportInvoer<'_>) -> Value {
    let (titel, resultaat, invoerrijen, uitgangspunten) = match invoer.berekening {
        RapportBerekening::Staal {
            invoer: i,
            resultaat,
        } => (
            "Voordimensionering stalen ligger",
            resultaat,
            staal_invoerrijen(i),
            staal_uitgangspunten(i, resultaat),
        ),
        RapportBerekening::Beton {
            invoer: i,
            resultaat,
        } => (
            "Voordimensionering betonbalk",
            resultaat,
            beton_invoerrijen(i),
            beton_uitgangspunten(i, resultaat),
        ),
        RapportBerekening::Hout {
            invoer: i,
            resultaat,
        } => (
            "Voordimensionering houten balk of balklaag",
            resultaat,
            hout_invoerrijen(i),
            hout_uitgangspunten(i, resultaat),
        ),
    };

    let mut secties = vec![
        sectie(
            "Disclaimer",
            vec![json!({ "type": "paragraph", "text": format!("<b>{}</b>", esc(DISCLAIMER)) })],
        ),
        sectie(
            "Invoer",
            vec![tabel("Invoer", &["Parameter", "Waarde"], invoerrijen)],
        ),
        sectie(
            "Uitgangspunten",
            vec![tabel(
                "Uitgangspunten",
                &["Uitgangspunt", "Waarde", "Bron"],
                uitgangspunten,
            )],
        ),
    ];
    secties.push(advies_sectie(resultaat));
    secties.push(alternatieven_sectie(resultaat));
    secties.push(aannames_sectie(resultaat));
    secties.push(sectie(
        "Rekenkern en normedities",
        vec![
            alinea(&format!("Versie rekenkern: {KERN_VERSIE}")),
            tabel(
                "Normedities",
                &["Sleutel", "Editie"],
                NORMEDITIES
                    .iter()
                    .map(|(k, e)| vec![(*k).to_string(), (*e).to_string()])
                    .collect(),
            ),
        ],
    ));

    json!({
        "template": "standaard_rapport",
        "format": "A4",
        "orientation": "portrait",
        "project": esc(invoer.project),
        "project_number": esc(invoer.projectnummer),
        "client": esc(invoer.opdrachtgever),
        "author": esc(invoer.auteur),
        "date": invoer.datum,
        "version": "1.0",
        "status": "CONCEPT",
        "cover": { "subtitle": titel },
        "colofon": { "enabled": false },
        "toc": { "enabled": false },
        "sections": secties,
        "backcover": { "enabled": false },
        "metadata": {
            "engine": "constructie-core",
            "kern_versie": KERN_VERSIE,
            "disclaimer": DISCLAIMER,
        },
    })
}

fn algemene_invoerrijen(a: &crate::model::Algemeen) -> Vec<Vec<String>> {
    let r = |n: &str, w: String| vec![n.to_string(), w];
    let mut rijen = vec![
        r("Toepassing", label(&a.toepassing)),
        r(
            "Overspanning l",
            format!("{} m", fmt_getal(a.overspanning_m, 2)),
        ),
        r(
            "Permanente belasting g_k (zonder eigen gewicht)",
            format!("{} kN/m2", fmt_getal(a.permanent_kn_m2, 2)),
        ),
        r(
            "Eigen gewicht automatisch",
            ja_nee(a.eigen_gewicht_automatisch).to_string(),
        ),
        r("Gebouwtype", gebouwtype_tekst(a.gebouwtype).to_string()),
        r(
            "Gevolgklasse (invoer)",
            a.gevolgklasse
                .map_or_else(|| "volgt gebouwtype".to_string(), |g| label(&g)),
        ),
        r("Lichte scheidingswanden", label(&a.lichte_scheidingswanden)),
        r(
            "Scheurgevoelige scheidingswanden",
            ja_nee(a.scheurgevoelige_scheidingswanden).to_string(),
        ),
        r(
            "Uiterlijk van belang",
            ja_nee(a.uiterlijk_van_belang).to_string(),
        ),
    ];
    if a.toepassing == crate::model::Toepassing::Dak {
        rijen.push(r("Dak beloopbaar", ja_nee(a.dak_beloopbaar).to_string()));
        rijen.push(r(
            "Dak hellingshoek",
            format!("{} graden", fmt_getal(a.dak_hellingshoek_graden, 1)),
        ));
        rijen.push(r(
            "Dak afschot",
            format!("{} %", fmt_getal(a.dak_afschot_procent, 1)),
        ));
    }
    if a.toepassing == crate::model::Toepassing::Vloer || a.dak_beloopbaar {
        rijen.insert(1, r("Gebruiksfunctie", label(&a.gebruiksfunctie)));
    }
    rijen
}

fn staal_invoerrijen(i: &StaalInvoer) -> Vec<Vec<String>> {
    let mut rijen = algemene_invoerrijen(&i.algemeen);
    let r = |n: &str, w: String| vec![n.to_string(), w];
    rijen.push(r("Element", label(&i.element)));
    rijen.push(r(
        "Belastingbreedte",
        format!("{} m", fmt_getal(i.belastingbreedte_m, 2)),
    ));
    rijen.push(r("Staalsoort", label(&i.staalsoort)));
    rijen.push(r(
        "Profielreeksen",
        i.reeksen
            .iter()
            .map(|x| x.naam())
            .collect::<Vec<_>>()
            .join(", "),
    ));
    rijen.push(r(
        "Bovenflens doorgaand gesteund",
        ja_nee(i.bovenflens_gesteund).to_string(),
    ));
    rijen
}

fn beton_invoerrijen(i: &BetonInvoer) -> Vec<Vec<String>> {
    let mut rijen = algemene_invoerrijen(&i.algemeen);
    let r = |n: &str, w: String| vec![n.to_string(), w];
    rijen.push(r("Element", label(&i.element)));
    rijen.push(r(
        "Belastingbreedte",
        format!("{} m", fmt_getal(i.belastingbreedte_m, 2)),
    ));
    rijen.push(r("Sterkteklasse", label(&i.sterkteklasse)));
    rijen.push(r(
        "Balkbreedte",
        format!("{} mm", fmt_getal(i.balkbreedte_mm, 0)),
    ));
    rijen.push(r(
        "Hoogte",
        if i.hoogte_automatisch {
            "automatisch (stappen van 50 mm)".to_string()
        } else {
            format!("{} mm", fmt_getal(i.hoogte_mm.unwrap_or(0.0), 0))
        },
    ));
    rijen.push(r("Milieuklasse", label(&i.milieuklasse)));
    rijen.push(r(
        "Aangenomen hoofdstaafdiameter",
        format!("{} mm", fmt_getal(i.phi_hoofd_mm, 0)),
    ));
    rijen.push(r(
        "Beugeldiameter",
        format!("{} mm", fmt_getal(i.phi_beugel_mm, 0)),
    ));
    rijen.push(r(
        "Grootste korrelafmeting",
        format!("{} mm", fmt_getal(i.d_g_mm, 0)),
    ));
    rijen
}

fn hout_invoerrijen(i: &HoutInvoer) -> Vec<Vec<String>> {
    let mut rijen = algemene_invoerrijen(&i.algemeen);
    let r = |n: &str, w: String| vec![n.to_string(), w];
    match i.element {
        HoutElement::Balklaag { hoh_mm } => {
            rijen.push(r("Element", "balklaag".to_string()));
            rijen.push(r(
                "Hart-op-hart-afstand",
                format!("{} mm", fmt_getal(hoh_mm, 0)),
            ));
        }
        HoutElement::Balk { belastingbreedte_m } => {
            rijen.push(r("Element", "balk".to_string()));
            rijen.push(r(
                "Belastingbreedte",
                format!("{} m", fmt_getal(belastingbreedte_m, 2)),
            ));
        }
    }
    rijen.push(r("Sterkteklasse", i.sterkteklasse.naam().to_string()));
    rijen.push(r("Breedte", format!("{} mm", fmt_getal(i.breedte_mm, 0))));
    rijen.push(r(
        "Hoogte",
        if i.hoogte_automatisch {
            "automatisch (assortiment 146 - 296 mm)".to_string()
        } else {
            format!("{} mm", fmt_getal(i.hoogte_mm.unwrap_or(0.0), 0))
        },
    ));
    rijen.push(r("Klimaatklasse", i.klimaatklasse.to_string()));
    rijen.push(r(
        "Drukrand doorgaand gesteund",
        ja_nee(i.drukrand_gesteund).to_string(),
    ));
    if let Some(p) = &i.vloerplaat {
        rijen.push(r(
            "Vloerplaat",
            format!(
                "dikte {} mm; E = {} N/mm2 (invoer gebruiker)",
                fmt_getal(p.dikte_mm, 0),
                fmt_getal(p.e_mean_n_mm2, 0)
            ),
        ));
    }
    if let Some(b) = i.vloerbreedte_m {
        rijen.push(r("Vloerbreedte B", format!("{} m", fmt_getal(b, 2))));
    }
    rijen
}

fn hout_uitgangspunten(i: &HoutInvoer, res: &Resultaat) -> Vec<Vec<String>> {
    let mut rijen = algemene_uitgangspunten(&i.algemeen, res);
    rijen.push(vec![
        format!("Klasse {}", i.sterkteklasse.naam()),
        "waarden per klasse uit de klassendata van de kern".to_string(),
        "EN 338 sterkteklassen, secundaire bron, te verifieren".to_string(),
    ]);
    if let Some(k) = res.kandidaten.first() {
        let tw = &k.tussenwaarden;
        if let (Some(kr), Some(f_m_d), Some(f_v_d)) =
            (tw.get("k_r"), tw.get("f_m_d"), tw.get("f_v_d"))
        {
            rijen.push(vec![
                "Reductiefactor puntlastmoment k_r".to_string(),
                fmt_getal(*kr, 3),
                "EC5 NB 5.2(5)".to_string(),
            ]);
            rijen.push(vec![
                "f_m,d / f_v,d bij k_mod voor G + Q (eerste kandidaat)".to_string(),
                format!("{} / {} N/mm2", fmt_getal(*f_m_d, 2), fmt_getal(*f_v_d, 2)),
                "EC5 2.4.1 (2.14)".to_string(),
            ]);
        }
    }
    rijen.push(vec![
        "k_mod blijvend / lang / middellang / kort / zeer kort".to_string(),
        [
            Duur::Blijvend,
            Duur::Lang,
            Duur::Middellang,
            Duur::Kort,
            Duur::ZeerKort,
        ]
        .iter()
        .map(|d| fmt_getal(k_mod(*d), 2))
        .collect::<Vec<_>>()
        .join(" / "),
        "EC5 tabel 3.1".to_string(),
    ]);
    rijen.push(vec![
        "Belastingduur: eigen gewicht blijvend; vloerbelasting middellang; sneeuw en dak H kort"
            .to_string(),
        "kortste duur in de combinatie; G alleen met k_mod blijvend".to_string(),
        "EC5 NB 2.3.1.2 tabel 2.2; EC5 2.3.1.2 tabel 2.1; 3.1.3(2)".to_string(),
    ]);
    rijen.push(vec![
        "k_def".to_string(),
        fmt_getal(k_def(i.klimaatklasse), 2),
        "EC5 tabel 3.2".to_string(),
    ]);
    rijen.push(vec![
        "gamma_M; k_cr".to_string(),
        format!("{}; {}", fmt_getal(GAMMA_M, 2), fmt_getal(K_CR, 1)),
        "EC5 tabel 2.3; EC5 NB 6.1.7(2)".to_string(),
    ]);
    rijen.push(vec![
        "Eigen gewicht hout".to_string(),
        "rho_mean / 100 kN/m3".to_string(),
        "EC1-1 tabel A.3".to_string(),
    ]);
    rijen
}

fn algemene_uitgangspunten(a: &crate::model::Algemeen, res: &Resultaat) -> Vec<Vec<String>> {
    let mut rijen = vec![vec![
        "Gevolgklasse (toegepast)".to_string(),
        label(&res.gevolgklasse),
        "EC0 NB tabel NB.24-B1".to_string(),
    ]];
    let c = combinatie(res.gevolgklasse);
    rijen.push(vec![
        "Combinaties UGT (6.10a en 6.10b, ongunstigste)".to_string(),
        format!(
            "gamma_G = {} / {}; gamma_Q = {}",
            fmt_getal(c.gamma_g_a, 2),
            fmt_getal(c.gamma_g_b, 2),
            fmt_getal(c.gamma_q, 2)
        ),
        "EC0 NB A1.3.1(1), tabel NB.4-A1.2(B)".to_string(),
    ]);
    if let Ok(b) = bepaal_belasting(a) {
        rijen.push(vec![
            format!("Veranderlijke belasting ({})", b.label),
            format!(
                "q_k = {} kN/m2; Q_k = {} kN; psi_0/1/2 = {}/{}/{}",
                fmt_getal(b.q_k, 2),
                fmt_getal(b.q_punt_kn, 1),
                fmt_getal(b.psi0, 1),
                fmt_getal(b.psi1, 1),
                fmt_getal(b.psi2, 1)
            ),
            b.bron_q.tekst(),
        ]);
    }
    rijen
}

fn staal_uitgangspunten(i: &StaalInvoer, res: &Resultaat) -> Vec<Vec<String>> {
    let mut rijen = algemene_uitgangspunten(&i.algemeen, res);
    if let Some(fy) = res
        .kandidaten
        .first()
        .and_then(|k| k.tussenwaarden.get("f_y"))
    {
        rijen.push(vec![
            "Vloeigrens f_y (grootste plaatdikte)".to_string(),
            format!("{} N/mm2", fmt_getal(*fy, 0)),
            "EC3 tabel 3.1".to_string(),
        ]);
    }
    rijen.push(vec![
        "E-modulus; gamma_M0 / gamma_M1".to_string(),
        "210000 N/mm2; 1,00 / 1,00".to_string(),
        "EC3 3.2.6(1); EC3 NB 6.1(1)".to_string(),
    ]);
    rijen.push(vec![
        "Eigen gewicht staal".to_string(),
        "78,5 kN/m3".to_string(),
        "EC1-1 tabel A.4".to_string(),
    ]);
    rijen.push(vec![
        "Profieldata".to_string(),
        "voorlopig: nominale waarden, bron volgt".to_string(),
        "-".to_string(),
    ]);
    rijen
}

fn beton_uitgangspunten(i: &BetonInvoer, res: &Resultaat) -> Vec<Vec<String>> {
    let mut rijen = algemene_uitgangspunten(&i.algemeen, res);
    let (f_ck, f_ctm, e_cm) = materiaal(i.sterkteklasse);
    rijen.push(vec![
        "Beton: f_ck / f_ctm / E_cm".to_string(),
        format!(
            "{} / {} MPa / {} GPa",
            fmt_getal(f_ck, 0),
            fmt_getal(f_ctm, 1),
            fmt_getal(e_cm, 0)
        ),
        "EC2 tabel 3.1".to_string(),
    ]);
    rijen.push(vec![
        "Partiele factoren gamma_C / gamma_S; wapening".to_string(),
        "1,50 / 1,15; B500".to_string(),
        "EC2 tabel 2.1N".to_string(),
    ]);
    let d = dekking(
        i.sterkteklasse,
        i.milieuklasse,
        i.phi_hoofd_mm,
        i.phi_beugel_mm,
    );
    rijen.push(vec![
        "Constructieklasse; c_min,dur".to_string(),
        format!("S{}; {} mm", d.constructieklasse, fmt_getal(d.c_min_dur, 0)),
        "EC2 NB 4.4.1.2(5), tabel 4.4N".to_string(),
    ]);
    rijen.push(vec![
        "Dekking c_nom hoofdstaaf / beugel; dekking op beugel".to_string(),
        format!(
            "{} / {} mm; {} mm",
            fmt_getal(d.c_nom_hoofd, 0),
            fmt_getal(d.c_nom_beugel, 0),
            fmt_getal(d.beugeldekking, 0)
        ),
        "EC2 NB 4.4.1.3(1)P".to_string(),
    ]);
    rijen.push(vec![
        "Eigen gewicht gewapend beton".to_string(),
        "25 kN/m3".to_string(),
        "EC1-1 tabel A.1".to_string(),
    ]);
    rijen
}

fn kandidaat_van<'a>(res: &'a Resultaat, a: &Advies) -> Option<&'a Kandidaat> {
    res.kandidaten.iter().find(|k| k.naam == a.naam)
}

fn advies_blok(titel: &str, a: &Advies, res: &Resultaat) -> Vec<Value> {
    let mut v = vec![alinea(&format!("{titel}: {}", a.naam))];
    if let Some(m) = &a.maatgevend {
        v.push(alinea(&m.uitleg));
    }
    let kg = &a.kengetallen;
    let mut rijen = vec![
        vec![
            "Constructiehoogte".to_string(),
            format!("{} mm", fmt_getal(kg.constructiehoogte_mm, 0)),
        ],
        vec!["l/h".to_string(), fmt_getal(kg.l_h, 1)],
        vec![
            "Eigen gewicht".to_string(),
            format!("{} kN/m", fmt_getal(kg.eigen_gewicht_kn_m, 2)),
        ],
    ];
    if let Some(g) = kg.gewicht_kg_m {
        rijen.push(vec![
            "Massa".to_string(),
            format!("{} kg/m", fmt_getal(g, 1)),
        ]);
    }
    if let Some(f) = kg.eigenfrequentie_hz {
        rijen.push(vec![
            "Eerste eigenfrequentie".to_string(),
            format!("{} Hz", fmt_getal(f, 1)),
        ]);
    }
    if let Some(b) = &a.beton {
        rijen.push(vec![
            "Hoofdwapening (een laag)".to_string(),
            format!("{} D{}", b.aantal, fmt_getal(b.diameter_mm, 0)),
        ]);
        rijen.push(vec![
            "Beugels (tweesnedig)".to_string(),
            format!(
                "D{}-{}",
                fmt_getal(b.beugel_diameter_mm, 0),
                fmt_getal(b.beugel_s_mm, 0)
            ),
        ]);
    }
    v.push(tabel("Kengetallen", &["Grootheid", "Waarde"], rijen));
    v.push(spatie());
    let _ = res;
    v
}

fn advies_sectie(res: &Resultaat) -> Value {
    let mut inhoud = Vec::new();
    match &res.advies {
        Some(a) => {
            inhoud.extend(advies_blok("Advies", a, res));
            inhoud.push(tabel(
                "Toetsing",
                &["Toets", "Waarde", "Grens", "UC", "Bron", "Status"],
                toetsrijen(&a.toetsen),
            ));
        }
        None => {
            inhoud.push(alinea(
                "Geen advies binnen bereik - constructeur raadplegen.",
            ));
            inhoud.push(tabel(
                "Doorgerekende kandidaten",
                &["Kandidaat", "Hoogste UC", "Opmerking"],
                res.kandidaten
                    .iter()
                    .map(|k| {
                        vec![
                            k.naam.clone(),
                            k.max_uc().map_or_else(|| "-".to_string(), fmt_uc),
                            k.afgewezen.clone().unwrap_or_default(),
                        ]
                    })
                    .collect(),
            ));
        }
    }
    if let Some(l) = &res.laagste_bouwhoogte {
        inhoud.push(spatie());
        inhoud.extend(advies_blok("Laagste bouwhoogte", l, res));
    }
    sectie("Advies en toetsing", inhoud)
}

fn alternatieven_sectie(res: &Resultaat) -> Value {
    let rijen: Vec<Vec<String>> = res
        .alternatieven
        .iter()
        .map(|a| {
            vec![
                a.naam.clone(),
                label(&a.positie),
                a.max_uc.map_or_else(|| "-".to_string(), fmt_uc),
                ja_nee(a.voldoet).to_string(),
                a.toelichting.clone(),
            ]
        })
        .collect();
    let inhoud = if rijen.is_empty() {
        vec![alinea("Geen alternatieven: er is geen advies.")]
    } else {
        vec![tabel(
            "Alternatieven",
            &[
                "Kandidaat",
                "Positie",
                "Hoogste UC",
                "Voldoet",
                "Toelichting",
            ],
            rijen,
        )]
    };
    sectie("Alternatieven", inhoud)
}

fn aannames_sectie(res: &Resultaat) -> Value {
    let mut aannames: Vec<Vec<String>> = Vec::new();
    let bron_kandidaat = res
        .advies
        .as_ref()
        .and_then(|a| kandidaat_van(res, a))
        .or_else(|| res.kandidaten.last());
    if let Some(k) = bron_kandidaat {
        for t in &k.toetsen {
            match &t.status {
                ToetsStatus::Aanname { tekst } => {
                    aannames.push(vec![
                        "aanname".into(),
                        format!("{}: {}", t.naam, tekst),
                        t.bron.tekst(),
                    ]);
                }
                ToetsStatus::NietGetoetst { reden, .. } => {
                    aannames.push(vec![
                        "niet getoetst".into(),
                        format!("{}: {}", t.naam, reden),
                        t.bron.tekst(),
                    ]);
                }
                ToetsStatus::Voldoet | ToetsStatus::VoldoetNiet => {}
            }
        }
    }
    for m in &res.meldingen {
        aannames.push(melding_rij(m));
    }
    sectie(
        "Aannames en open punten",
        vec![tabel(
            "Aannames, niet getoetste onderdelen en meldingen",
            &["Soort", "Toelichting", "Bron"],
            aannames,
        )],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bereken_staal;

    fn staal() -> (StaalInvoer, StaalResultaat) {
        let i: StaalInvoer = serde_json::from_str(
            r#"{ "algemeen": { "overspanning_m": 5.4, "permanent_kn_m2": 0.75 },
            "belastingbreedte_m": 3.6, "staalsoort": "S235", "reeksen": ["IPE","HEA"] }"#,
        )
        .unwrap();
        let r = bereken_staal(&i).unwrap();
        (i, r)
    }

    #[test]
    fn rapport_bevat_disclaimer_en_normedities() {
        let (i, r) = staal();
        let j = rapport_json(&RapportInvoer {
            project: "Test",
            projectnummer: "",
            opdrachtgever: "",
            auteur: "",
            datum: "2026-10-07",
            berekening: RapportBerekening::Staal {
                invoer: &i,
                resultaat: &r,
            },
        });
        let tekst = j.to_string();
        assert!(tekst.contains("Indicatieve voordimensionering"));
        assert!(tekst.contains("NEN-EN 1993-1-1+A1+NB:2016"));
        assert_eq!(j["template"], "standaard_rapport");
        assert!(j["sections"].as_array().map_or(0, Vec::len) >= 6);
        assert_eq!(j["sections"][0]["title"], "Disclaimer");
    }

    #[test]
    fn rapport_zonder_advies_heeft_disclaimer() {
        let (mut i, _) = staal();
        i.bovenflens_gesteund = false;
        let r = bereken_staal(&i).unwrap();
        let j = rapport_json(&RapportInvoer {
            project: "Test",
            projectnummer: "",
            opdrachtgever: "",
            auteur: "",
            datum: "2026-10-07",
            berekening: RapportBerekening::Staal {
                invoer: &i,
                resultaat: &r,
            },
        });
        let tekst = j.to_string();
        assert!(tekst.contains("Geen advies binnen bereik"));
        assert!(tekst.contains("vervangt geen constructieve berekening"));
    }
}
