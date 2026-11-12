//! Van verdeling naar treden, welstuk en stootborden (prisma's), en de nesten
//! die ze in bomen en palen maken. Port van `trap_3d.py` (O-2026-10-29-01).
//!
//! Conventies (Wattjes, `wattjes-deel8__h2-par3-4` r. 162-185):
//! - Voorkant k is de voorkant (neus) van trede k+1; trede i ligt tussen
//!   voorkant i-1 en voorkant i + neus + stootbord (onder de volgende door).
//! - Stootbord j staat op trede j (j = 0 op de vloer) en loopt tot de
//!   onderkant van trede j+1; zijn voorvlak ligt `neus` achter voorkant j.
//! - Treden en stootborden steken `nest` in de bomen. Anders dan de referentie
//!   (strook vooraf verbreed) wordt eerst het deel van de loopstrook bepaald en
//!   worden daarna alleen de randen op een boomzijde `nest` naar buiten gezet:
//!   in een U-trap hangt de verbrede strook van een andere arm anders via de
//!   hoek aan een trede vast. Bij een kwartdraai is de uitkomst gelijk.

use crate::model::{Prisma, P2};
use crate::verdrijving::{Verdeling, Voorkant};
use crate::vlak::{
    afstand_tot_veelhoek, doorsnede_oppervlak, midden, oppervlak, plus, schoon, snijd_convex,
    snijd_halfvlak, sub, Halfvlak, Polylijn,
};
use crate::Fout;

fn verschoven(v0: P2, v1: P2, afstand: f64, naar: P2) -> (P2, P2) {
    let d = sub(v1, v0);
    let l = d[0].hypot(d[1]);
    let mut n = [-d[1] / l, d[0] / l];
    if (naar[0] - v0[0]) * n[0] + (naar[1] - v0[1]) * n[1] < 0.0 {
        n = [-n[0], -n[1]];
    }
    (plus(v0, n, afstand), plus(v1, n, afstand))
}

/// Deel van `gebied` tussen twee lijnen, de component rond `binnenpunt`.
fn band(gebied: &[P2], voor: (P2, P2), achter: (P2, P2), binnenpunt: P2) -> Result<Vec<P2>, Fout> {
    let h1 = Halfvlak::aan_kant(voor.0, sub(voor.1, voor.0), binnenpunt);
    let h2 = Halfvlak::aan_kant(achter.0, sub(achter.1, achter.0), binnenpunt);
    let stukken: Vec<Vec<P2>> = snijd_halfvlak(gebied, h1)
        .iter()
        .flat_map(|s| snijd_halfvlak(s, h2))
        .collect();
    stukken
        .into_iter()
        .min_by(|a, b| {
            afstand_tot_veelhoek(binnenpunt, a).total_cmp(&afstand_tot_veelhoek(binnenpunt, b))
        })
        .ok_or_else(|| Fout::Ontwerp("trede of stootbord heeft geen oppervlak".into()))
}

fn verder(vk: &[Voorkant], i: usize) -> P2 {
    if i + 1 < vk.len() {
        return vk[i + 1].p_loop;
    }
    let (p, q) = (vk[i - 1].p_loop, vk[i].p_loop);
    [2.0 * q[0] - p[0], 2.0 * q[1] - p[1]]
}

/// Zet de randen van `kern` die op een van de `zijden` liggen `nest` naar
/// buiten (scherpe hoeken, zoals een verschoven polylijn).
fn verbreed(kern: Vec<P2>, zijden: [&Polylijn; 2], nest: f64) -> Vec<P2> {
    let p = crate::vlak::tegen_de_klok(schoon(&kern, 1e-9));
    let n = p.len();
    if nest <= 0.0 {
        return p;
    }
    let op_zijde = |a: P2, b: P2| {
        zijden.iter().any(|z| {
            [a, b, midden(a, b)].iter().all(|&q| z.projecteer(q).1 < 1e-6)
        })
    };
    let lijnen: Vec<(P2, P2)> = (0..n)
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % n]);
            if op_zijde(a, b) {
                let d = sub(b, a);
                let l = d[0].hypot(d[1]);
                let nr = [d[1] / l, -d[0] / l]; // rechts = buiten bij tegen de klok in
                (plus(a, nr, nest), plus(b, nr, nest))
            } else {
                (a, b)
            }
        })
        .collect();
    (0..n)
        .map(|i| {
            let (a0, a1) = lijnen[(i + n - 1) % n];
            let (b0, b1) = lijnen[i];
            let (da, db) = (sub(a1, a0), sub(b1, b0));
            let k = crate::vlak::kruis(da, db);
            if k.abs() < 1e-12 * da[0].hypot(da[1]) * db[0].hypot(db[1]) {
                b0
            } else {
                plus(a0, da, crate::vlak::kruis(sub(b0, a0), db) / k)
            }
        })
        .collect()
}

pub struct Afmetingen {
    pub z_basis: f64,
    pub optrede: f64,
    pub trede_dikte: f64,
    pub neus: f64,
    /// 0 bij een open trap (dan geen stootborden).
    pub stootbord: f64,
    pub met_stootborden: bool,
}

/// Alle treden (incl. welstuk) en stootborden. `gebied` is de loopstrook
/// tussen `binnenvlak` en `buitenvlak` (zonder nesten); `eindlijn` de
/// achterkant van het welstuk (trapgatrand).
pub fn treden_en_stootborden(
    v: &Verdeling,
    gebied: &[P2],
    zijden: [&Polylijn; 2],
    nest: f64,
    eindlijn: (P2, P2),
    a: &Afmetingen,
) -> Result<(Vec<Prisma>, Vec<Prisma>), Fout> {
    let vk = &v.voorkanten;
    let n = vk.len() - 1;
    let z: Vec<f64> = (0..n + 2).map(|k| a.z_basis + k as f64 * a.optrede).collect();
    let mut treden = Vec::new();
    let mut borden = Vec::new();
    for i in 1..n + 2 {
        let voor = (vk[i - 1].p_binnen, vk[i - 1].p_buiten);
        let (achter, m, naam, soort) = if i <= n {
            (
                verschoven(vk[i].p_binnen, vk[i].p_buiten, a.neus + a.stootbord, verder(vk, i)),
                midden(vk[i - 1].p_loop, vk[i].p_loop),
                format!("trede {i:02}"),
                "TREDE",
            )
        } else {
            (
                eindlijn,
                midden(vk[n].p_loop, midden(eindlijn.0, eindlijn.1)),
                "welstuk".to_string(),
                "WELSTUK",
            )
        };
        let vorm = verbreed(band(gebied, voor, achter, m)?, zijden, nest);
        treden.push(Prisma {
            naam,
            soort: soort.into(),
            veelhoek: vorm,
            z0: z[i] - a.trede_dikte,
            z1: z[i],
        });
    }
    if a.met_stootborden {
        for j in 0..=n {
            let vd = if j < n { verder(vk, j) } else { midden(eindlijn.0, eindlijn.1) };
            let voor = verschoven(vk[j].p_binnen, vk[j].p_buiten, a.neus, vd);
            let achter = verschoven(vk[j].p_binnen, vk[j].p_buiten, a.neus + a.stootbord, vd);
            let m = midden(midden(voor.0, voor.1), midden(achter.0, achter.1));
            let vorm = verbreed(band(gebied, voor, achter, m)?, zijden, nest);
            borden.push(Prisma {
                naam: format!("stootbord {j:02}"),
                soort: "STOOTBORD".into(),
                veelhoek: vorm,
                z0: z[j],
                z1: z[j + 1] - a.trede_dikte,
            });
        }
    }
    Ok((treden, borden))
}

/// De stukken van treden en stootborden binnen een convexe plattegrond.
pub fn nesten_in_vlak(prismas: &[Prisma], voet: &[P2], min_opp: f64) -> Vec<Prisma> {
    let mut uit = Vec::new();
    for p in prismas {
        for g in snijd_convex(&p.veelhoek, voet) {
            if oppervlak(&g).abs() > min_opp {
                uit.push(Prisma {
                    naam: format!("nest {}", p.naam),
                    soort: p.soort.clone(),
                    veelhoek: schoon(&g, 1e-3),
                    z0: p.z0,
                    z1: p.z1,
                });
            }
        }
    }
    uit
}

/// Achter-onderkanten van de nesten op een boom: per prisma het hoekpunt
/// binnen de nestdiepte met de grootste u, op de onderkant van het prisma.
pub fn achterpunten(prismas: &[Prisma], vlak: &Polylijn, nest: f64) -> (Vec<f64>, Vec<f64>) {
    let (mut us, mut zs) = (Vec::new(), Vec::new());
    for p in prismas {
        let kandidaten: Vec<f64> = p
            .veelhoek
            .iter()
            .filter_map(|&q| {
                let (s, d) = vlak.projecteer(q);
                (d <= nest + 0.5).then_some(s)
            })
            .collect();
        if let Some(m) = kandidaten.into_iter().reduce(f64::max) {
            us.push(m);
            zs.push(p.z0);
        }
    }
    (us, zs)
}

/// Totaal overlappend volume tussen alle paren prisma's (moet 0 zijn).
pub fn overlap_onderling(prismas: &[Prisma]) -> Result<f64, Fout> {
    let mut totaal = 0.0;
    for (i, a) in prismas.iter().enumerate() {
        for b in &prismas[i + 1..] {
            let dz = a.z1.min(b.z1) - a.z0.max(b.z0);
            if dz <= 1e-6 {
                continue;
            }
            totaal += doorsnede_oppervlak(&a.veelhoek, &b.veelhoek)? * dz;
        }
    }
    Ok(totaal)
}

pub fn volume(p: &Prisma) -> f64 {
    oppervlak(&p.veelhoek).abs() * (p.z1 - p.z0)
}
