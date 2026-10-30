//! Van invoer naar ontwerp: verdrijving, onderdelen, bomen, palen, clearance,
//! toetsing en het trapmodel voor de IFC-schrijver.

use serde::Serialize;

use crate::boom::{
    boomdelen, boven_en_onder, deel_voetafdruk, nest_voetafdruk, paal_voetafdruk, profiel_oppervlak,
    El, Paalmaat,
};
use crate::clearance::clearance_volume;
use crate::invoer::{
    Bouwsoort, Breedtemaat, Doorlopend, Invoer, Kant, Looplijnhoek, Plattegrond, Stootbordsoort,
    Vak,
};
use crate::model::{BoomdeelMetNesten, Paal, Prisma, Trap, Trapmodel, Waarde, P2};
use crate::onderdelen::{achterpunten, nesten_in_vlak, treden_en_stootborden, Afmetingen};
use crate::toets::{tabel_4_26, toets, Meting, Regel};
use crate::verdrijving::{
    aantreden_loodrecht, aantreden_op_klimlijn, kies_vakken, looplijn_tot_zijkanten,
    smalste_tredevlak, verdeel, Contour, Loodrecht, Verdeling, Voorkant, Waar,
};
use crate::vlak::{afstand_tot_segment, eenheid, plus, punt_in_veelhoek, verschuif, Polylijn};
use crate::Fout;

pub const OPDRACHT: &str = "O-2026-10-30-03";
pub const AGENT: &str = "open-bouwlab-trap";

/// Informatie naast het oordeel: aantrede loodrecht op de voorkant (A). Het
/// oordeel volgt de aantrede langs de looplijn (meting B, besluit Jochem
/// 30-10-2026, planner msg_496305ebd730).
pub const LOODRECHT: Loodrecht = Loodrecht::VanafVolgende;

/// Stap waarmee de band van mogelijke klimlijnen wordt doorzocht (mm).
pub const KLIMLIJN_STAP: f64 = 5.0;

/// Resultaat: het trapmodel (voor IFC) en de toetsing (voor de toets-JSON).
pub struct Ontwerp {
    pub model: Trapmodel,
    pub rapport: Rapport,
}

#[derive(Debug, Clone, Serialize)]
pub struct Rapport {
    pub trap: String,
    pub opdracht: &'static str,
    pub disclaimer: &'static str,
    pub resultaat: Resultaat,
    pub regels: Vec<Regel>,
    /// Minimaal trapgat: plattegrond van de clearance waar die boven de
    /// onderkant van de bovenvloer uitkomt.
    pub trapgat_minimaal: Vec<P2>,
    pub waarschuwingen: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Resultaat {
    pub hoogte: f64,
    pub optreden: u32,
    pub aantreden: u32,
    pub optrede: f64,
    pub aantrede_looplijn: f64,
    /// Informatie: op de looplijn loodrecht op de voorkant, meting A.
    pub aantreden_loodrecht: Vec<f64>,
    /// Informatie: idem, meting A' (vanaf de eigen voorkant).
    pub aantreden_loodrecht_vanaf_eigen_voorkant: Vec<f64>,
    pub aantreden_binnenzijde: Vec<f64>,
    pub aantreden_buitenzijde: Vec<f64>,
    pub looplijn_lengte: f64,
    pub vakken: Vec<Vak>,
    pub voorkanten: Vec<Voorkant>,
    pub loopbreedte: f64,
    pub vrije_hoogte: f64,
    pub treden: usize,
    pub stootborden: usize,
    pub boomdelen: Vec<Boomdeelmaat>,
    pub palen: Vec<Paalmaatuit>,
    pub meting: Meting,
    /// Informatie: kleinste aantrede (langs de lijn) per klimlijn in de band:
    /// (afstand uit de buitenzijde, aantrede).
    pub klimlijnen: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Boomdeelmaat {
    pub naam: String,
    /// Lengte langs het deel (s-bereik van het profiel).
    pub lengte: f64,
    pub z_min: f64,
    pub z_max: f64,
    pub nesten: usize,
    pub doorlopend: bool,
    /// Profiel x dikte min de nesten (controle tegen het IFC-volume).
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Paalmaatuit {
    pub naam: String,
    pub breedte: f64,
    pub diepte: f64,
    pub onderkant: f64,
    pub bovenkant: f64,
    pub nesten: usize,
}

pub const DISCLAIMER: &str = "Coordinatiemodel, indicatief ontwerp. Definitieve uitwerking, \
maatvoering, inmeten ter plaatse en constructieve verantwoording door de trappenmaker of \
uitvoerder. Niet zonder controle gebruiken voor productie. Opgesteld door Kolthof Bouwadvies.";

struct Plat {
    /// Randen van de loopbreedte tot de voorkant van het welstuk.
    binnen: Polylijn,
    buiten: Polylijn,
    /// Idem, doorgetrokken tot de trapgatrand (achterkant welstuk).
    binnenvlak: Polylijn,
    buitenvlak: Polylijn,
    kant_binnen: Kant,
}

fn draai(d: P2, kant: Kant) -> P2 {
    match kant {
        Kant::Links => [-d[1], d[0]],
        Kant::Rechts => [d[1], -d[0]],
    }
}

fn verleng_eind(pl: &Polylijn, lengte: f64) -> Result<Polylijn, Fout> {
    let n = pl.punten.len();
    let d = eenheid([
        pl.punten[n - 1][0] - pl.punten[n - 2][0],
        pl.punten[n - 1][1] - pl.punten[n - 2][1],
    ])?;
    let mut p = pl.punten.clone();
    p[n - 1] = plus(p[n - 1], d, lengte);
    Polylijn::nieuw(p)
}

fn plattegrond(inv: &Invoer) -> Result<Plat, Fout> {
    let (binnen, buiten, kant, welstuk) = match &inv.plattegrond {
        Plattegrond::Armen(a) => {
            if a.armlengtes.is_empty() || a.armlengtes.len() > 3 {
                return Err(Fout::Invoer(
                    "armen: 1 (recht), 2 (kwartdraai) of 3 (halfdraai) armlengtes".into(),
                ));
            }
            let t = inv.boom.dikte;
            let n = a.armlengtes.len();
            let (breedte, start, lengtes) = match a.breedte_is {
                Breedtemaat::Loopbreedte => (a.breedte, a.start, a.armlengtes.clone()),
                Breedtemaat::Buitenwerks => {
                    let d = eenheid(a.richting)?;
                    let naar_binnen = draai(d, a.binnenzijde);
                    let l: Vec<f64> = (0..n)
                        .map(|i| {
                            let hoeken = usize::from(i > 0) + usize::from(i + 1 < n);
                            a.armlengtes[i] - t * hoeken as f64
                        })
                        .collect();
                    (a.breedte - 2.0 * t, plus(a.start, naar_binnen, t), l)
                }
            };
            if breedte <= 0.0 || lengtes.iter().any(|&l| l <= 0.0) {
                return Err(Fout::Invoer("breedte en armlengtes moeten positief zijn".into()));
            }
            let mut d = eenheid(a.richting)?;
            let mut pts = vec![start];
            for l in &lengtes {
                let p = plus(pts[pts.len() - 1], d, *l);
                pts.push(p);
                d = draai(d, a.binnenzijde);
            }
            let buiten = Polylijn::nieuw(pts)?;
            let binnen = verschuif(&buiten, breedte, a.binnenzijde)?;
            (binnen, buiten, a.binnenzijde, a.welstuk)
        }
        Plattegrond::Contour(c) => (
            Polylijn::nieuw(c.binnenzijde.clone())?,
            Polylijn::nieuw(c.buitenzijde.clone())?,
            c.kant_binnen,
            c.welstuk,
        ),
    };
    if welstuk <= 0.0 {
        return Err(Fout::Invoer("welstuk moet een diepte hebben".into()));
    }
    Ok(Plat {
        binnenvlak: verleng_eind(&binnen, welstuk)?,
        buitenvlak: verleng_eind(&buiten, welstuk)?,
        binnen,
        buiten,
        kant_binnen: kant,
    })
}

fn strook(binnenvlak: &Polylijn, buitenvlak: &Polylijn, kant_binnen: Kant, nest: f64) -> Result<(Vec<P2>, (P2, P2)), Fout> {
    let b = if nest > 0.0 { verschuif(binnenvlak, nest, kant_binnen)? } else { binnenvlak.clone() };
    let u = if nest > 0.0 { verschuif(buitenvlak, nest, kant_binnen.andere())? } else { buitenvlak.clone() };
    let eind = (b.punten[b.punten.len() - 1], u.punten[u.punten.len() - 1]);
    let mut p = b.punten.clone();
    p.extend(u.punten.iter().rev());
    Ok((p, eind))
}

/// Aantal optreden: kleinste aantal waarbij de optrede aan tabel 4.26 voldoet,
/// daarboven tot vier extra het aantal met de stapmaat het dichtst bij 630.
fn kies_optreden(inv: &Invoer, hoogte: f64, looplijn: f64) -> u32 {
    let g = tabel_4_26(inv.gebruiksfunctie);
    let n_min = ((hoogte / g.optrede) - 1e-9).ceil().max(2.0) as u32;
    (n_min..=n_min + 4)
        .filter(|&n| looplijn / f64::from(n - 1) >= g.aantrede - 1e-9 || n == n_min)
        .min_by(|&a, &b| {
            let stap = |n: u32| (2.0 * hoogte / f64::from(n) + looplijn / f64::from(n - 1) - 630.0).abs();
            stap(a).total_cmp(&stap(b))
        })
        .unwrap_or(n_min)
}

pub fn ontwerp(inv: &Invoer) -> Result<Ontwerp, Fout> {
    let pl = plattegrond(inv)?;
    let hoogte = inv.niveaus.boven - inv.niveaus.onder;
    if hoogte <= 0.0 {
        return Err(Fout::Invoer("niveau boven moet boven niveau onder liggen".into()));
    }
    let c = Contour::nieuw(
        pl.binnen.clone(),
        pl.buiten.clone(),
        pl.kant_binnen,
        inv.looplijn.afstand_buitenzijde,
        inv.looplijn.hoek,
        inv.looplijn.straal,
    )?;
    let n_opt = match inv.optreden {
        Some(n) if n >= 2 => n,
        Some(_) => return Err(Fout::Invoer("minstens twee optreden".into())),
        None => kies_optreden(inv, hoogte, c.looplijn.lengte()),
    };
    let optrede = hoogte / f64::from(n_opt);
    let aantreden = (n_opt - 1) as usize;
    let vakken = match &inv.verdrijving.vakken {
        Some(v) => v.clone(),
        None if c.binnenhoeken().is_empty() => Vec::new(),
        None => kies_vakken(&c, aantreden)?,
    };
    let v = verdeel(&c, aantreden, &vakken)?;

    let z_basis = inv.niveaus.onder;
    let z_top = inv.niveaus.boven;
    let nest = inv.boom.nestdiepte;
    let (gebied, eindlijn) = strook(&pl.binnenvlak, &pl.buitenvlak, pl.kant_binnen, 0.0)?;
    let met_borden = inv.stootbord.soort == Stootbordsoort::Dicht;
    let afm = Afmetingen {
        z_basis,
        optrede,
        trede_dikte: inv.trede.dikte,
        neus: inv.trede.neus,
        stootbord: if met_borden { inv.stootbord.dikte } else { 0.0 },
        met_stootborden: met_borden,
    };
    let (treden, borden) = treden_en_stootborden(
        &v,
        &gebied,
        [&pl.binnenvlak, &pl.buitenvlak],
        nest,
        eindlijn,
        &afm,
    )?;
    let alle: Vec<Prisma> = treden.iter().chain(&borden).cloned().collect();
    let steunend: Vec<Prisma> = treden[..treden.len() - 1]
        .iter()
        .chain(borden.iter().skip(1))
        .cloned()
        .collect();
    let z_neus: Vec<f64> = (0..v.voorkanten.len())
        .map(|k| z_basis + (k as f64 + 1.0) * optrede)
        .collect();

    let paalmaat = inv.trappaal.as_ref().map(|p| Paalmaat {
        langs_in: p.langs_in,
        langs_uit: p.langs_uit,
        insteek_in: p.insteek_in,
        insteek_uit: p.insteek_uit,
    });
    let doorlopend = inv.hoekverbinding.doorlopend;
    let mut bomen = Vec::new();
    let mut boommaten = Vec::new();
    let mut achterhout_min = f64::INFINITY;
    for (naam, vlak, kant, binnen) in [
        ("binnenboom", &pl.binnenvlak, pl.kant_binnen, true),
        ("buitenboom", &pl.buitenvlak, pl.kant_binnen.andere(), false),
    ] {
        let u_neus: Vec<f64> = v
            .voorkanten
            .iter()
            .map(|x| if binnen { x.u_binnen } else { x.w_buiten })
            .collect();
        let (boven, onder) = boven_en_onder(
            &u_neus,
            &z_neus,
            inv.boom.voorhout,
            inv.boom.plankbreedte,
            0.0,
            vlak.lengte(),
        )?;
        let (ua, za) = achterpunten(&steunend, vlak, nest);
        for (u, z) in ua.iter().zip(&za) {
            // getekend: negatief als de achter-onderkant van het nest onder de
            // onderkant van de boom ligt (nest steekt uit de boom)
            let a = onder.afstand([*u, *z]);
            achterhout_min = achterhout_min.min(if *z < onder.z(*u) { -a } else { a });
        }
        let delen = boomdelen(
            naam,
            &vlak.punten,
            kant,
            inv.boom.dikte,
            &boven,
            &onder,
            z_basis,
            z_top,
            if binnen { paalmaat } else { None },
            doorlopend,
        )?;
        let mut met_nesten = Vec::new();
        for (k, d) in delen.into_iter().enumerate() {
            let nesten = nesten_in_vlak(&alle, &nest_voetafdruk(&d), 1e-3);
            let s: Vec<f64> = d.profiel.iter().map(|e| e.p0()[0]).collect();
            let z: Vec<f64> = d.profiel.iter().map(|e| e.p0()[1]).collect();
            let seg = vlak.stations()[k + 1] - vlak.stations()[k];
            let lengte = s.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                - s.iter().copied().fold(f64::INFINITY, f64::min);
            boommaten.push(Boomdeelmaat {
                naam: d.naam.clone(),
                lengte,
                z_min: z.iter().copied().fold(f64::INFINITY, f64::min),
                z_max: z.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                nesten: nesten.len(),
                doorlopend: !binnen && lengte > seg + 1e-6,
                volume: profiel_oppervlak(&d.profiel.iter().map(El::van_model).collect::<Vec<_>>()).abs()
                    * d.dikte
                    - nesten_in_vlak(&nesten, &deel_voetafdruk(&d), 1e-3)
                        .iter()
                        .map(crate::onderdelen::volume)
                        .sum::<f64>(),
            });
            met_nesten.push(BoomdeelMetNesten { deel: d, nesten });
        }
        bomen.push(crate::model::Boom {
            naam: naam.into(),
            delen: met_nesten,
        });
    }

    // trappalen in de binnenhoeken
    let mut palen = Vec::new();
    let mut paalmaten = Vec::new();
    let bst = pl.binnenvlak.stations().to_vec();
    for k in 1..pl.binnenvlak.punten.len() - 1 {
        let (Some(m), Some(p_inv)) = (paalmaat, inv.trappaal.as_ref()) else {
            return Err(Fout::Invoer("binnenhoek zonder trappaal: geef 'trappaal' op".into()));
        };
        let voet = paal_voetafdruk(&pl.binnenvlak.punten, k, pl.kant_binnen, m);
        let delen = &bomen[0].delen;
        let z_eind = |deel: usize, s: f64| {
            delen[deel]
                .deel
                .profiel
                .iter()
                .filter(|e| (e.p0()[0] - s).abs() < 1e-6)
                .map(|e| e.p0()[1])
                .fold(f64::NEG_INFINITY, f64::max)
        };
        let z_boom = z_eind(k - 1, bst[k] - m.langs_in - bst[k - 1]).max(z_eind(k, m.langs_uit));
        let z1 = z_boom + p_inv.boven_boom;
        let naam = if pl.binnenvlak.punten.len() > 3 {
            format!("trappaal binnenhoek {k}")
        } else {
            "trappaal binnenhoek".to_string()
        };
        let paal = Prisma {
            naam: naam.clone(),
            soort: "PAAL".into(),
            veelhoek: voet.clone(),
            z0: z_basis,
            z1,
        };
        let nesten = nesten_in_vlak(&alle, &voet, 1e-3);
        let breedte = m.langs_uit + m.insteek_in;
        let diepte = m.langs_in + m.insteek_uit;
        paalmaten.push(Paalmaatuit {
            naam: naam.clone(),
            breedte,
            diepte,
            onderkant: z_basis,
            bovenkant: z1,
            nesten: nesten.len(),
        });
        palen.push(Paal {
            paal,
            nesten,
            eigenschappen: vec![
                ("Materiaal".into(), Waarde::Label(p_inv.materiaal.clone())),
                ("Breedte".into(), Waarde::Real(breedte)),
                ("Diepte".into(), Waarde::Real(diepte)),
                ("Hoogte".into(), Waarde::Real(r1(z1 - z_basis))),
                ("Onderkant".into(), Waarde::Real(z_basis)),
                ("Bovenkant".into(), Waarde::Real(r1(z1))),
                ("Nestdiepte".into(), Waarde::Real(nest)),
            ],
        });
    }

    // clearance
    let g = tabel_4_26(inv.gebruiksfunctie);
    let vrije_hoogte = inv.clearance.vrije_hoogte.unwrap_or(g.vrije_hoogte);
    let mut neuzen: Vec<(P2, P2, f64)> = v
        .voorkanten
        .iter()
        .enumerate()
        .map(|(k, x)| (x.p_binnen, x.p_buiten, z_neus[k]))
        .collect();
    neuzen.push((
        pl.binnenvlak.punten[pl.binnenvlak.punten.len() - 1],
        pl.buitenvlak.punten[pl.buitenvlak.punten.len() - 1],
        z_top,
    ));
    let vol = clearance_volume(&neuzen, &pl.binnenvlak, &pl.buitenvlak, vrije_hoogte, "clearance trap")?;
    let mut net = vol.net;
    net.eigenschappen = clearance_eigenschappen(inv, vrije_hoogte, z_basis + optrede, z_top);

    let trapgat_min = trapgat_minimaal(
        &neuzen,
        &pl.binnenvlak,
        &pl.buitenvlak,
        vrije_hoogte,
        inv.niveaus.boven - inv.niveaus.vloerdikte_boven,
    );
    let in_trapgat = inv.trapgat.as_ref().map(|gat| {
        trapgat_min
            .iter()
            .all(|&p| punt_in_veelhoek(p, gat) || op_rand(p, gat, 1e-6))
    });

    // metingen en toetsing
    let loodrecht = aantreden_loodrecht(&v, LOODRECHT)?;
    let loodrecht_eigen = aantreden_loodrecht(&v, Loodrecht::VanafEigen)?;
    let ontwerp_a = loodrecht.iter().copied().fold(f64::INFINITY, f64::min);
    let (kl_binnen, kl_buiten) = looplijn_tot_zijkanten(&c);
    let loopbreedte = loopbreedte_min(&pl.binnen, &pl.buiten);
    // informatie: klimlijnen in de band 300 .. breedte - 300, kleinste aantrede
    // langs elke lijn (het oordeel volgt de ingestelde looplijn, besluit Jochem)
    let band = (loopbreedte >= 2.0 * g.klimlijn_zijkant)
        .then_some([g.klimlijn_zijkant, loopbreedte - g.klimlijn_zijkant]);
    let mut klimlijnen: Vec<[f64; 2]> = Vec::new();
    if let Some([lo, hi]) = band {
        let n = ((hi - lo) / KLIMLIJN_STAP).ceil().max(1.0) as usize;
        for i in 0..=n {
            let d = lo + (hi - lo) * i as f64 / n as f64;
            if let Ok(k) = aantreden_op_klimlijn(&c, &v, d, LOODRECHT) {
                klimlijnen.push([r1(d), r3(k.langs_min)]);
            }
        }
    }
    let meting = Meting {
        gebruiksfunctie: inv.gebruiksfunctie,
        bouwsoort: inv.bouwsoort,
        hoogte,
        optreden: n_opt,
        optrede,
        aantrede_looplijn: v.aantrede_loop,
        info_aantrede_loodrecht: ontwerp_a,
        info_aantrede_loodrecht_eigen: loodrecht_eigen.iter().copied().fold(f64::INFINITY, f64::min),
        klimlijn_tot_binnenzijde: kl_binnen,
        klimlijn_tot_buitenzijde: kl_buiten,
        klimlijn_band: band,
        neus: inv.trede.neus,
        tredevlak_smalst: smalste_tredevlak(&v, inv.trede.neus)?,
        loopbreedte_min: loopbreedte,
        looplijn_afstand_buitenzijde: inv.looplijn.afstand_buitenzijde,
        looplijn_afronding: c.afronding.iter().map(|&r| r3(r)).collect(),
        looplijn_grootste_knik: c.looplijn.grootste_knik(),
        vrije_hoogte,
        clearance_in_trapgat: in_trapgat,
        boomdikte: inv.boom.dikte,
        nestdiepte: nest,
        voorhout: inv.boom.voorhout,
        achterhout_min_eis: inv.boom.achterhout_min,
        achterhout_min_gemeten: achterhout_min,
        verdrijving_waarschuwingen: v.waarschuwingen.clone(),
        stootbord_tot_buitenhoek: stootbord_tot_buitenhoek(&v, &c, &afm),
        stootborddikte: afm.stootbord,
        doorlopend: (pl.buiten.punten.len() > 2).then_some(doorlopend),
    };
    let regels = toets(&meting);
    let mut waarschuwingen = v.waarschuwingen.clone();
    if achterhout_min < 0.0 {
        waarschuwingen.push(format!(
            "een nest steekt {:.1} mm onder de boom uit: plank {} te smal of trede rust niet op de boom",
            -achterhout_min, inv.boom.plankbreedte
        ));
    } else if achterhout_min < inv.boom.achterhout_min {
        waarschuwingen.push(format!(
            "achterhout {achterhout_min:.1} mm < {} mm: plank {} te smal",
            inv.boom.achterhout_min, inv.boom.plankbreedte
        ));
    }
    if paalmaat.is_some_and(|m| m.insteek_in > 0.0 || m.insteek_uit > 0.0) {
        waarschuwingen.push(
            "trappaal steekt in de loopbreedte: de clearance en de klimlijntoets houden daar \
             geen rekening mee"
                .into(),
        );
    }

    let vorm = match pl.binnen.punten.len() {
        2 => "STRAIGHT_RUN_STAIR",
        3 => "QUARTER_WINDING_STAIR",
        _ => "HALF_WINDING_STAIR",
    };
    let steek = if v.voorkanten.iter().any(|x| x.verdreven) || pl.binnen.punten.len() > 2 {
        "WINDER"
    } else {
        "STRAIGHT"
    };
    let a = v.aantrede_loop;
    let walking_offset = loopbreedte - inv.looplijn.afstand_buitenzijde;
    let pset_trap = vec![
        ("NumberOfRiser".into(), Waarde::Int(i64::from(n_opt))),
        ("NumberOfTreads".into(), Waarde::Int(aantreden as i64)),
        ("RiserHeight".into(), Waarde::Real(r3(optrede))),
        ("TreadLength".into(), Waarde::Real(r3(a))),
        ("NosingLength".into(), Waarde::Real(inv.trede.neus)),
        ("WalkingLineOffset".into(), Waarde::Real(r3(walking_offset))),
        ("TreadLengthAtOffset".into(), Waarde::Real(r3(a))),
        (
            "TreadLengthAtInnerSide".into(),
            Waarde::Real(r1(v.aantreden(Waar::Binnen).iter().copied().fold(f64::INFINITY, f64::min))),
        ),
        ("IsExternal".into(), Waarde::Bool(false)),
    ];
    let pset_extra = uitgangspunten(inv, &v, &pl, paalmaat);
    let model = Trapmodel {
        bron: format!("trap-core {} ({OPDRACHT})", env!("CARGO_PKG_VERSION")),
        projectnaam: inv.project.naam.clone(),
        bouwlaag: inv.project.bouwlaag.clone(),
        z_bouwlaag: inv.project.z_bouwlaag.unwrap_or(z_basis),
        trap: Trap {
            naam: inv.naam.clone(),
            vorm: vorm.into(),
            steek: steek.into(),
            optreden: n_opt,
            aantreden: aantreden as u32,
            optrede,
            aantrede: a,
        },
        pset_trap,
        pset_extra,
        treden: treden.clone(),
        borden: borden.clone(),
        bomen,
        palen,
        clearance: Some(net),
    };
    let rapport = Rapport {
        trap: inv.naam.clone(),
        opdracht: OPDRACHT,
        disclaimer: DISCLAIMER,
        resultaat: Resultaat {
            hoogte,
            optreden: n_opt,
            aantreden: aantreden as u32,
            optrede,
            aantrede_looplijn: a,
            aantreden_loodrecht: loodrecht,
            aantreden_loodrecht_vanaf_eigen_voorkant: loodrecht_eigen,
            aantreden_binnenzijde: v.aantreden(Waar::Binnen),
            aantreden_buitenzijde: v.aantreden(Waar::Buiten),
            looplijn_lengte: c.looplijn.lengte(),
            vakken: v.vakken.clone(),
            voorkanten: v.voorkanten.clone(),
            loopbreedte,
            vrije_hoogte,
            treden: treden.len(),
            stootborden: borden.len(),
            boomdelen: boommaten,
            palen: paalmaten,
            meting,
            klimlijnen,
        },
        regels,
        trapgat_minimaal: trapgat_min,
        waarschuwingen,
    };
    Ok(Ontwerp { model, rapport })
}

fn r1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}
fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn op_rand(p: P2, v: &[P2], tol: f64) -> bool {
    (0..v.len()).any(|i| afstand_tot_segment(p, v[i], v[(i + 1) % v.len()]) <= tol)
}

/// Kleinste afstand tussen de randen van de loopbreedte (over de hoekpunten).
fn loopbreedte_min(binnen: &Polylijn, buiten: &Polylijn) -> f64 {
    let a = binnen.punten.iter().map(|&p| buiten.projecteer(p).1);
    let b = buiten.punten.iter().map(|&p| binnen.projecteer(p).1);
    // hoekpunten van de buitenzijde liggen in een draai verder weg dan de breedte
    a.chain(b.filter(|_| buiten.punten.len() == 2)).fold(f64::INFINITY, f64::min)
}

/// Afstand van het dichtstbijzijnde stootbord(voorvlak) tot een buitenhoek,
/// langs de buitenzijde.
fn stootbord_tot_buitenhoek(v: &Verdeling, c: &Contour, a: &Afmetingen) -> Option<f64> {
    let hoeken = c.buiten.knikken();
    if hoeken.is_empty() || !a.met_stootborden {
        return None;
    }
    let mut min = f64::INFINITY;
    for vk in &v.voorkanten[1..v.voorkanten.len() - 1] {
        // stootbord ligt `neus` .. `neus + dikte` achter de voorkant (langs de zijde benaderd)
        for &h in hoeken {
            let d0 = vk.w_buiten + a.neus - h;
            let d1 = d0 + a.stootbord;
            let d = if d0 <= 0.0 && d1 >= 0.0 { 0.0 } else { d0.abs().min(d1.abs()) };
            min = min.min(d);
        }
    }
    Some(min)
}

/// Plattegrond waar de bovenkant van de clearance boven `z_snede` uitkomt.
fn trapgat_minimaal(
    neuzen: &[(P2, P2, f64)],
    binnenvlak: &Polylijn,
    buitenvlak: &Polylijn,
    hoogte: f64,
    z_snede: f64,
) -> Vec<P2> {
    let zijde = |vlak: &Polylijn, punten: Vec<P2>| -> (f64, Vec<P2>) {
        let st: Vec<f64> = punten.iter().map(|&p| vlak.projecteer(p).0).collect();
        let zs: Vec<f64> = neuzen.iter().map(|n| n.2 + hoogte).collect();
        let mut s_snede = st[0];
        if zs[0] < z_snede {
            s_snede = st[st.len() - 1];
            for k in 0..st.len() - 1 {
                if zs[k] < z_snede && zs[k + 1] >= z_snede {
                    s_snede = st[k] + (z_snede - zs[k]) / (zs[k + 1] - zs[k]) * (st[k + 1] - st[k]);
                    break;
                }
            }
        }
        let mut pts = vec![vlak.punt_op(s_snede)];
        for (i, &s) in vlak.stations().iter().enumerate() {
            if s > s_snede + 1e-6 {
                pts.push(vlak.punten[i]);
            }
        }
        (s_snede, pts)
    };
    let (_, b) = zijde(binnenvlak, neuzen.iter().map(|n| n.0).collect());
    let (_, u) = zijde(buitenvlak, neuzen.iter().map(|n| n.1).collect());
    let mut uit = b;
    uit.extend(u.into_iter().rev());
    uit
}

fn clearance_eigenschappen(inv: &Invoer, h: f64, z_begin: f64, z_eind: f64) -> Vec<(String, Waarde)> {
    let functie = if inv.gebruiksfunctie == crate::invoer::Gebruiksfunctie::Woonfunctie {
        "woonfunctie, reguliere trap"
    } else {
        "andere gebruiksfunctie, reguliere trap"
    };
    let verbouw = if inv.bouwsoort == Bouwsoort::Verbouw {
        "; verbouw: art. 5.4/5.5 rechtens verkregen niveau, ondergrens art. 3.20 tabel 3.20 1,9 m"
    } else {
        ""
    };
    let l = |s: String| Waarde::Label(s);
    vec![
        ("VrijeHoogte".into(), Waarde::Real(h)),
        ("Vorm".into(), l(format!("bovenvlak {h:.0} boven het neuslijnvlak"))),
        (
            "Meetwijze".into(),
            l("verticaal boven het neuslijnvlak: per trede de neuslijn (voorkant trede, rand \
               loopbreedte tot rand loopbreedte) op bovenkant trede, ertussen een regelvlak; \
               vrije hoogte als bedoeld in NEN 2580 via Bbl bijlage I"
                .into()),
        ),
        (
            "Bron".into(),
            l(format!(
                "Bbl art. 4.26 lid 1, tabel 4.26 ({functie}; geldend, toestand 2026-09-24){verbouw}"
            )),
        ),
        (
            "Begin".into(),
            l(format!(
                "verticaal in het vlak van de voorkant van trede 01; onderkant op de neus van \
                 trede 01 ({z_begin:+.1}), bovenkant {:+.1}",
                z_begin + h
            )),
        ),
        (
            "Einde".into(),
            l(format!(
                "loopt door over het welstuk tot de trapgatrand; onderkant {z_eind:+.1}, \
                 bovenkant {:+.1}",
                z_eind + h
            )),
        ),
        (
            "Trapgat".into(),
            l("trapgat volgt uit de trap; minimaal trapgat in de toets-JSON".into()),
        ),
        (
            "Representatie".into(),
            l("IfcStair, RepresentationIdentifier 'Clearance', 'Brep' (IFC4 ADD1 concept \
               template Clearance Geometry)"
                .into()),
        ),
        ("Opdracht".into(), l(OPDRACHT.into())),
        ("Agent".into(), l(AGENT.into())),
    ]
}

fn uitgangspunten(
    inv: &Invoer,
    v: &Verdeling,
    pl: &Plat,
    paal: Option<Paalmaat>,
) -> Vec<(String, Waarde)> {
    let l = |s: String| Waarde::Label(s);
    let vakken = if v.vakken.is_empty() {
        "geen verdrijving".to_string()
    } else {
        v.vakken
            .iter()
            .map(|x| format!("{}-{}", x.van, x.tot))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let boog = match (inv.looplijn.hoek, inv.looplijn.straal) {
        (Looplijnhoek::Scherp, _) => "scherpe hoek".to_string(),
        (Looplijnhoek::Boog, Some(0.0)) => "scherpe hoek (rho 0)".to_string(),
        (Looplijnhoek::Boog, Some(r)) => format!("boog r {r:.0} (afronding rho)"),
        (Looplijnhoek::Boog, None) => {
            let r = pl
                .binnen
                .punten
                .get(1)
                .filter(|_| pl.binnen.punten.len() > 2)
                .map(|&h| {
                    let lp = verschuif(&pl.buiten, inv.looplijn.afstand_buitenzijde, pl.kant_binnen)
                        .ok()
                        .map(|x| x.projecteer(h).1)
                        .unwrap_or(0.0);
                    format!("boog r {lp:.0} uit de binnenhoek")
                })
                .unwrap_or_else(|| "recht".into());
            r
        }
    };
    let mut uit = vec![
        ("Opdracht".into(), l(OPDRACHT.into())),
        ("Bron".into(), l(format!("trap-core (open-bouwlab), invoer '{}'", inv.naam))),
        (
            "Verdrijfmethode".into(),
            l(format!("fuikmethode (Wattjes deel VIII h2 par. 8), vakken {vakken}")),
        ),
        (
            "Looplijn".into(),
            l(format!(
                "{:.0} mm uit buitenzijde (rand loopbreedte), {boog}",
                inv.looplijn.afstand_buitenzijde
            )),
        ),
        ("Voorhout".into(), Waarde::Real(inv.boom.voorhout)),
        ("Achterhout_min".into(), Waarde::Real(inv.boom.achterhout_min)),
        ("Nestdiepte".into(), Waarde::Real(inv.boom.nestdiepte)),
        (
            "Boom".into(),
            l(format!("{:.0} x {:.0}", inv.boom.dikte, inv.boom.plankbreedte)),
        ),
    ];
    if let Some(p) = paal {
        uit.push((
            "Trappaal".into(),
            l(format!(
                "{:.0} x {:.0} {}, binnenboom stomp ertegen",
                p.langs_uit + p.insteek_in,
                p.langs_in + p.insteek_uit,
                inv.trappaal.as_ref().map(|t| t.materiaal.as_str()).unwrap_or("")
            )),
        ));
    }
    if pl.buiten.punten.len() > 2 {
        uit.push((
            "Hoekverbinding".into(),
            l(match inv.hoekverbinding.doorlopend {
                Doorlopend::Hoger => "buitenboom koud (stomp); hoger deel loopt door, lager deel \
                                      stomp ertegen (Wattjes deel VIII h2 par. 7-11)"
                    .into(),
                Doorlopend::Lager => "buitenboom koud (stomp); lager deel loopt door, hoger deel \
                                      stomp ertegen"
                    .into(),
            }),
        ));
    }
    uit.push(("Agent".into(), l(AGENT.into())));
    uit
}
