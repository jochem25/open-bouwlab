//! 2D-meetkunde in plattegrond: vectoren, polylijnen en veelhoeken.
//!
//! Port van de plattegrondhulpen uit de Python-referentie
//! (`verdrijving_contour.py`, `trap_3d.py`). Waar die shapely gebruikt, staat
//! hier een eigen, exacte bewerking: het afsnijden van een (niet-convexe)
//! veelhoek met een halfvlak, met alle losse stukken als aparte veelhoeken.

use crate::invoer::Kant;
use crate::model::P2;
use crate::Fout;

pub const EPS: f64 = 1e-9;

pub fn sub(p: P2, q: P2) -> P2 {
    [p[0] - q[0], p[1] - q[1]]
}

pub fn plus(p: P2, v: P2, f: f64) -> P2 {
    [p[0] + f * v[0], p[1] + f * v[1]]
}

pub fn kruis(p: P2, q: P2) -> f64 {
    p[0] * q[1] - p[1] * q[0]
}

pub fn dot(p: P2, q: P2) -> f64 {
    p[0] * q[0] + p[1] * q[1]
}

pub fn afstand(p: P2, q: P2) -> f64 {
    (p[0] - q[0]).hypot(p[1] - q[1])
}

pub fn midden(p: P2, q: P2) -> P2 {
    [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0]
}

pub fn eenheid(v: P2) -> Result<P2, Fout> {
    let l = v[0].hypot(v[1]);
    if l < EPS {
        return Err(Fout::Invoer("segment van lengte nul".into()));
    }
    Ok([v[0] / l, v[1] / l])
}

/// Eenheidsnormaal naar `kant` van de richting.
pub fn normaal(r: P2, kant: Kant) -> P2 {
    match kant {
        Kant::Links => [-r[1], r[0]],
        Kant::Rechts => [r[1], -r[0]],
    }
}

// ------------------------------------------------------------------ polylijn

#[derive(Debug, Clone, PartialEq)]
pub struct Polylijn {
    pub punten: Vec<P2>,
    st: Vec<f64>,
}

impl Polylijn {
    pub fn nieuw(punten: Vec<P2>) -> Result<Self, Fout> {
        if punten.len() < 2 {
            return Err(Fout::Invoer("een polylijn heeft minstens twee punten".into()));
        }
        let mut st = vec![0.0];
        for w in punten.windows(2) {
            let l = afstand(w[0], w[1]);
            if l < EPS {
                return Err(Fout::Invoer(format!("polylijn: dubbel punt {:?}", w[0])));
            }
            st.push(st[st.len() - 1] + l);
        }
        Ok(Self { punten, st })
    }

    pub fn segmenten(&self) -> impl Iterator<Item = (P2, P2)> + '_ {
        self.punten.windows(2).map(|w| (w[0], w[1]))
    }

    pub fn lengte(&self) -> f64 {
        self.st[self.st.len() - 1]
    }

    /// Ontwikkelde afstand van elk hoekpunt vanaf het begin.
    pub fn stations(&self) -> &[f64] {
        &self.st
    }

    /// Stations van de knikken (zonder begin en eind).
    pub fn knikken(&self) -> &[f64] {
        &self.st[1..self.st.len() - 1]
    }

    fn segment_bij(&self, s: f64) -> usize {
        // eerste k met s <= st[k+1], anders het laatste segment
        let n = self.punten.len() - 1;
        let k = self.st[1..].partition_point(|&x| x < s);
        k.min(n - 1)
    }

    /// Punt op ontwikkelde afstand `s`; buiten het bereik doorgetrokken.
    pub fn punt_op(&self, s: f64) -> P2 {
        let k = self.segment_bij(s);
        let (p, q) = (self.punten[k], self.punten[k + 1]);
        let t = (s - self.st[k]) / (self.st[k + 1] - self.st[k]);
        [p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]
    }

    pub fn richting_op(&self, s: f64) -> P2 {
        let n = self.punten.len() - 1;
        let k = self.st[1..].partition_point(|&x| s >= x - EPS).min(n - 1);
        let (p, q) = (self.punten[k], self.punten[k + 1]);
        let l = afstand(p, q);
        [(q[0] - p[0]) / l, (q[1] - p[1]) / l]
    }

    /// Station en afstand van het dichtstbijzijnde punt op de polylijn.
    pub fn projecteer(&self, punt: P2) -> (f64, f64) {
        let (mut beste, mut beste_s) = (f64::INFINITY, 0.0);
        for (k, (p, q)) in self.segmenten().enumerate() {
            let d = sub(q, p);
            let l2 = dot(d, d);
            let t = (dot(sub(punt, p), d) / l2).clamp(0.0, 1.0);
            let voet = plus(p, d, t);
            let a = afstand(voet, punt);
            if a < beste - EPS {
                beste = a;
                beste_s = self.st[k] + t * l2.sqrt();
            }
        }
        (beste_s, beste)
    }

    /// Grootste richtingsverandering tussen opvolgende segmenten (graden):
    /// maat voor 'vloeiend' (raaklijncontinu op de koordenbenadering na).
    pub fn grootste_knik(&self) -> f64 {
        self.punten
            .windows(3)
            .map(|w| {
                let (a, b) = (sub(w[1], w[0]), sub(w[2], w[1]));
                kruis(a, b).atan2(dot(a, b)).abs().to_degrees()
            })
            .fold(0.0, f64::max)
    }

    /// Eerste snijpunt van de halfrechte p0 + t*richting (t > 0): (station, punt).
    pub fn snijd(&self, p0: P2, richting: P2) -> Option<(f64, P2)> {
        let mut beste: Option<(f64, f64, P2)> = None;
        for (k, (p, q)) in self.segmenten().enumerate() {
            let d = sub(q, p);
            let noemer = kruis(richting, d);
            if noemer.abs() < EPS {
                continue;
            }
            let w = sub(p, p0);
            let t = kruis(w, d) / noemer;
            let u = kruis(w, richting) / noemer;
            if t > EPS && (-1e-7..=1.0 + 1e-7).contains(&u) && beste.is_none_or(|b| t < b.0) {
                let s = self.st[k] + u * dot(d, d).sqrt();
                beste = Some((t, s, plus(p0, richting, t)));
            }
        }
        beste.map(|b| (b.1, b.2))
    }
}

/// Evenwijdige polylijn op `afstand` naar `kant`, scherpe hoeken.
pub fn verschuif(pl: &Polylijn, afstand: f64, kant: Kant) -> Result<Polylijn, Fout> {
    let mut lijnen = Vec::new();
    for (p, q) in pl.segmenten() {
        let n = normaal(eenheid(sub(q, p))?, kant);
        lijnen.push((plus(p, n, afstand), plus(q, n, afstand)));
    }
    let mut punten = vec![lijnen[0].0];
    for w in lijnen.windows(2) {
        let ((a0, a1), (b0, b1)) = (w[0], w[1]);
        let (da, db) = (sub(a1, a0), sub(b1, b0));
        let noemer = kruis(da, db);
        if noemer.abs() < EPS {
            punten.push(a1);
            continue;
        }
        let t = kruis(sub(b0, a0), db) / noemer;
        punten.push(plus(a0, da, t));
    }
    punten.push(lijnen[lijnen.len() - 1].1);
    Polylijn::nieuw(punten)
}

// ----------------------------------------------------------------- veelhoek

/// Oppervlak met teken (positief = tegen de klok in).
pub fn oppervlak(p: &[P2]) -> f64 {
    let n = p.len();
    (0..n).map(|i| kruis(p[i], p[(i + 1) % n])).sum::<f64>() / 2.0
}

pub fn tegen_de_klok(mut p: Vec<P2>) -> Vec<P2> {
    if oppervlak(&p) < 0.0 {
        p.reverse();
    }
    p
}

/// Halfvlak `n . (x - p) >= 0`.
#[derive(Debug, Clone, Copy)]
pub struct Halfvlak {
    pub p: P2,
    pub n: P2,
}

impl Halfvlak {
    /// Halfvlak begrensd door de lijn door `p` met `richting`, aan de kant van `kant`.
    pub fn aan_kant(p: P2, richting: P2, kant: P2) -> Self {
        let mut n = [-richting[1], richting[0]];
        if dot(sub(kant, p), n) < 0.0 {
            n = [-n[0], -n[1]];
        }
        Self { p, n }
    }

    fn s(&self, q: P2) -> f64 {
        let v = dot(sub(q, self.p), self.n);
        if v.abs() < 1e-9 * dot(self.n, self.n).sqrt() {
            0.0
        } else {
            v
        }
    }
}

/// Het deel van een enkelvoudige veelhoek in het halfvlak, als losse
/// veelhoeken (tegen de klok in).
pub fn snijd_halfvlak(veelhoek: &[P2], h: Halfvlak) -> Vec<Vec<P2>> {
    let poly = tegen_de_klok(veelhoek.to_vec());
    let m = poly.len();
    let s: Vec<f64> = poly.iter().map(|&q| h.s(q)).collect();
    if s.iter().all(|&v| v >= 0.0) {
        return if s.iter().any(|&v| v > 0.0) { vec![poly] } else { vec![] };
    }
    if s.iter().all(|&v| v <= 0.0) {
        return vec![];
    }
    let mut ring: Vec<(P2, f64)> = Vec::with_capacity(2 * m);
    for i in 0..m {
        let j = (i + 1) % m;
        ring.push((poly[i], s[i]));
        if s[i] * s[j] < 0.0 {
            let t = s[i] / (s[i] - s[j]);
            let (a, b) = (poly[i], poly[j]);
            ring.push(([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])], 0.0));
        }
    }
    let start = ring.iter().position(|r| r.1 < 0.0).expect("negatief punt");
    ring.rotate_left(start);
    // ketens binnen het halfvlak, van binnenkomst tot uittreden (beide op de lijn)
    let mut ketens: Vec<Vec<P2>> = Vec::new();
    let mut huidig: Vec<(P2, f64)> = Vec::new();
    for r in ring.iter().chain(std::iter::once(&ring[0])) {
        if r.1 >= 0.0 {
            huidig.push(*r);
        } else {
            if huidig.iter().any(|c| c.1 > 0.0) {
                ketens.push(huidig.iter().map(|c| c.0).collect());
            }
            huidig.clear();
        }
    }
    // langs de snijlijn loopt de rand (tegen de klok in) in richting l
    let l = [h.n[1], -h.n[0]];
    let t = |q: P2| dot(q, l);
    let mut gebruikt = vec![false; ketens.len()];
    let mut uit = Vec::new();
    for begin in 0..ketens.len() {
        if gebruikt[begin] {
            continue;
        }
        let mut lus: Vec<P2> = Vec::new();
        let mut k = begin;
        loop {
            gebruikt[k] = true;
            lus.extend_from_slice(&ketens[k]);
            let t_uit = t(*ketens[k].last().expect("keten"));
            // volgende binnenkomst: kleinste t >= t_uit
            let volgende = (0..ketens.len())
                .filter(|&j| t(ketens[j][0]) >= t_uit - 1e-9)
                .min_by(|&a, &b| t(ketens[a][0]).total_cmp(&t(ketens[b][0])));
            match volgende {
                Some(j) if j != begin && !gebruikt[j] => k = j,
                _ => break,
            }
        }
        let lus = zonder_dubbel(lus);
        if lus.len() >= 3 && oppervlak(&lus).abs() > 1e-12 {
            uit.push(lus);
        }
    }
    uit
}

/// Opvolgende (bijna) gelijke punten weg, ook over de sluiting.
pub fn zonder_dubbel(p: Vec<P2>) -> Vec<P2> {
    let mut uit: Vec<P2> = Vec::with_capacity(p.len());
    for q in p {
        if uit.last().is_none_or(|&l| afstand(l, q) > 1e-9) {
            uit.push(q);
        }
    }
    while uit.len() > 1 && afstand(uit[0], uit[uit.len() - 1]) <= 1e-9 {
        uit.pop();
    }
    uit
}

/// Doorsnede van een enkelvoudige veelhoek met een convexe veelhoek.
pub fn snijd_convex(veelhoek: &[P2], convex: &[P2]) -> Vec<Vec<P2>> {
    let c = tegen_de_klok(convex.to_vec());
    let mut stukken = vec![veelhoek.to_vec()];
    for i in 0..c.len() {
        let (a, b) = (c[i], c[(i + 1) % c.len()]);
        let d = sub(b, a);
        let h = Halfvlak {
            p: a,
            n: [-d[1], d[0]],
        };
        stukken = stukken.iter().flat_map(|s| snijd_halfvlak(s, h)).collect();
        if stukken.is_empty() {
            break;
        }
    }
    stukken
}

/// Afstand van een punt tot een veelhoek (0 als het erin ligt).
pub fn afstand_tot_veelhoek(p: P2, v: &[P2]) -> f64 {
    if punt_in_veelhoek(p, v) {
        return 0.0;
    }
    (0..v.len())
        .map(|i| afstand_tot_segment(p, v[i], v[(i + 1) % v.len()]))
        .fold(f64::INFINITY, f64::min)
}

pub fn afstand_tot_segment(p: P2, a: P2, b: P2) -> f64 {
    let d = sub(b, a);
    let t = (dot(sub(p, a), d) / dot(d, d)).clamp(0.0, 1.0);
    afstand(p, plus(a, d, t))
}

pub fn punt_in_veelhoek(p: P2, v: &[P2]) -> bool {
    let mut binnen = false;
    let n = v.len();
    for i in 0..n {
        let (a, b) = (v[i], v[(i + 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            if p[0] < x {
                binnen = !binnen;
            }
        }
    }
    binnen
}

/// Zonder (bijna) dubbele en rechtdoorlopende hoekpunten (`_schoon` in de
/// referentie). Een segment van lengte nul in een IFC-profiel laat de
/// geometriekern een nest stil overslaan (les 29-10-2026).
pub fn schoon(punten: &[P2], tol: f64) -> Vec<P2> {
    let mut uit = punten.to_vec();
    let mut veranderd = true;
    while veranderd && uit.len() > 3 {
        veranderd = false;
        let n = uit.len();
        for i in 0..n {
            let (a, b, c) = (uit[(i + n - 1) % n], uit[i], uit[(i + 1) % n]);
            let k = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
            if afstand(a, b) < tol || k.abs() < tol * afstand(a, c).max(tol) {
                uit.remove(i);
                veranderd = true;
                break;
            }
        }
    }
    uit
}

/// Driehoeken van een enkelvoudige veelhoek (tegen de klok in), oorknippen.
pub fn oorknippen(pl: &[P2]) -> Result<Vec<[usize; 3]>, Fout> {
    let mut idx: Vec<usize> = (0..pl.len()).collect();
    let mut uit = Vec::new();
    let kr = |a: P2, b: P2, c: P2| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    while idx.len() > 3 {
        let n = idx.len();
        let mut gevonden = None;
        for k in 0..n {
            let (a, b, c) = (idx[(k + n - 1) % n], idx[k], idx[(k + 1) % n]);
            if kr(pl[a], pl[b], pl[c]) <= 1e-9 {
                continue;
            }
            let erin = idx
                .iter()
                .filter(|&&j| j != a && j != b && j != c)
                .filter(|&&j| [a, b, c].iter().all(|&h| afstand(pl[j], pl[h]) > 1e-9))
                // ook een punt OP de diagonaal blokkeert (anders een oor over een
                // inspringende hoek heen)
                .any(|&j| strikt_in_driehoek(pl[j], pl[a], pl[b], pl[c], -1e-9));
            if erin {
                continue;
            }
            gevonden = Some((k, [a, b, c]));
            break;
        }
        match gevonden {
            Some((k, d)) => {
                uit.push(d);
                idx.remove(k);
            }
            None => return Err(Fout::Model("oorknippen loopt vast: geen enkelvoudige veelhoek".into())),
        }
    }
    uit.push([idx[0], idx[1], idx[2]]);
    Ok(uit)
}

/// p ligt meer dan `marge` binnen driehoek abc (tegen de klok in); een
/// negatieve marge telt ook punten op de rand mee.
fn strikt_in_driehoek(p: P2, a: P2, b: P2, c: P2, marge: f64) -> bool {
    [(a, b), (b, c), (c, a)].iter().all(|&(u, v)| {
        let d = sub(v, u);
        kruis(d, sub(p, u)) / dot(d, d).sqrt() > marge
    })
}

/// Oppervlak van de doorsnede van twee enkelvoudige veelhoeken.
pub fn doorsnede_oppervlak(a: &[P2], b: &[P2]) -> Result<f64, Fout> {
    let b = tegen_de_klok(b.to_vec());
    let mut som = 0.0;
    for d in oorknippen(&b)? {
        let tri = [b[d[0]], b[d[1]], b[d[2]]];
        som += snijd_convex(a, &tri)
            .iter()
            .map(|s| oppervlak(s).abs())
            .sum::<f64>();
    }
    Ok(som)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halfvlak_geeft_twee_stukken() {
        // U-vorm: open aan de bovenkant; snijlijn y = 1 houdt de twee benen over
        let u = vec![
            [0.0, 0.0],
            [3.0, 0.0],
            [3.0, 2.0],
            [2.0, 2.0],
            [2.0, 0.5],
            [1.0, 0.5],
            [1.0, 2.0],
            [0.0, 2.0],
        ];
        let h = Halfvlak { p: [0.0, 1.0], n: [0.0, 1.0] };
        let st = snijd_halfvlak(&u, h);
        assert_eq!(st.len(), 2);
        let opp: f64 = st.iter().map(|s| oppervlak(s)).sum();
        assert!((opp - 2.0).abs() < 1e-12);
        let h2 = Halfvlak { p: [0.0, 1.0], n: [0.0, -1.0] };
        let st2 = snijd_halfvlak(&u, h2);
        assert_eq!(st2.len(), 1);
        assert!((oppervlak(&st2[0]) - (3.0 - 0.5)).abs() < 1e-12);
    }

    #[test]
    fn doorsnede_l_vorm() {
        let l = vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0], [1.0, 2.0], [0.0, 2.0]];
        let vierkant = vec![[0.5, 0.5], [1.5, 0.5], [1.5, 1.5], [0.5, 1.5]];
        let a = doorsnede_oppervlak(&l, &vierkant).unwrap();
        assert!((a - 0.75).abs() < 1e-12);
        let b = doorsnede_oppervlak(&vierkant, &l).unwrap();
        assert!((b - 0.75).abs() < 1e-12);
    }
}
