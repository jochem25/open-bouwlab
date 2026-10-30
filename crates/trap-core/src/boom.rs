//! Uitslag van een boom als lijnen en echte bogen, en de boomdelen met koude
//! hoekverbinding. Port van `boom_profiel.py` (O-2026-10-29-02), met het
//! verstek vervangen door een stompe (koude) hoek (besluit Jochem 30-10-2026).
//!
//! Constructie (Wattjes, `wattjes-deel8__h2-par5-6` r. 165-195, voorhoutcirkels):
//! - Neuslijn: veelhoek door de neuspunten (u, z) in de uitslag.
//! - Bovenkant: evenwijdig aan de neuslijn op `voorhout`; bij een bolle knik een
//!   boog met straal `voorhout` om het neuspunt, bij een holle knik een
//!   afrondboog met straal `voorhout`.
//! - Onderkant: evenwijdig aan de bovenkant op de plankbreedte; onder een holle
//!   afronding een boog met straal `voorhout + breedte`, onder een bolle knik
//!   snijden de lijnen elkaar.
//!
//! Hoekverbinding buitenboom (koud): het doorlopende deel loopt `dikte` door in
//! het hoekvierkant, het andere eindigt stomp op de hoek. Standaard loopt het
//! HOGERE deel door (Wattjes `wattjes-deel8__h2-par7-11` r. 141-145: de
//! uitkeping komt in de hogere boom, de lagere sluit er met een vlakke borst op
//! aan). In het hoekvierkant krijgt het doorlopende deel een horizontaal stuk
//! boven- of onderkant op de hoogte van de hoek, zodat de borst van het
//! stompe deel geheel tegen het doorlopende deel ligt.

use crate::invoer::{Doorlopend, Kant};
use crate::model::{Boomdeel, Profielelement, P2};
use crate::vlak::afstand;
use crate::Fout;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum El {
    Lijn { p0: P2, p1: P2 },
    Boog { p0: P2, p1: P2, c: P2, r: f64, boven: bool },
}

impl El {
    pub fn p0(&self) -> P2 {
        match *self {
            El::Lijn { p0, .. } | El::Boog { p0, .. } => p0,
        }
    }
    pub fn p1(&self) -> P2 {
        match *self {
            El::Lijn { p1, .. } | El::Boog { p1, .. } => p1,
        }
    }

    pub fn z(&self, u: f64) -> f64 {
        match *self {
            El::Lijn { p0, p1 } => {
                let t = (u - p0[0]) / (p1[0] - p0[0]);
                p0[1] + t * (p1[1] - p0[1])
            }
            El::Boog { c, r, boven, .. } => {
                let du = u - c[0];
                let h = (r * r - du * du).max(0.0).sqrt();
                c[1] + if boven { h } else { -h }
            }
        }
    }

    fn deel(&self, a: f64, b: f64) -> El {
        let (p0, p1) = ([a, self.z(a)], [b, self.z(b)]);
        match *self {
            El::Lijn { .. } => El::Lijn { p0, p1 },
            El::Boog { c, r, boven, .. } => El::Boog { p0, p1, c, r, boven },
        }
    }

    fn omgekeerd(&self) -> El {
        match *self {
            El::Lijn { p0, p1 } => El::Lijn { p0: p1, p1: p0 },
            El::Boog { p0, p1, c, r, boven } => El::Boog { p0: p1, p1: p0, c, r, boven },
        }
    }

    fn schuif(&self, du: f64) -> El {
        let s = |p: P2| [p[0] + du, p[1]];
        match *self {
            El::Lijn { p0, p1 } => El::Lijn { p0: s(p0), p1: s(p1) },
            El::Boog { p0, p1, c, r, boven } => El::Boog { p0: s(p0), p1: s(p1), c: s(c), r, boven },
        }
    }

    fn afstand(&self, p: P2) -> f64 {
        match *self {
            El::Lijn { p0, p1 } => {
                let (dx, dy) = (p1[0] - p0[0], p1[1] - p0[1]);
                let t = (((p[0] - p0[0]) * dx + (p[1] - p0[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
                (p[0] - p0[0] - t * dx).hypot(p[1] - p0[1] - t * dy)
            }
            El::Boog { p0, p1, c, r, .. } => {
                let hoek = (p[1] - c[1]).atan2(p[0] - c[0]);
                let a0 = (p0[1] - c[1]).atan2(p0[0] - c[0]);
                let a1 = (p1[1] - c[1]).atan2(p1[0] - c[0]);
                let (lo, hi) = (a0.min(a1), a0.max(a1));
                if hi - lo <= std::f64::consts::PI && lo - 1e-12 <= hoek && hoek <= hi + 1e-12 {
                    ((p[0] - c[0]).hypot(p[1] - c[1]) - r).abs()
                } else {
                    afstand(p, p0).min(afstand(p, p1))
                }
            }
        }
    }

    pub fn van_model(e: &Profielelement) -> El {
        match *e {
            Profielelement::Lijn { p0, p1 } => El::Lijn { p0, p1 },
            Profielelement::Boog { p0, p1, centrum, r, boven } => El::Boog { p0, p1, c: centrum, r, boven },
        }
    }

    pub fn naar_model(&self) -> Profielelement {
        match *self {
            El::Lijn { p0, p1 } => Profielelement::Lijn { p0, p1 },
            El::Boog { p0, p1, c, r, boven } => Profielelement::Boog { p0, p1, centrum: c, r, boven },
        }
    }
}

/// Aaneengesloten lijnen en bogen, stijgend in u.
#[derive(Debug, Clone)]
pub struct Keten {
    pub el: Vec<El>,
}

impl Keten {
    pub fn z(&self, u: f64) -> f64 {
        for e in &self.el {
            if u <= e.p1()[0] + 1e-9 {
                return e.z(u);
            }
        }
        self.el[self.el.len() - 1].z(u)
    }

    pub fn u_begin(&self) -> f64 {
        self.el[0].p0()[0]
    }
    pub fn u_eind(&self) -> f64 {
        self.el[self.el.len() - 1].p1()[0]
    }

    pub fn knip(&self, u0: f64, u1: f64) -> Vec<El> {
        self.el
            .iter()
            .filter_map(|e| {
                let (a, b) = (e.p0()[0].max(u0), e.p1()[0].min(u1));
                (b - a > 1e-9).then(|| e.deel(a, b))
            })
            .collect()
    }

    /// u waar de keten hoogte z bereikt (de keten is stijgend).
    pub fn kruising(&self, z: f64) -> Result<f64, Fout> {
        for e in &self.el {
            if e.p0()[1] - 1e-9 <= z && z <= e.p1()[1] + 1e-9 {
                return Ok(match *e {
                    El::Lijn { p0, p1 } => p0[0] + (z - p0[1]) / (p1[1] - p0[1]) * (p1[0] - p0[0]),
                    El::Boog { p0, p1, c, r, .. } => {
                        let dz = z - c[1];
                        let du = (r * r - dz * dz).max(0.0).sqrt();
                        let straf = |u: f64| {
                            (e.z(u) - z).abs()
                                + if p0[0] - 1e-6 <= u && u <= p1[0] + 1e-6 { 0.0 } else { 1e9 }
                        };
                        let (k0, k1) = (c[0] - du, c[0] + du);
                        if straf(k1) < straf(k0) { k1 } else { k0 }
                    }
                });
            }
        }
        Err(Fout::Ontwerp(format!("boomrand bereikt z = {z:.1} niet")))
    }

    pub fn afstand(&self, p: P2) -> f64 {
        self.el.iter().map(|e| e.afstand(p)).fold(f64::INFINITY, f64::min)
    }

    /// Voor u < h horizontaal op z(h), daarna de keten.
    fn vlak_voor(&self, h: f64) -> Keten {
        let zh = self.z(h);
        let mut el = vec![El::Lijn { p0: [self.u_begin().min(h - 1.0), zh], p1: [h, zh] }];
        el.extend(self.knip(h, self.u_eind()));
        Keten { el }
    }

    /// Tot h de keten, daarna horizontaal op z(h).
    fn vlak_na(&self, h: f64) -> Keten {
        let zh = self.z(h);
        let mut el = self.knip(self.u_begin(), h);
        el.push(El::Lijn { p0: [h, zh], p1: [self.u_eind().max(h + 1.0), zh] });
        Keten { el }
    }
}

fn normaal_omhoog(p: P2, q: P2) -> P2 {
    let d = [q[0] - p[0], q[1] - p[1]];
    let l = d[0].hypot(d[1]);
    [-d[1] / l, d[0] / l]
}

fn snijpunt(a: P2, da: P2, b: P2, db: P2) -> P2 {
    let det = da[0] * (-db[1]) - da[1] * (-db[0]);
    let t = ((b[0] - a[0]) * (-db[1]) - (b[1] - a[1]) * (-db[0])) / det;
    [a[0] + t * da[0], a[1] + t * da[1]]
}

fn pl(p: P2, n: P2, f: f64) -> P2 {
    [p[0] + f * n[0], p[1] + f * n[1]]
}

fn neuspunten(u: &[f64], z: &[f64]) -> Vec<P2> {
    let mut paren: Vec<P2> = u.iter().zip(z).map(|(&a, &b)| [a, b]).collect();
    paren.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let mut uit: Vec<P2> = Vec::new();
    for p in paren {
        match uit.last_mut() {
            Some(l) if p[0] - l[0] < 1.0 => l[1] = l[1].max(p[1]),
            _ => uit.push(p),
        }
    }
    uit
}

/// Bovenkant (voorhout) en onderkant (plankbreedte) over [u_van, u_tot].
pub fn boven_en_onder(
    u_neus: &[f64],
    z_neus: &[f64],
    voorhout: f64,
    breedte: f64,
    u_van: f64,
    u_tot: f64,
) -> Result<(Keten, Keten), Fout> {
    let n = neuspunten(u_neus, z_neus);
    if n.len() < 2 {
        return Err(Fout::Ontwerp("minstens twee neuspunten nodig".into()));
    }
    let normalen: Vec<P2> = n.windows(2).map(|w| normaal_omhoog(w[0], w[1])).collect();
    let richtingen: Vec<P2> = n.windows(2).map(|w| [w[1][0] - w[0][0], w[1][1] - w[0][1]]).collect();
    let mut boven: Vec<[P2; 2]> = (0..normalen.len())
        .map(|k| [pl(n[k], normalen[k], voorhout), pl(n[k + 1], normalen[k], voorhout)])
        .collect();
    let mut onder: Vec<[P2; 2]> = (0..normalen.len())
        .map(|k| {
            [
                pl(n[k], normalen[k], voorhout - breedte),
                pl(n[k + 1], normalen[k], voorhout - breedte),
            ]
        })
        .collect();
    let mut bogen_boven: Vec<Option<El>> = vec![None; normalen.len()];
    let mut bogen_onder: Vec<Option<El>> = vec![None; normalen.len()];
    for k in 1..n.len() - 1 {
        let (m0, m1) = (normalen[k - 1], normalen[k]);
        let (d0, d1) = (richtingen[k - 1], richtingen[k]);
        let kr = d0[0] * d1[1] - d0[1] * d1[0];
        if kr.abs() < 1e-12 * d0[0].hypot(d0[1]) * d1[0].hypot(d1[1]) {
            continue;
        }
        if kr < 0.0 {
            let (t0, t1) = (pl(n[k], m0, voorhout), pl(n[k], m1, voorhout));
            bogen_boven[k] = Some(El::Boog { p0: t0, p1: t1, c: n[k], r: voorhout, boven: true });
            boven[k - 1][1] = t0;
            boven[k][0] = t1;
            let s = snijpunt(onder[k - 1][0], d0, onder[k][0], d1);
            onder[k - 1][1] = s;
            onder[k][0] = s;
        } else {
            let (a0, a1) = (pl(n[k], m0, 2.0 * voorhout), pl(n[k], m1, 2.0 * voorhout));
            let c = snijpunt(a0, d0, a1, d1);
            let (t0, t1) = (pl(c, m0, -voorhout), pl(c, m1, -voorhout));
            bogen_boven[k] = Some(El::Boog { p0: t0, p1: t1, c, r: voorhout, boven: false });
            boven[k - 1][1] = t0;
            boven[k][0] = t1;
            let (b0, b1) = (pl(c, m0, -voorhout - breedte), pl(c, m1, -voorhout - breedte));
            bogen_onder[k] = Some(El::Boog { p0: b0, p1: b1, c, r: voorhout + breedte, boven: false });
            onder[k - 1][1] = b0;
            onder[k][0] = b1;
        }
    }
    let marge = 2.0 * breedte;
    let doortrekken = |l: &mut Vec<[P2; 2]>| {
        let (p, q) = (l[0][0], l[0][1]);
        l[0][0] = verleng(p, q, u_van - marge);
        let m = l.len() - 1;
        let (p, q) = (l[m][0], l[m][1]);
        l[m][1] = verleng(p, q, u_tot + marge);
    };
    doortrekken(&mut boven);
    doortrekken(&mut onder);
    Ok((keten(&boven, &bogen_boven)?, keten(&onder, &bogen_onder)?))
}

/// Bogen met een kortere koorde worden als lijn geschreven (mm). Een grotere
/// drempel (pijl 0,01 mm) maakte het in web-ifc juist slechter (30-10-2026).
pub const MIN_BOOGKOORDE: f64 = 0.5;

fn verleng(p: P2, q: P2, u: f64) -> P2 {
    let t = (u - p[0]) / (q[0] - p[0]);
    [u, p[1] + t * (q[1] - p[1])]
}

fn keten(lijnen: &[[P2; 2]], bogen: &[Option<El>]) -> Result<Keten, Fout> {
    let mut el = Vec::new();
    for (k, [p, q]) in lijnen.iter().enumerate() {
        if let Some(b) = bogen[k] {
            let koorde = afstand(b.p0(), b.p1());
            if koorde >= MIN_BOOGKOORDE {
                el.push(b);
            } else if b.p1()[0] - b.p0()[0] > 1e-9 {
                // bijna rechte knik: als lijn (pijl < 0,001 mm); web-ifc 0.0.77
                // maakt van zo'n kleine boog open randen (gemeten 30-10-2026)
                el.push(El::Lijn { p0: b.p0(), p1: b.p1() });
            }
        }
        if q[0] - p[0] > 1e-9 {
            el.push(El::Lijn { p0: *p, p1: *q });
        } else if q[0] - p[0] < -1e-6 {
            return Err(Fout::Ontwerp(format!(
                "boomrand keert om bij segment {k} ({:.1} -> {:.1}): neuspunten te dicht op elkaar \
                 voor deze plankbreedte",
                p[0], q[0]
            )));
        }
    }
    for w in el.windows(2) {
        if afstand(w[0].p1(), w[1].p0()) > 1e-6 || w[1].p1()[0] < w[1].p0()[0] {
            return Err(Fout::Ontwerp("boomrand sluit niet aan of keert om".into()));
        }
    }
    Ok(Keten { el })
}

/// Gebied tussen onder- en bovenkant binnen [u0, u1] x [z_min, z_max].
pub fn profiel(
    boven: &Keten,
    onder: &Keten,
    u0: f64,
    u1: f64,
    z_min: f64,
    z_max: f64,
) -> Result<Vec<El>, Fout> {
    let mut el: Vec<El> = Vec::new();
    let mut ua = u0;
    if onder.z(u0) < z_min {
        ua = onder.kruising(z_min)?.min(u1);
        el.push(El::Lijn { p0: [u0, z_min], p1: [ua, z_min] });
    }
    if ua < u1 {
        el.extend(onder.knip(ua, u1));
    }
    let z_onder_rechts = onder.z(u1).max(z_min);
    let mut ub = u1;
    if boven.z(u1) > z_max {
        ub = boven.kruising(z_max)?.max(u0);
    }
    let z_boven_rechts = boven.z(u1).min(z_max);
    el.push(El::Lijn { p0: [u1, z_onder_rechts], p1: [u1, z_boven_rechts] });
    if ub < u1 {
        el.push(El::Lijn { p0: [u1, z_max], p1: [ub, z_max] });
    }
    if u0 < ub {
        el.extend(boven.knip(u0, ub).iter().rev().map(El::omgekeerd));
    }
    let z_links_onder = onder.z(u0).max(z_min);
    el.push(El::Lijn { p0: [u0, boven.z(u0).min(z_max)], p1: [u0, z_links_onder] });
    let el: Vec<El> = el.into_iter().filter(|e| afstand(e.p0(), e.p1()) > 1e-6).collect();
    let n = el.len();
    for i in 0..n {
        if afstand(el[i].p1(), el[(i + 1) % n].p0()) > 1e-6 {
            return Err(Fout::Ontwerp(format!(
                "boomprofiel sluit niet: {:?} -> {:?}",
                el[i].p1(),
                el[(i + 1) % n].p0()
            )));
        }
    }
    Ok(el)
}

/// Oppervlak van een gesloten profiel van lijnen en bogen.
pub fn profiel_oppervlak(el: &[El]) -> f64 {
    let mut a = 0.0;
    for e in el {
        let (p, q) = (e.p0(), e.p1());
        a += (p[0] * q[1] - q[0] * p[1]) / 2.0;
        if let El::Boog { r, .. } = *e {
            // cirkelsegment tussen koorde en boog (boog < 180 graden); bolt de boog
            // rechts van p->q uit, dan komt het segment erbij (tegen de klok in)
            let koorde = afstand(p, q);
            let hoek = 2.0 * (koorde / (2.0 * r)).clamp(-1.0, 1.0).asin();
            let segment = r * r / 2.0 * (hoek - hoek.sin());
            let um = (p[0] + q[0]) / 2.0;
            let m = [um, e.z(um)];
            let k = (q[0] - p[0]) * (m[1] - p[1]) - (q[1] - p[1]) * (m[0] - p[0]);
            a += if k < 0.0 { segment } else { -segment };
        }
    }
    a
}

fn eenheid(p: P2, q: P2) -> P2 {
    let l = afstand(p, q);
    [(q[0] - p[0]) / l, (q[1] - p[1]) / l]
}

/// Een trappaal in een binnenhoek.
#[derive(Debug, Clone, Copy)]
pub struct Paalmaat {
    pub langs_in: f64,
    pub langs_uit: f64,
    pub insteek_in: f64,
    pub insteek_uit: f64,
}

/// Een boom langs het binnenvlak `punten` in rechte delen. Binnenhoek: stomp
/// tegen de trappaal (verplicht). Buitenhoek: koude hoekverbinding.
#[allow(clippy::too_many_arguments)]
pub fn boomdelen(
    naam: &str,
    punten: &[P2],
    naar_buiten: Kant,
    dikte: f64,
    boven: &Keten,
    onder: &Keten,
    z_min: f64,
    z_max: f64,
    paal: Option<Paalmaat>,
    doorlopend: Doorlopend,
) -> Result<Vec<Boomdeel>, Fout> {
    let mut stations = vec![0.0];
    for w in punten.windows(2) {
        stations.push(stations[stations.len() - 1] + afstand(w[0], w[1]));
    }
    let richtingen: Vec<P2> = punten.windows(2).map(|w| eenheid(w[0], w[1])).collect();
    let teken = if naar_buiten == Kant::Links { 1.0 } else { -1.0 };
    let normalen: Vec<P2> = richtingen.iter().map(|d| [-teken * d[1], teken * d[0]]).collect();
    let n_delen = richtingen.len();
    let buitenkant: Vec<bool> = (1..n_delen)
        .map(|k| {
            let (d0, d1) = (richtingen[k - 1], richtingen[k]);
            let links = d0[0] * d1[1] - d0[1] * d1[0] > 0.0;
            links != (naar_buiten == Kant::Links)
        })
        .collect();
    let mut delen = Vec::new();
    for k in 0..n_delen {
        let (mut u0, mut u1) = (stations[k], stations[k + 1]);
        let (mut b, mut o) = (boven.clone(), onder.clone());
        if k > 0 {
            let h = stations[k];
            if buitenkant[k - 1] {
                if doorlopend == Doorlopend::Hoger {
                    // dit (hogere) deel loopt door het hoekvierkant; bovenkant daar
                    // horizontaal op de hoogte van de hoek
                    u0 -= dikte;
                    b = b.vlak_voor(h);
                }
            } else {
                let p = paal.ok_or_else(|| {
                    Fout::Invoer(format!("{naam}: binnenhoek {k} zonder trappaal"))
                })?;
                u0 += p.langs_uit;
            }
        }
        if k + 1 < n_delen {
            let h = stations[k + 1];
            if buitenkant[k] {
                if doorlopend == Doorlopend::Lager {
                    u1 += dikte;
                    o = o.vlak_na(h);
                }
            } else {
                let p = paal.ok_or_else(|| {
                    Fout::Invoer(format!("{naam}: binnenhoek {} zonder trappaal", k + 1))
                })?;
                u1 -= p.langs_in;
            }
        }
        if u1 - u0 < 1.0 {
            return Err(Fout::Ontwerp(format!("{naam} deel {}: geen lengte over", k + 1)));
        }
        let prof = profiel(&b, &o, u0, u1, z_min, z_max)?;
        let s0 = stations[k];
        delen.push(Boomdeel {
            naam: format!("{naam} deel {}", k + 1),
            oorsprong: punten[k],
            richting: richtingen[k],
            naar_buiten: normalen[k],
            dikte,
            profiel: prof.iter().map(|e| e.schuif(-s0).naar_model()).collect(),
        });
    }
    Ok(delen)
}

/// Plattegrond van een trappaal in binnenhoek k van het binnenvlak `punten`.
pub fn paal_voetafdruk(punten: &[P2], k: usize, naar_buiten: Kant, m: Paalmaat) -> Vec<P2> {
    let teken = if naar_buiten == Kant::Links { 1.0 } else { -1.0 };
    let d0 = eenheid(punten[k - 1], punten[k]);
    let d1 = eenheid(punten[k], punten[k + 1]);
    let n0 = [-teken * d0[1], teken * d0[0]];
    let n1 = [-teken * d1[1], teken * d1[0]];
    let p = punten[k];
    // n0 wijst langs de uitgaande arm, n1 terug langs de inkomende arm
    let a = pl(pl(p, n0, -m.insteek_in), n1, -m.insteek_uit);
    vec![
        a,
        pl(pl(p, n0, m.langs_uit), n1, -m.insteek_uit),
        pl(pl(p, n0, m.langs_uit), n1, m.langs_in),
        pl(pl(p, n0, -m.insteek_in), n1, m.langs_in),
    ]
}

/// Plattegrond van een boomdeel.
pub fn deel_voetafdruk(d: &Boomdeel) -> Vec<P2> {
    deel_voetafdruk_met(d, 0.0)
}

/// Marge van de nestvoetafdruk voorbij de einden van een boomdeel (mm). Een nest
/// dat precies in het eindvlak stopt (stompe hoek) geeft een coplanair vlak in
/// de boolean; web-ifc 0.0.77 maakt daar een open mesh van (gemeten 30-10-2026).
/// Het stuk voorbij het eindvlak ligt buiten de boom en verandert niets aan het volume.
pub const NEST_MARGE_EIND: f64 = 1.0;

/// Voetafdruk om de nesten mee te bepalen (einden `NEST_MARGE_EIND` verlengd).
pub fn nest_voetafdruk(d: &Boomdeel) -> Vec<P2> {
    deel_voetafdruk_met(d, NEST_MARGE_EIND)
}

fn deel_voetafdruk_met(d: &Boomdeel, marge: f64) -> Vec<P2> {
    let s: Vec<f64> = d.profiel.iter().map(|e| e.p0()[0]).collect();
    let (s0, s1) = (
        s.iter().copied().fold(f64::INFINITY, f64::min) - marge,
        s.iter().copied().fold(f64::NEG_INFINITY, f64::max) + marge,
    );
    let pt = |s: f64, t: f64| {
        [
            d.oorsprong[0] + s * d.richting[0] + t * d.naar_buiten[0],
            d.oorsprong[1] + s * d.richting[1] + t * d.naar_buiten[1],
        ]
    };
    vec![pt(s0, 0.0), pt(s1, 0.0), pt(s1, d.dikte), pt(s0, d.dikte)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oppervlak_halve_cirkel() {
        // halve schijf r 1 boven de u-as, tegen de klok in
        let el = [
            El::Lijn { p0: [-1.0, 0.0], p1: [1.0, 0.0] },
            El::Boog { p0: [1.0, 0.0], p1: [0.0, 1.0], c: [0.0, 0.0], r: 1.0, boven: true },
            El::Boog { p0: [0.0, 1.0], p1: [-1.0, 0.0], c: [0.0, 0.0], r: 1.0, boven: true },
        ];
        let a = profiel_oppervlak(&el);
        assert!((a - std::f64::consts::FRAC_PI_2).abs() < 1e-12, "{a}");
    }
}
