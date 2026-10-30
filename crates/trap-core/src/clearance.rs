//! Clearance als één gesloten volume boven het neuslijnvlak. Port van
//! `trap_3d.clearance_volume` (O-2026-10-29-06).
//!
//! Per voorkant de neuslijn (binnenzijde tot buitenzijde, op de bovenkant van
//! de trede); ertussen een regelvlak, langs de zijden z lineair in de
//! ontwikkelde lengte. Bovenvlak = onderkant + vrije hoogte; zijvlakken
//! verticaal; begin en eind verticaal in het vlak van de eerste en laatste
//! neuslijn. Vrije hoogte: Bbl bijlage I (vrije hoogte als bedoeld in NEN 2580).

use std::collections::HashMap;

use crate::model::{Clearance, P2, P3};
use crate::vlak::{afstand, oorknippen, oppervlak, Polylijn};
use crate::Fout;

/// Een strook met vier hoekpunten die binnen deze afstand vlak ligt, wordt één
/// vlak; anders twee driehoeken.
pub const VLAK_TOL: f64 = 0.01;

type PuntZ = (P2, f64);

fn ketens(vlak: &Polylijn, punten: &[P2], zs: &[f64], tol: f64) -> Result<Vec<Vec<PuntZ>>, Fout> {
    let mut st = Vec::with_capacity(punten.len());
    for &p in punten {
        let (s, d) = vlak.projecteer(p);
        if d > tol {
            return Err(Fout::Model(format!("neuspunt {p:?} ligt niet op de zijde van de trap")));
        }
        st.push(s);
    }
    if st.windows(2).any(|w| w[1] <= w[0]) {
        return Err(Fout::Model("neuspunten niet oplopend langs de zijde".into()));
    }
    let knikken = vlak.knikken();
    let mut uit = Vec::new();
    for k in 0..st.len() - 1 {
        let mut keten = vec![(punten[k], zs[k])];
        for &s in knikken {
            if st[k] + tol < s && s < st[k + 1] - tol {
                let t = (s - st[k]) / (st[k + 1] - st[k]);
                keten.push((vlak.punt_op(s), zs[k] + t * (zs[k + 1] - zs[k])));
            }
        }
        keten.push((punten[k + 1], zs[k + 1]));
        uit.push(keten);
    }
    Ok(uit)
}

/// Grootste afstand van een punt tot het best passende vlak (kleinste
/// eigenvector van de spreidingsmatrix, Jacobi).
pub fn uit_vlak(q: &[P3]) -> f64 {
    let n = q.len() as f64;
    let mut m = [0.0; 3];
    for p in q {
        for i in 0..3 {
            m[i] += p[i] / n;
        }
    }
    let mut a = [[0.0f64; 3]; 3];
    for p in q {
        let d = [p[0] - m[0], p[1] - m[1], p[2] - m[2]];
        for i in 0..3 {
            for j in 0..3 {
                a[i][j] += d[i] * d[j];
            }
        }
    }
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..60 {
        let (mut p, mut r) = (0, 1);
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            if a[i][j].abs() > a[p][r].abs() {
                (p, r) = (i, j);
            }
        }
        if a[p][r].abs() < 1e-18 {
            break;
        }
        let th = 0.5 * (2.0 * a[p][r]).atan2(a[r][r] - a[p][p]);
        let (c, s) = (th.cos(), th.sin());
        let mut b = a;
        for k in 0..3 {
            b[p][k] = c * a[p][k] - s * a[r][k];
            b[r][k] = s * a[p][k] + c * a[r][k];
        }
        let mut b2 = b;
        for k in 0..3 {
            b2[k][p] = c * b[k][p] - s * b[k][r];
            b2[k][r] = s * b[k][p] + c * b[k][r];
        }
        a = b2;
        for row in &mut v {
            let (x, y) = (row[p], row[r]);
            row[p] = c * x - s * y;
            row[r] = s * x + c * y;
        }
    }
    let k = (0..3).min_by(|&i, &j| a[i][i].total_cmp(&a[j][j])).expect("drie");
    let nrm = [v[0][k], v[1][k], v[2][k]];
    q.iter()
        .map(|p| ((p[0] - m[0]) * nrm[0] + (p[1] - m[1]) * nrm[1] + (p[2] - m[2]) * nrm[2]).abs())
        .fold(0.0, f64::max)
}

pub struct Volume {
    pub net: Clearance,
    /// Plattegrond per strook (voor controles en het trapgat).
    pub stroken: Vec<Vec<P2>>,
}

/// Neuslijnen: (punt binnenzijde, punt buitenzijde, z) in looprichting.
pub fn clearance_volume(
    neuzen: &[(P2, P2, f64)],
    binnen: &Polylijn,
    buiten: &Polylijn,
    hoogte: f64,
    naam: &str,
) -> Result<Volume, Fout> {
    let zs: Vec<f64> = neuzen.iter().map(|n| n.2).collect();
    let ki = ketens(binnen, &neuzen.iter().map(|n| n.0).collect::<Vec<_>>(), &zs, 1e-3)?;
    let ku = ketens(buiten, &neuzen.iter().map(|n| n.1).collect::<Vec<_>>(), &zs, 1e-3)?;
    let mut hp: Vec<P3> = Vec::new();
    let mut sleutel: HashMap<(i64, i64, i64), usize> = HashMap::new();
    let mut punt = |p: P2, z: f64, hp: &mut Vec<P3>| -> usize {
        let k = (
            (p[0] * 1e6).round() as i64,
            (p[1] * 1e6).round() as i64,
            (z * 1e6).round() as i64,
        );
        *sleutel.entry(k).or_insert_with(|| {
            hp.push([p[0], p[1], z]);
            hp.len() - 1
        })
    };
    let mut onder: Vec<Vec<usize>> = Vec::new();
    let mut boven: Vec<Vec<usize>> = Vec::new();
    let mut stroken = Vec::new();
    for (a, b) in ki.iter().zip(&ku) {
        let mut rand: Vec<PuntZ> = a.iter().copied().chain(b.iter().rev().copied()).collect();
        let pts: Vec<P2> = rand.iter().map(|r| r.0).collect();
        if oppervlak(&pts) < 0.0 {
            rand.reverse();
        }
        let pts: Vec<P2> = rand.iter().map(|r| r.0).collect();
        stroken.push(pts.clone());
        let q: Vec<P3> = rand.iter().map(|(p, z)| [p[0], p[1], *z]).collect();
        let delen: Vec<Vec<usize>> = if rand.len() == 4 && uit_vlak(&q) <= VLAK_TOL {
            vec![vec![0, 1, 2, 3]]
        } else if rand.len() == 4 {
            let d = |i: usize, j: usize| {
                ((q[i][0] - q[j][0]).powi(2) + (q[i][1] - q[j][1]).powi(2) + (q[i][2] - q[j][2]).powi(2)).sqrt()
            };
            if d(0, 2) <= d(1, 3) {
                vec![vec![0, 1, 2], vec![0, 2, 3]]
            } else {
                vec![vec![0, 1, 3], vec![1, 2, 3]]
            }
        } else {
            oorknippen(&pts)?.into_iter().map(|t| t.to_vec()).collect()
        };
        for d in delen {
            onder.push(d.iter().rev().map(|&i| punt(rand[i].0, rand[i].1, &mut hp)).collect());
            boven.push(d.iter().map(|&i| punt(rand[i].0, rand[i].1 + hoogte, &mut hp)).collect());
        }
    }
    let mut zij: Vec<Vec<usize>> = Vec::new();
    for (kk, vlak) in [(&ki, binnen), (&ku, buiten)] {
        let mut reeks: Vec<PuntZ> = vec![kk[0][0]];
        for k in kk.iter() {
            reeks.extend_from_slice(&k[1..]);
        }
        let knik: Vec<P2> = vlak.punten[1..vlak.punten.len() - 1].to_vec();
        let is_knik = |p: P2| knik.iter().any(|&q| afstand(p, q) < 1e-6);
        let mut stuk = vec![reeks[0]];
        for (j, pz) in reeks.iter().enumerate().skip(1) {
            stuk.push(*pz);
            if is_knik(pz.0) || j == reeks.len() - 1 {
                let mut v: Vec<usize> = stuk.iter().map(|(p, z)| punt(*p, *z, &mut hp)).collect();
                v.extend(stuk.iter().rev().map(|(p, z)| punt(*p, z + hoogte, &mut hp)));
                zij.push(v);
                stuk = vec![*pz];
            }
        }
    }
    for &(p_i, p_u, z) in [neuzen[0], neuzen[neuzen.len() - 1]].iter() {
        zij.push(vec![
            punt(p_i, z, &mut hp),
            punt(p_u, z, &mut hp),
            punt(p_u, z + hoogte, &mut hp),
            punt(p_i, z + hoogte, &mut hp),
        ]);
    }
    let gericht: std::collections::HashSet<(usize, usize)> = onder
        .iter()
        .chain(&boven)
        .flat_map(|v| (0..v.len()).map(move |i| (v[i], v[(i + 1) % v.len()])))
        .collect();
    let mut vlakken: Vec<Vec<usize>> = onder;
    vlakken.extend(boven);
    for mut v in zij {
        if (0..v.len()).any(|i| gericht.contains(&(v[i], v[(i + 1) % v.len()]))) {
            v.reverse();
        }
        vlakken.push(v);
    }
    let mut net = Clearance {
        naam: naam.to_string(),
        hoekpunten: hp,
        vlakken,
        eigenschappen: Vec::new(),
        met_zone: false,
    };
    if net_volume(&net) < 0.0 {
        for v in &mut net.vlakken {
            v.reverse();
        }
    }
    Ok(Volume { net, stroken })
}

pub fn net_volume(c: &Clearance) -> f64 {
    let p = &c.hoekpunten;
    let mut t = 0.0;
    for v in &c.vlakken {
        for i in 1..v.len() - 1 {
            let (a, b, d) = (p[v[0]], p[v[i]], p[v[i + 1]]);
            let kr = [
                b[1] * d[2] - b[2] * d[1],
                b[2] * d[0] - b[0] * d[2],
                b[0] * d[1] - b[1] * d[0],
            ];
            t += a[0] * kr[0] + a[1] * kr[1] + a[2] * kr[2];
        }
    }
    t / 6.0
}

/// Aantal gerichte randen zonder precies een tegengestelde partner (0 = gesloten).
pub fn open_randen(c: &Clearance) -> usize {
    let mut randen: HashMap<(usize, usize), usize> = HashMap::new();
    for v in &c.vlakken {
        for i in 0..v.len() {
            *randen.entry((v[i], v[(i + 1) % v.len()])).or_default() += 1;
        }
    }
    randen
        .iter()
        .filter(|(&(a, b), &n)| n != 1 || randen.get(&(b, a)) != Some(&1))
        .count()
}
