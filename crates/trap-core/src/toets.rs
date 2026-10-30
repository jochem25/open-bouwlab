//! Toetsing van een trapontwerp, met bron per regel.
//!
//! Bbl (normcorpus kennis-v2, toestand 2026-09-24, alle genoemde artikelen
//! `artikel_status: geldend`):
//! - art. 4.26 lid 1 + tabel 4.26 (`wet/bbl__afdeling-4-2` r. 800-845): breedte,
//!   vrije hoogte, aantrede klimlijn, optrede, tredevlak, tredevlak klimlijn,
//!   klimlijn tot zijkanten; kolom woonfunctie of andere gebruiksfunctie;
//! - art. 4.26 lid 2 (r. 806-808): hoogteverschil max 4 m (tabel 4.24, r. 685-787:
//!   niet voor industriefunctie en overige gebruiksfunctie);
//! - art. 4.26a, 4.27, 4.28 (r. 847-870): markering, bordes, leuning;
//! - verbouw: art. 5.4 lid 1 en 5.5 (`wet/bbl__afdeling-5-2` r. 18-50): rechtens
//!   verkregen niveau, ondergrens bestaande bouw art. 3.20 tabel 3.20
//!   (`wet/bbl__afdeling-3-2` r. 405-437). Een overschrijding van het
//!   nieuwbouwniveau wordt bij verbouw GEMELD, niet afgekeurd;
//! - begrippen klimlijn en vrije hoogte: bijlage I (`wet/bbl__bijlage-i` r. 403-404,
//!   r. 872-873).
//!
//! Wattjes (`toelichting/wattjes-deel8__h2-*`, leerboek, niet bindend) en
//! praktijkregels alleen als advies.

use serde::Serialize;

use crate::invoer::{Bouwsoort, Doorlopend, Gebruiksfunctie};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Oordeel {
    Voldoet,
    VoldoetNiet,
    /// Verbouw: onder het nieuwbouwniveau; rechtens verkregen niveau bepaalt.
    Melding,
    AdviesVoldoet,
    AdviesAfwijking,
    /// Niet door de rekenkern te beoordelen; in het IFC of op de tekening nagaan.
    Handmatig,
    NietVanToepassing,
}

#[derive(Debug, Clone, Serialize)]
pub struct Regel {
    pub id: String,
    pub onderwerp: String,
    pub eis: String,
    pub grenswaarde: Option<f64>,
    pub waarde: Option<f64>,
    pub eenheid: String,
    pub oordeel: Oordeel,
    pub toelichting: String,
    pub bron: String,
    pub bron_id: String,
    pub regel: String,
}

/// Wat de rekenkern gemeten heeft (alles in mm).
/// Grootste knik (graden) waaronder de looplijn als vloeiend telt; de
/// boogbenadering knikt 90 / `BOOG_SEGMENTEN` = 0,25 graad per koorde.
pub const KNIK_VLOEIEND: f64 = 1.0;

#[derive(Debug, Clone, Serialize)]
pub struct Meting {
    pub gebruiksfunctie: Gebruiksfunctie,
    pub bouwsoort: Bouwsoort,
    pub hoogte: f64,
    pub optreden: u32,
    pub optrede: f64,
    /// Aantrede op de klimlijn: horizontale afstand tussen opvolgende
    /// neuspunten, ontwikkeld langs de ingestelde looplijn (meting B, besluit
    /// Jochem 30-10-2026 via planner msg_496305ebd730). Hierop het oordeel.
    pub aantrede_looplijn: f64,
    /// Informatie: kleinste aantrede op de looplijn loodrecht op de voorkant,
    /// vanaf het klimlijnpunt van de volgende voorkant (A) en vanaf de eigen
    /// voorkant (A'). Een strikte lezing van 'loodrecht op de voorkant' geeft
    /// deze lagere waarden.
    pub info_aantrede_loodrecht: f64,
    pub info_aantrede_loodrecht_eigen: f64,
    /// Afstand van de looplijn (= klimlijn) tot binnen- en buitenzijde.
    pub klimlijn_tot_binnenzijde: f64,
    pub klimlijn_tot_buitenzijde: f64,
    /// Informatie: band van mogelijke klimlijnen (afstand uit de buitenzijde);
    /// None als de trap smaller is dan 2 x 300.
    pub klimlijn_band: Option<[f64; 2]>,
    pub neus: f64,
    pub tredevlak_smalst: f64,
    pub loopbreedte_min: f64,
    pub looplijn_afstand_buitenzijde: f64,
    /// Afrondingsstraal rho van de looplijn per binnenhoek (0 = scherp).
    pub looplijn_afronding: Vec<f64>,
    /// Grootste knik van de looplijn (graden); vloeiend (Bbl bijlage I) als
    /// die kleiner is dan `KNIK_VLOEIEND`.
    pub looplijn_grootste_knik: f64,
    pub vrije_hoogte: f64,
    /// None = geen trapgat opgegeven; Some(true) = clearance past in het trapgat.
    pub clearance_in_trapgat: Option<bool>,
    pub boomdikte: f64,
    pub nestdiepte: f64,
    pub voorhout: f64,
    pub achterhout_min_eis: f64,
    pub achterhout_min_gemeten: f64,
    pub verdrijving_waarschuwingen: Vec<String>,
    /// Afstand van het dichtstbijzijnde stootbord tot een buitenhoek (None: geen hoek).
    pub stootbord_tot_buitenhoek: Option<f64>,
    pub stootborddikte: f64,
    pub doorlopend: Option<Doorlopend>,
}

const BBL_42: &str = "wet/bbl__afdeling-4-2";
const BBL_TOESTAND: &str = "geldend, toestand 2026-09-24";
const P34: &str = "toelichting/wattjes-deel8__h2-par3-4";
const P56: &str = "toelichting/wattjes-deel8__h2-par5-6";
const P711: &str = "toelichting/wattjes-deel8__h2-par7-11";

/// Tabel 4.26 (nieuwbouw) en tabel 3.20 (bestaande bouw).
#[derive(Debug, Clone, Copy)]
pub struct Grenzen {
    pub breedte: f64,
    pub vrije_hoogte: f64,
    pub aantrede: f64,
    pub optrede: f64,
    pub tredevlak: f64,
    pub tredevlak_klimlijn: f64,
    pub klimlijn_zijkant: f64,
}

pub fn tabel_4_26(f: Gebruiksfunctie) -> Grenzen {
    if f == Gebruiksfunctie::Woonfunctie {
        Grenzen {
            breedte: 800.0,
            vrije_hoogte: 2300.0,
            aantrede: 220.0,
            optrede: 188.0,
            tredevlak: 50.0,
            tredevlak_klimlijn: 230.0,
            klimlijn_zijkant: 300.0,
        }
    } else {
        Grenzen {
            breedte: 800.0,
            vrije_hoogte: 2100.0,
            aantrede: 185.0,
            optrede: 210.0,
            tredevlak: 50.0,
            tredevlak_klimlijn: 230.0,
            klimlijn_zijkant: 300.0,
        }
    }
}

/// Tabel 3.20 (bestaande bouw): geen rijen voor het tredevlak.
pub struct Grenzen320 {
    pub breedte: f64,
    pub vrije_hoogte: f64,
    pub aantrede: f64,
    pub optrede: f64,
    pub klimlijn_zijkant: f64,
}

pub const TABEL_3_20: Grenzen320 = Grenzen320 {
    breedte: 700.0,
    vrije_hoogte: 1900.0,
    aantrede: 130.0,
    optrede: 220.0,
    klimlijn_zijkant: 200.0,
};

#[derive(Clone, Copy)]
enum Zin {
    Min,
    Max,
}

fn voldoet(waarde: f64, grens: f64, zin: Zin) -> bool {
    match zin {
        Zin::Min => waarde >= grens - 1e-9,
        Zin::Max => waarde <= grens + 1e-9,
    }
}

fn kolom(f: Gebruiksfunctie) -> &'static str {
    if f == Gebruiksfunctie::Woonfunctie {
        "woonfunctie"
    } else {
        "andere gebruiksfunctie"
    }
}

struct Bbl<'a> {
    m: &'a Meting,
    regels: Vec<Regel>,
}

impl Bbl<'_> {
    #[allow(clippy::too_many_arguments)]
    fn tabel(
        &mut self,
        id: &str,
        onderwerp: &str,
        waarde: f64,
        grens: f64,
        zin: Zin,
        bestaand: Option<f64>,
        toelichting: String,
    ) {
        let ok = voldoet(waarde, grens, zin);
        let teken = match zin {
            Zin::Min => ">=",
            Zin::Max => "<=",
        };
        let mut toel = toelichting;
        let oordeel = if ok {
            Oordeel::Voldoet
        } else if self.m.bouwsoort == Bouwsoort::Verbouw {
            let ondergrens = match bestaand {
                Some(b) if voldoet(waarde, b, zin) => format!(
                    "voldoet aan de ondergrens bestaande bouw ({teken} {b}, art. 3.20 tabel 3.20)"
                ),
                Some(b) => format!(
                    "voldoet NIET aan de ondergrens bestaande bouw ({teken} {b}, art. 3.20 tabel 3.20)"
                ),
                None => "tabel 3.20 heeft voor deze afmeting geen ondergrens".into(),
            };
            toel = format!(
                "{toel}{}verbouw: onder het nieuwbouwniveau; het rechtens verkregen niveau \
                 bepaalt (art. 5.4 lid 1, art. 5.5); {ondergrens}",
                if toel.is_empty() { "" } else { "; " }
            );
            Oordeel::Melding
        } else {
            Oordeel::VoldoetNiet
        };
        self.regels.push(Regel {
            id: id.into(),
            onderwerp: onderwerp.into(),
            eis: format!("{teken} {grens} mm ({})", kolom(self.m.gebruiksfunctie)),
            grenswaarde: Some(grens),
            waarde: Some(rond(waarde)),
            eenheid: "mm".into(),
            oordeel,
            toelichting: toel,
            bron: format!("Bbl art. 4.26 lid 1, tabel 4.26 ({BBL_TOESTAND})"),
            bron_id: BBL_42.into(),
            regel: "r. 800-845".into(),
        });
    }
}

/// Bron van de klimlijnregels (aantrede, tredevlak op de klimlijn).
const BRON_KLIMLIJN: &str = "Bbl art. 4.26 lid 1, tabel 4.26 (geldend, toestand 2026-09-24); \
     bijlage I 'klimlijn'; meetwijze: aantreden uitgezet op de klimlijn (Wattjes deel VIII h2, \
     leerboek)";

fn rond(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

#[allow(clippy::too_many_arguments)]
fn advies(
    id: &str,
    onderwerp: &str,
    eis: &str,
    waarde: Option<f64>,
    eenheid: &str,
    ok: bool,
    toelichting: String,
    bron: &str,
    bron_id: &str,
    regel: &str,
) -> Regel {
    Regel {
        id: id.into(),
        onderwerp: onderwerp.into(),
        eis: eis.into(),
        grenswaarde: None,
        waarde: waarde.map(rond),
        eenheid: eenheid.into(),
        oordeel: if ok { Oordeel::AdviesVoldoet } else { Oordeel::AdviesAfwijking },
        toelichting,
        bron: bron.into(),
        bron_id: bron_id.into(),
        regel: regel.into(),
    }
}

/// Alle toetsregels voor een meting.
pub fn toets(m: &Meting) -> Vec<Regel> {
    let g = tabel_4_26(m.gebruiksfunctie);
    let b = &TABEL_3_20;
    let mut t = Bbl { m, regels: Vec::new() };

    t.tabel(
        "bbl-4.26-breedte",
        "minimum breedte van de trap",
        m.loopbreedte_min,
        g.breedte,
        Zin::Min,
        Some(b.breedte),
        "gemeten als loopbreedte tussen de binnenzijden van de bomen".into(),
    );
    // vrije hoogte: de clearance heeft de opgegeven hoogte; past hij in het trapgat?
    let vh_ok = voldoet(m.vrije_hoogte, g.vrije_hoogte, Zin::Min);
    t.tabel(
        "bbl-4.26-vrije-hoogte-instelling",
        "minimum vrije hoogte boven de trap (ingestelde clearance)",
        m.vrije_hoogte,
        g.vrije_hoogte,
        Zin::Min,
        Some(b.vrije_hoogte),
        "hoogte van de clearance boven het neuslijnvlak (Bbl bijlage I: vrije hoogte als \
         bedoeld in NEN 2580)"
            .into(),
    );
    let (oordeel, toel) = match m.clearance_in_trapgat {
        Some(true) if vh_ok => (Oordeel::Voldoet, "clearance valt binnen het opgegeven trapgat".to_string()),
        Some(true) => (Oordeel::Melding, "clearance past in het trapgat, maar de ingestelde hoogte ligt onder tabel 4.26".to_string()),
        Some(false) => (Oordeel::VoldoetNiet, "clearance botst met de bovenvloer: valt buiten het opgegeven trapgat".to_string()),
        None => (
            Oordeel::Handmatig,
            "geen trapgat opgegeven: de trap is leidend; het minimale trapgat staat in de toets-JSON \
             (trapgat_minimaal) en de clearance in het IFC"
                .to_string(),
        ),
    };
    t.regels.push(Regel {
        id: "bbl-4.26-vrije-hoogte-trapgat".into(),
        onderwerp: "vrije hoogte boven de trap tegen de bovenvloer".into(),
        eis: format!("clearance {} mm vrij van de bovenvloer", m.vrije_hoogte),
        grenswaarde: Some(g.vrije_hoogte),
        waarde: None,
        eenheid: "mm".into(),
        oordeel,
        toelichting: toel,
        bron: format!("Bbl art. 4.26 lid 1, tabel 4.26 ({BBL_TOESTAND}); bijlage I 'vrije hoogte'"),
        bron_id: BBL_42.into(),
        regel: "r. 800-845".into(),
    });
    let a_bbl = m.aantrede_looplijn;
    let rho = if m.looplijn_afronding.is_empty() {
        "rechte trap".to_string()
    } else {
        let r: Vec<String> = m.looplijn_afronding.iter().map(|r| format!("{r:.0}")).collect();
        format!("afronding rho {} mm", r.join(" / "))
    };
    let vloeiend = if m.looplijn_grootste_knik < KNIK_VLOEIEND {
        "vloeiend".to_string()
    } else {
        format!(
            "NIET vloeiend: knik {:.1} graden (bijlage I: 'vloeiend verlopende lijn')",
            m.looplijn_grootste_knik
        )
    };
    let welke = format!(
        "klimlijn = looplijn {:.0} uit de buitenzijde, {rho}, {vloeiend}; aantrede = afstand \
         tussen opvolgende neuspunten, ontwikkeld langs de looplijn (meetwijze besluit Jochem \
         30-10-2026)",
        m.looplijn_afstand_buitenzijde
    );
    let andere = format!(
        "informatie: loodrecht op de voorkant gemeten (strikte lezing) {:.1} (A) / {:.1} (A')",
        m.info_aantrede_loodrecht, m.info_aantrede_loodrecht_eigen
    );
    t.tabel(
        "bbl-4.26-aantrede",
        "minimum aantrede ter plaatse van de klimlijn",
        a_bbl,
        g.aantrede,
        Zin::Min,
        Some(b.aantrede),
        format!("{welke}; {andere}"),
    );
    t.tabel(
        "bbl-4.26-optrede",
        "maximum hoogte van een optrede",
        m.optrede,
        g.optrede,
        Zin::Max,
        Some(b.optrede),
        format!("hoogte {:.1} / {} optreden", m.hoogte, m.optreden),
    );
    t.tabel(
        "bbl-4.26-tredevlak",
        "minimum breedte van het tredevlak, loodrecht op de voorkant",
        m.tredevlak_smalst,
        g.tredevlak,
        Zin::Min,
        None,
        format!("smalste plek (eindpunten van de volgende voorkant) + neus {:.0}", m.neus),
    );
    t.tabel(
        "bbl-4.26-tredevlak-klimlijn",
        "minimum breedte van het tredevlak ter plaatse van de klimlijn",
        a_bbl + m.neus,
        g.tredevlak_klimlijn,
        Zin::Min,
        None,
        format!("aantrede op de klimlijn + neus {:.0}; {welke}; {andere}", m.neus),
    );
    for r in t.regels.iter_mut() {
        if r.id == "bbl-4.26-aantrede" || r.id == "bbl-4.26-tredevlak-klimlijn" {
            r.bron = BRON_KLIMLIJN.into();
            r.bron_id = format!("{BBL_42}; wet/bbl__bijlage-i; {P56}; {P711}");
            r.regel = "r. 800-845; r. 403-404; r. 68; r. 550".into();
        }
    }
    t.tabel(
        "bbl-4.26-klimlijn-zijkant",
        "minimum afstand van de klimlijn tot de zijkanten",
        m.klimlijn_tot_binnenzijde.min(m.klimlijn_tot_buitenzijde),
        g.klimlijn_zijkant,
        Zin::Min,
        Some(b.klimlijn_zijkant),
        format!(
            "binnenzijde {:.1}, buitenzijde {:.1} (zijkant = rand loopbreedte; een insteek van \
             de trappaal telt niet mee)",
            m.klimlijn_tot_binnenzijde, m.klimlijn_tot_buitenzijde
        ),
    );
    let mut r = t.regels;

    // art. 4.26 lid 2
    let lid2 = !matches!(m.gebruiksfunctie, Gebruiksfunctie::Industriefunctie | Gebruiksfunctie::Overig);
    r.push(Regel {
        id: "bbl-4.26-lid2-hoogteverschil".into(),
        onderwerp: "hoogteverschil per trap".into(),
        eis: "<= 4000 mm".into(),
        grenswaarde: Some(4000.0),
        waarde: Some(rond(m.hoogte)),
        eenheid: "mm".into(),
        oordeel: if !lid2 {
            Oordeel::NietVanToepassing
        } else if m.hoogte <= 4000.0 + 1e-9 {
            Oordeel::Voldoet
        } else if m.bouwsoort == Bouwsoort::Verbouw {
            Oordeel::Melding
        } else {
            Oordeel::VoldoetNiet
        },
        toelichting: if lid2 {
            String::new()
        } else {
            "lid 2 niet aangewezen voor deze gebruiksfunctie (tabel 4.24)".into()
        },
        bron: format!("Bbl art. 4.26 lid 2 en tabel 4.24 ({BBL_TOESTAND})"),
        bron_id: BBL_42.into(),
        regel: "r. 685-787, r. 806-808".into(),
    });
    // art. 4.26a markering
    let markering = matches!(
        m.gebruiksfunctie,
        Gebruiksfunctie::Bijeenkomstfunctie | Gebruiksfunctie::Gezondheidszorgfunctie | Gebruiksfunctie::Winkelfunctie
    );
    r.push(Regel {
        id: "bbl-4.26a-markering".into(),
        onderwerp: "markering trap (50 mm, hoog contrast)".into(),
        eis: "markering bovenste/onderste trederand en zijkanten".into(),
        grenswaarde: None,
        waarde: None,
        eenheid: String::new(),
        oordeel: if markering { Oordeel::Handmatig } else { Oordeel::NietVanToepassing },
        toelichting: if markering {
            "niet gemodelleerd; bij bijeenkomstfunctie alleen subfuncties c en d (tabel 4.24)".into()
        } else {
            "niet aangewezen voor deze gebruiksfunctie (tabel 4.24)".into()
        },
        bron: format!("Bbl art. 4.26a en tabel 4.24 ({BBL_TOESTAND})"),
        bron_id: BBL_42.into(),
        regel: "r. 847-852".into(),
    });
    r.push(Regel {
        id: "bbl-4.27-bordes".into(),
        onderwerp: "aansluitende vloer bij de bovenste trede".into(),
        eis: ">= 800 x 800 mm over de breedte van de trap".into(),
        grenswaarde: Some(800.0),
        waarde: None,
        eenheid: "mm".into(),
        oordeel: Oordeel::Handmatig,
        toelichting: "buiten de trap; in het model van de verdieping nagaan".into(),
        bron: format!("Bbl art. 4.27 ({BBL_TOESTAND})"),
        bron_id: BBL_42.into(),
        regel: "r. 854-858".into(),
    });
    let helling = m.optrede / m.aantrede_looplijn;
    let leuning = m.hoogte > 1000.0 && helling > 2.0 / 3.0;
    r.push(Regel {
        id: "bbl-4.28-leuning".into(),
        onderwerp: "leuning".into(),
        eis: "bij hoogteverschil > 1 m en helling klimlijn > 2:3: leuning, bovenkant 800-1000 mm".into(),
        grenswaarde: None,
        waarde: Some(rond(helling)),
        eenheid: "optrede/aantrede".into(),
        oordeel: if leuning { Oordeel::Handmatig } else { Oordeel::NietVanToepassing },
        toelichting: if leuning {
            "leuning vereist aan ten minste een zijkant (lid 2: beide zijkanten bij de functies uit \
             tabel 4.24); niet gemodelleerd"
                .into()
        } else {
            "geen leuning vereist volgens lid 1".into()
        },
        bron: format!("Bbl art. 4.28 ({BBL_TOESTAND})"),
        bron_id: BBL_42.into(),
        regel: "r. 861-870".into(),
    });

    // advies: Wattjes en praktijk
    let stap = 2.0 * m.optrede + m.aantrede_looplijn;
    r.push(advies(
        "praktijk-stapmaat",
        "stapmaat 2 x optrede + aantrede",
        "570 - 630 mm",
        Some(stap),
        "mm",
        (570.0..=630.0).contains(&stap),
        "praktijkregel; Wattjes hoofdstuk 1 (algemene afmetingen) staat niet in het corpus, dus \
         niet tegen een bron nagekeken"
            .into(),
        "praktijkregel (bron ontbreekt in het corpus)",
        "-",
        "-",
    ));
    let verhouding = m.looplijn_afstand_buitenzijde / m.loopbreedte_min;
    r.push(advies(
        "wattjes-klimlijn-plaats",
        "plaats klimlijn vanaf de buitenboom",
        "1/3 - 2/5 van de binnenwerkse breedte",
        Some(verhouding),
        "deel van de breedte",
        (1.0 / 3.0 - 1e-9..=0.4 + 1e-9).contains(&verhouding),
        format!("looplijn {:.0} mm uit de buitenzijde", m.looplijn_afstand_buitenzijde),
        "Wattjes deel VIII h2 par. 5-6 (leerboek, niet bindend)",
        P56,
        "r. 321-322",
    ));
    r.push(advies(
        "wattjes-nestdiepte",
        "nestdiepte",
        "10 mm tot hoogstens 1/4 van de boomdikte",
        Some(m.nestdiepte),
        "mm",
        m.nestdiepte <= m.boomdikte / 4.0 + 1e-9,
        format!("boomdikte {:.0}: max {:.1}", m.boomdikte, m.boomdikte / 4.0),
        "Wattjes deel VIII h2 par. 3-4 (leerboek, niet bindend)",
        P34,
        "r. 162-164",
    ));
    r.push(advies(
        "wattjes-voorhout",
        "voorhout",
        "15 - 30 mm",
        Some(m.voorhout),
        "mm",
        (15.0..=30.0).contains(&m.voorhout),
        "loodrecht op de neuslijn; meer voorhout = bredere plank, geen constructief bezwaar".into(),
        "Wattjes deel VIII h2 par. 3-4 (leerboek, niet bindend)",
        P34,
        "r. 167-168",
    ));
    r.push(advies(
        "wattjes-achterhout",
        "achterhout (kleinste hout onder de nesten)",
        &format!(">= {} mm (invoer)", m.achterhout_min_eis),
        Some(m.achterhout_min_gemeten),
        "mm",
        m.achterhout_min_gemeten >= m.achterhout_min_eis - 1e-6,
        "Wattjes: achterhout gelijk aan voorhout bij een schone onderkant, 45 mm bij een \
         geplafonneerde onderkant"
            .into(),
        "Wattjes deel VIII h2 par. 3-4 (leerboek, niet bindend)",
        P34,
        "r. 167-172",
    ));
    let geen_waarsch = m.verdrijving_waarschuwingen.is_empty();
    r.push(advies(
        "wattjes-verdrijving-overgang",
        "overgang onverdreven - verdreven treden",
        "sprong <= verschil van de rekenkundige reeks",
        None,
        "",
        geen_waarsch,
        if geen_waarsch {
            "fuikmethode; geen sprong groter dan het reeksverschil".into()
        } else {
            m.verdrijving_waarschuwingen.join("; ")
        },
        "Wattjes deel VIII h2 par. 7-11 (leerboek, niet bindend)",
        P711,
        "r. 851-857",
    ));
    if let Some(d) = m.stootbord_tot_buitenhoek {
        let grens = m.stootborddikte + 5.0;
        r.push(advies(
            "wattjes-stootbord-hoek",
            "geen stootbord in de hoek tussen twee bomen",
            &format!("stootbord > {grens:.0} mm uit de buitenhoek"),
            Some(d),
            "mm",
            d > grens,
            "een stootbord juist in de hoek geeft scherpe inspringende hoeken".into(),
            "Wattjes deel VIII h2 par. 7-11 (leerboek, niet bindend)",
            P711,
            "r. 79-81",
        ));
    }
    if let Some(dl) = m.doorlopend {
        r.push(advies(
            "wattjes-hoekverbinding",
            "hoekverbinding buitenboom (koud): welk deel loopt door",
            "het hogere deel loopt door; het lagere sluit er met een vlakke borst op aan",
            None,
            "",
            dl == Doorlopend::Hoger,
            "Wattjes beschrijft een lip (uitkeping ter diepte van de nesten) in de hogere boom; de \
             rekenkern maakt de verbinding koud zonder lip (besluit Jochem 30-10-2026)"
                .into(),
            "Wattjes deel VIII h2 par. 7-11 (leerboek, niet bindend)",
            P711,
            "r. 141-145",
        ));
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meting() -> Meting {
        Meting {
            gebruiksfunctie: Gebruiksfunctie::Woonfunctie,
            bouwsoort: Bouwsoort::Nieuwbouw,
            hoogte: 3000.0,
            optreden: 16,
            optrede: 187.5,
            aantrede_looplijn: 230.0,
            info_aantrede_loodrecht: 225.0,
            info_aantrede_loodrecht_eigen: 226.0,
            klimlijn_band: Some([300.0, 550.0]),
            neus: 20.0,
            tredevlak_smalst: 60.0,
            loopbreedte_min: 850.0,
            klimlijn_tot_binnenzijde: 550.0,
            klimlijn_tot_buitenzijde: 300.0,
            looplijn_afstand_buitenzijde: 300.0,
            looplijn_afronding: vec![550.0],
            looplijn_grootste_knik: 0.25,
            vrije_hoogte: 2300.0,
            clearance_in_trapgat: None,
            boomdikte: 40.0,
            nestdiepte: 10.0,
            voorhout: 25.0,
            achterhout_min_eis: 20.0,
            achterhout_min_gemeten: 25.0,
            verdrijving_waarschuwingen: vec![],
            stootbord_tot_buitenhoek: None,
            stootborddikte: 18.0,
            doorlopend: None,
        }
    }

    fn oordeel(m: &Meting, id: &str) -> Oordeel {
        toets(m).into_iter().find(|r| r.id == id).expect(id).oordeel
    }

    /// (regel, veld zetten op waarde) -> oordeel; grens net goed en net fout.
    fn grens(id: &str, zet: fn(&mut Meting, f64), goed: f64, fout: f64) {
        for (f, bouw) in [
            (Gebruiksfunctie::Woonfunctie, Bouwsoort::Nieuwbouw),
            (Gebruiksfunctie::Woonfunctie, Bouwsoort::Verbouw),
        ] {
            let mut m = meting();
            m.gebruiksfunctie = f;
            m.bouwsoort = bouw;
            zet(&mut m, goed);
            assert_eq!(oordeel(&m, id), Oordeel::Voldoet, "{id} net goed {goed}");
            zet(&mut m, fout);
            let verwacht = if bouw == Bouwsoort::Verbouw { Oordeel::Melding } else { Oordeel::VoldoetNiet };
            assert_eq!(oordeel(&m, id), verwacht, "{id} net fout {fout} ({bouw:?})");
        }
    }

    #[test]
    fn grenzen_woonfunctie() {
        grens("bbl-4.26-breedte", |m, v| m.loopbreedte_min = v, 800.0, 799.9);
        grens("bbl-4.26-vrije-hoogte-instelling", |m, v| m.vrije_hoogte = v, 2300.0, 2299.9);
        grens("bbl-4.26-aantrede", |m, v| m.aantrede_looplijn = v, 220.0, 219.9);
        grens("bbl-4.26-optrede", |m, v| m.optrede = v, 188.0, 188.1);
        grens("bbl-4.26-tredevlak", |m, v| m.tredevlak_smalst = v, 50.0, 49.9);
        // tredevlak klimlijn = aantrede + neus 20
        grens("bbl-4.26-tredevlak-klimlijn", |m, v| m.aantrede_looplijn = v - 20.0, 230.0, 229.9);
        grens("bbl-4.26-klimlijn-zijkant", |m, v| m.klimlijn_tot_buitenzijde = v, 300.0, 299.9);
    }

    #[test]
    fn grenzen_andere_gebruiksfunctie() {
        let mut m = meting();
        m.gebruiksfunctie = Gebruiksfunctie::Kantoorfunctie;
        m.aantrede_looplijn = 185.0;
        m.neus = 45.0;
        assert_eq!(oordeel(&m, "bbl-4.26-aantrede"), Oordeel::Voldoet);
        m.aantrede_looplijn = 184.9;
        assert_eq!(oordeel(&m, "bbl-4.26-aantrede"), Oordeel::VoldoetNiet);
        m.optrede = 210.0;
        assert_eq!(oordeel(&m, "bbl-4.26-optrede"), Oordeel::Voldoet);
        m.optrede = 210.1;
        assert_eq!(oordeel(&m, "bbl-4.26-optrede"), Oordeel::VoldoetNiet);
        m.vrije_hoogte = 2100.0;
        assert_eq!(oordeel(&m, "bbl-4.26-vrije-hoogte-instelling"), Oordeel::Voldoet);
        m.vrije_hoogte = 2099.9;
        assert_eq!(oordeel(&m, "bbl-4.26-vrije-hoogte-instelling"), Oordeel::VoldoetNiet);
    }

    /// Oordeel op meting B (langs de looplijn); A/A' veranderen het oordeel niet.
    #[test]
    fn oordeel_op_looplijn_niet_op_loodrecht() {
        let mut m = meting();
        m.aantrede_looplijn = 221.0;
        m.info_aantrede_loodrecht = 150.0;
        m.info_aantrede_loodrecht_eigen = 150.0;
        assert_eq!(oordeel(&m, "bbl-4.26-aantrede"), Oordeel::Voldoet);
    }

    #[test]
    fn hoogteverschil_lid2() {
        let mut m = meting();
        m.hoogte = 4000.0;
        assert_eq!(oordeel(&m, "bbl-4.26-lid2-hoogteverschil"), Oordeel::Voldoet);
        m.hoogte = 4000.1;
        assert_eq!(oordeel(&m, "bbl-4.26-lid2-hoogteverschil"), Oordeel::VoldoetNiet);
        m.gebruiksfunctie = Gebruiksfunctie::Industriefunctie;
        assert_eq!(oordeel(&m, "bbl-4.26-lid2-hoogteverschil"), Oordeel::NietVanToepassing);
    }

    #[test]
    fn verbouw_meldt_ondergrens_bestaande_bouw() {
        let mut m = meting();
        m.bouwsoort = Bouwsoort::Verbouw;
        m.optrede = 221.0;
        let r = toets(&m).into_iter().find(|r| r.id == "bbl-4.26-optrede").unwrap();
        assert_eq!(r.oordeel, Oordeel::Melding);
        assert!(r.toelichting.contains("voldoet NIET aan de ondergrens bestaande bouw"));
        m.optrede = 220.0;
        let r = toets(&m).into_iter().find(|r| r.id == "bbl-4.26-optrede").unwrap();
        assert!(r.toelichting.contains("voldoet aan de ondergrens bestaande bouw"));
    }

    #[test]
    fn leuning_bij_steile_trap() {
        let mut m = meting();
        m.optrede = 188.0;
        m.aantrede_looplijn = 230.0; // 0,82 > 2:3
        assert_eq!(oordeel(&m, "bbl-4.28-leuning"), Oordeel::Handmatig);
        m.hoogte = 1000.0;
        assert_eq!(oordeel(&m, "bbl-4.28-leuning"), Oordeel::NietVanToepassing);
    }
}
