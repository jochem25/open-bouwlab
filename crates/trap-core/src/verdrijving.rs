//! Verdrijving op een trapcontour: looplijn, onverdreven voorkanten en de
//! fuikmethode. Port van `verdrijving_contour.py` en `verdrijving.py`
//! (ai-tekenaar, O-2026-10-29-01); de golden test vergelijkt met die keten.
//!
//! Bron: Wattjes, Constructie van Gebouwen deel VIII h2 (corpus
//! `toelichting/wattjes-deel8__h2-par7-11`, fuikmethode r. 765-870;
//! `__h2-par5-6` r. 321 klimlijn).
//!
//! Conventies:
//! - Een `Polylijn` loopt in de klimrichting; `kant_binnen` legt vast aan welke
//!   kant van de looplijn de binnenzijde ligt.
//! - De looplijn is de buitenzijde, `afstand` naar binnen verschoven; in een
//!   hoek een cirkelboog (standaard uit de binnenhoek) of een scherpe knik.
//!   De boog is de afronding van de hoek van die offsetlijn met straal rho:
//!   rho = breedte - afstand is de boog uit de binnenhoek, rho klein is bijna
//!   scherp, rho = 0 is scherp (O-2026-10-30-07).
//! - Een onverdreven voorkant staat haaks op de looplijn; in de hoekzone gaat
//!   hij door de binnenhoek.
//! - Elke voorkant gaat door zijn punt op de binnenzijde en zijn punt op de
//!   looplijn, verlengd tot de buitenzijde.

use crate::invoer::{Kant, Looplijnhoek, Vak};
use crate::model::P2;
use crate::vlak::{afstand, eenheid, kruis, normaal, plus, sub, verschuif, Polylijn, EPS};
use crate::Fout;

/// Kleinste hoek tussen een voorkant en de binnenzijde (graden).
pub const MIN_HOEK_VOORKANT: f64 = 10.0;

/// Kleinste hoek van de hoekvoorkant (tusschenkwart) met beide armen (graden);
/// 45 = precies diagonaal.
pub const MIN_HOEK_HOEKVOORKANT: f64 = 20.0;

/// Koorden per kwartcirkel (lengtefout bij r 500: 0,0012 mm).
pub const BOOG_SEGMENTEN: f64 = 360.0;

#[derive(Debug, Clone)]
pub struct Contour {
    pub binnen: Polylijn,
    pub buiten: Polylijn,
    pub kant_binnen: Kant,
    pub afstand: f64,
    pub looplijn: Polylijn,
    /// Afrondingsstraal rho per binnenhoek (mm); 0 = scherpe knik.
    pub afronding: Vec<f64>,
}

impl Contour {
    pub fn nieuw(
        binnen: Polylijn,
        buiten: Polylijn,
        kant_binnen: Kant,
        afstand: f64,
        hoek: Looplijnhoek,
        straal: Option<f64>,
    ) -> Result<Self, Fout> {
        if afstand <= 0.0 {
            return Err(Fout::Invoer("afstand looplijn tot de buitenzijde moet positief zijn".into()));
        }
        let mut ruw = verschuif(&buiten, afstand, kant_binnen)?;
        let mut afronding = vec![0.0; binnen.punten.len().saturating_sub(2)];
        if hoek == Looplijnhoek::Boog && straal != Some(0.0) {
            (ruw, afronding) = rond_af(&ruw, &binnen, straal)?;
        }
        let looplijn = knip_tussen(
            &ruw,
            (buiten.punten[0], binnen.punten[0]),
            (buiten.punten[buiten.punten.len() - 1], binnen.punten[binnen.punten.len() - 1]),
        )?;
        Ok(Self {
            binnen,
            buiten,
            kant_binnen,
            afstand,
            looplijn,
            afronding,
        })
    }

    /// Stations op de binnenzijde waar hij knikt.
    pub fn binnenhoeken(&self) -> Vec<f64> {
        self.binnen.knikken().to_vec()
    }
}

fn knip_tussen(pl: &Polylijn, begin: (P2, P2), eind: (P2, P2)) -> Result<Polylijn, Fout> {
    let snijpunt = |lijn: (P2, P2), p: P2, q: P2| -> Result<P2, Fout> {
        let (d1, d2) = (sub(lijn.1, lijn.0), sub(q, p));
        let noemer = kruis(d2, d1);
        if noemer.abs() < EPS {
            return Err(Fout::Invoer("looplijn loopt evenwijdig aan begin- of eindlijn".into()));
        }
        let t = kruis(sub(lijn.0, p), d1) / noemer;
        Ok(plus(p, d2, t))
    };
    let mut punten = pl.punten.clone();
    let n = punten.len();
    punten[0] = snijpunt(begin, punten[0], punten[1])?;
    punten[n - 1] = snijpunt(eind, punten[n - 2], punten[n - 1])?;
    Polylijn::nieuw(punten)
}

fn voet(p: P2, a: P2, b: P2) -> P2 {
    let d = sub(b, a);
    let t = ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]);
    plus(a, d, t)
}

/// Vervang elke knik van de looplijn door een cirkelboog. Zonder `straal`:
/// boog uit de binnenhoek, die moet dan even ver van beide benen liggen.
/// Met `straal`: afrondboog die beide benen raakt. Geeft ook de straal per hoek.
fn rond_af(loop_: &Polylijn, binnen: &Polylijn, straal: Option<f64>) -> Result<(Polylijn, Vec<f64>), Fout> {
    // knikken bepalen op de oorspronkelijke looplijn; daarna van achter naar
    // voren vervangen (anders kan een boogpunt van de vorige hoek dichterbij liggen)
    let punten = loop_.punten.clone();
    let mut bogen: Vec<(usize, Vec<P2>)> = Vec::new();
    let mut stralen = Vec::new();
    for &hoek in &binnen.punten[1..binnen.punten.len() - 1] {
        let k = (1..punten.len() - 1)
            .min_by(|&a, &b| afstand(punten[a], hoek).total_cmp(&afstand(punten[b], hoek)))
            .ok_or_else(|| Fout::Invoer("looplijn heeft geen knik bij de binnenhoek".into()))?;
        let (a, kp, b) = (punten[k - 1], punten[k], punten[k + 1]);
        let centrum = match straal {
            None => {
                let (v0, v1) = (voet(hoek, a, kp), voet(hoek, kp, b));
                let (r0, r1) = (afstand(hoek, v0), afstand(hoek, v1));
                if (r0 - r1).abs() > 1e-6 {
                    return Err(Fout::Invoer(format!(
                        "looplijn ligt {r0:.1} en {r1:.1} mm van de binnenhoek; geef een straal op"
                    )));
                }
                hoek
            }
            Some(r) => {
                if r <= 0.0 {
                    return Err(Fout::Invoer("straal looplijnboog moet positief zijn".into()));
                }
                // middelpunt op de bissectrice, aan de kant van de binnenhoek
                let (d0, d1) = (eenheid(sub(a, kp))?, eenheid(sub(b, kp))?);
                let bis = eenheid([d0[0] + d1[0], d0[1] + d1[1]])?;
                let half = (d0[0] * d1[0] + d0[1] * d1[1]).clamp(-1.0, 1.0).acos() / 2.0;
                plus(kp, bis, r / half.sin())
            }
        };
        let (v0, v1) = (voet(centrum, a, kp), voet(centrum, kp, b));
        // de raakpunten moeten op de benen liggen, anders past de boog niet
        let op_been = |v: P2, p: P2, q: P2| {
            let d = sub(q, p);
            let t = ((v[0] - p[0]) * d[0] + (v[1] - p[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]);
            (-1e-9..=1.0 + 1e-9).contains(&t)
        };
        if !op_been(v0, a, kp) || !op_been(v1, kp, b) {
            return Err(Fout::Invoer(format!(
                "straal looplijnboog {:.0} past niet in de hoek (raakpunt buiten de arm)",
                afstand(centrum, v0)
            )));
        }
        let r0 = afstand(centrum, v0);
        stralen.push(r0);
        let a0 = (v0[1] - centrum[1]).atan2(v0[0] - centrum[0]);
        let a1 = (v1[1] - centrum[1]).atan2(v1[0] - centrum[0]);
        let delta = (a1 - a0 + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI)
            - std::f64::consts::PI;
        let n = ((BOOG_SEGMENTEN * delta.abs() / std::f64::consts::FRAC_PI_2).round() as usize).max(2);
        let boog: Vec<P2> = (0..=n)
            .map(|j| {
                let h = a0 + delta * j as f64 / n as f64;
                [centrum[0] + r0 * h.cos(), centrum[1] + r0 * h.sin()]
            })
            .collect();
        bogen.push((k, boog));
    }
    let mut punten = punten;
    bogen.sort_by_key(|b| std::cmp::Reverse(b.0));
    for (k, boog) in bogen {
        punten.splice(k..k + 1, boog);
    }
    Ok((Polylijn::nieuw(punten)?, stralen))
}

/// Stuk looplijn waarvan de onverdreven voorkant door de binnenhoek gaat.
pub fn hoekzone(c: &Contour, hoek_station: f64) -> Result<(f64, f64), Fout> {
    let hoek = c.binnen.punt_op(hoek_station);
    let mut zones = Vec::new();
    for richting in [
        c.binnen.richting_op(hoek_station - 1.0),
        c.binnen.richting_op(hoek_station + 1.0),
    ] {
        let n = normaal(richting, c.kant_binnen.andere());
        let (s, _) = c
            .looplijn
            .snijd(hoek, n)
            .ok_or_else(|| Fout::Invoer("loodlijn uit de binnenhoek mist de looplijn".into()))?;
        zones.push(s);
    }
    Ok((zones[0].min(zones[1]), zones[0].max(zones[1])))
}

/// Station op de binnenzijde van de onverdreven voorkant door looplijnpunt `s`.
pub fn u_onverdreven(c: &Contour, zones: &[(f64, (f64, f64))], s: f64) -> Result<f64, Fout> {
    for &(hoek_s, (s0, s1)) in zones {
        if s0 - EPS <= s && s <= s1 + EPS {
            return Ok(hoek_s);
        }
    }
    let p = c.looplijn.punt_op(s);
    let n = normaal(c.looplijn.richting_op(s), c.kant_binnen);
    c.binnen
        .snijd(p, n)
        .map(|(u, _)| u)
        .ok_or_else(|| Fout::Invoer(format!("loodlijn uit looplijnpunt {s:.1} mist de binnenzijde")))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Voorkant {
    pub nummer: usize,
    pub s_loop: f64,
    pub u_binnen: f64,
    pub w_buiten: f64,
    pub p_binnen: P2,
    pub p_loop: P2,
    pub p_buiten: P2,
    pub verdreven: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Verdeling {
    pub aantrede_loop: f64,
    pub voorkanten: Vec<Voorkant>,
    pub vakken: Vec<Vak>,
    pub waarschuwingen: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waar {
    Loop,
    Binnen,
    Buiten,
}

impl Verdeling {
    /// Aantreden langs looplijn, binnen- of buitenzijde (ontwikkeld).
    pub fn aantreden(&self, waar: Waar) -> Vec<f64> {
        let w: Vec<f64> = self
            .voorkanten
            .iter()
            .map(|v| match waar {
                Waar::Loop => v.s_loop,
                Waar::Binnen => v.u_binnen,
                Waar::Buiten => v.w_buiten,
            })
            .collect();
        w.windows(2).map(|p| p[1] - p[0]).collect()
    }
}

/// Fuikmethode (Wattjes, `wattjes-deel8__h2-par7-11` r. 838-870): het stuk AB
/// in `n` termen van een rekenkundige reeks, zo dat de eerstvolgende term de
/// normale aantrede is. Geeft (posities vanaf A, verschil, eerste term).
pub fn fuik(ab: f64, n: usize, a: f64) -> Result<(Vec<f64>, f64, f64), Fout> {
    if n < 2 {
        return Err(Fout::Invoer("minstens twee treden nodig om te verdrijven".into()));
    }
    if ab <= 0.0 || a <= 0.0 {
        return Err(Fout::Ontwerp("te verdrijven lengte en aantrede moeten positief zijn".into()));
    }
    let nf = n as f64;
    let gemiddeld = ab / nf;
    let verschil = (a - gemiddeld) / ((nf + 1.0) / 2.0);
    let eerste = gemiddeld - verschil * (nf - 1.0) / 2.0;
    if eerste <= 0.0 {
        return Err(Fout::Ontwerp(format!(
            "fuik: de eerste trede wordt {eerste:.1} mm ({ab:.1} mm voor {n} termen bij aantrede \
             {a:.1}); neem minder treden in het kwart (Wattjes fig. 96 I)"
        )));
    }
    let mut pos = Vec::with_capacity(n);
    let mut loop_ = 0.0;
    for i in 0..n {
        loop_ += eerste + i as f64 * verschil;
        pos.push(loop_);
    }
    Ok((pos, verschil, eerste))
}

/// Verdeel de looplijn in `aantal` gelijke aantreden en verdrijf per vak.
pub fn verdeel(c: &Contour, aantal: usize, vakken: &[Vak]) -> Result<Verdeling, Fout> {
    verdeel_intern(c, aantal, vakken, true)
}

/// `controle`: weiger voorkanten die langs de binnenzijde lopen (uit voor
/// tussenstappen van de vakkeuze, waar een hoek nog niet verdreven is).
fn verdeel_intern(c: &Contour, aantal: usize, vakken: &[Vak], controle: bool) -> Result<Verdeling, Fout> {
    if aantal < 1 {
        return Err(Fout::Invoer("minstens een trede".into()));
    }
    let zones: Vec<(f64, (f64, f64))> = c
        .binnenhoeken()
        .into_iter()
        .map(|h| hoekzone(c, h).map(|z| (h, z)))
        .collect::<Result<_, _>>()?;
    let a = c.looplijn.lengte() / aantal as f64;
    let s: Vec<f64> = (0..=aantal).map(|k| k as f64 * a).collect();
    let mut u: Vec<f64> = s
        .iter()
        .map(|&sk| u_onverdreven(c, &zones, sk))
        .collect::<Result<_, _>>()?;
    u[0] = 0.0;
    u[aantal] = c.binnen.lengte();
    let mut verdreven = vec![false; aantal + 1];
    let hoeken = c.binnenhoeken();
    let mut waarschuwingen = Vec::new();
    for vak in vakken {
        if !(vak.van < vak.tot && vak.tot <= aantal) {
            return Err(Fout::Invoer(format!("vak {}-{} ligt buiten de trap", vak.van, vak.tot)));
        }
        let n = vak.tot - vak.van;
        let ab = u[vak.tot] - u[vak.van];
        let mut hoek_bij_a = hoeken.iter().any(|h| (u[vak.van] - h).abs() < 1e-6);
        let hoek_bij_b = hoeken.iter().any(|h| (u[vak.tot] - h).abs() < 1e-6);
        if hoek_bij_a == hoek_bij_b {
            hoek_bij_a = (u[vak.van + 1] - u[vak.van]) <= (u[vak.tot] - u[vak.tot - 1]);
        }
        if ab <= 0.0 {
            return Err(Fout::Ontwerp(format!(
                "vak {}-{}: te verdrijven stuk is {ab:.1} mm",
                vak.van, vak.tot
            )));
        }
        let (pos, verschil, eerste) = fuik(ab, n, a)?;
        if eerste > verschil && n > 1 {
            waarschuwingen.push(format!(
                "vak {}-{}: eerste verdreven aantrede {eerste:.1} groter dan het verschil {verschil:.1}; \
                 overgang zichtbaar (Wattjes, wattjes-deel8__h2-par7-11 r. 851-856)",
                vak.van, vak.tot
            ));
        }
        for i in 1..n {
            if hoek_bij_a {
                u[vak.van + i] = u[vak.van] + pos[i - 1];
            } else {
                u[vak.tot - i] = u[vak.tot] - pos[i - 1];
            }
            verdreven[vak.van + i] = true;
        }
    }
    let mut voorkanten = Vec::with_capacity(aantal + 1);
    for k in 0..=aantal {
        let p_b = c.binnen.punt_op(u[k]);
        let p_l = c.looplijn.punt_op(s[k]);
        let (w, p_u) = if k == 0 {
            (0.0, c.buiten.punten[0])
        } else if k == aantal {
            (c.buiten.lengte(), c.buiten.punten[c.buiten.punten.len() - 1])
        } else {
            c.buiten
                .snijd(p_b, eenheid(sub(p_l, p_b))?)
                .ok_or_else(|| Fout::Ontwerp(format!("voorkant {k} haalt de buitenzijde niet")))?
        };
        voorkanten.push(Voorkant {
            nummer: k,
            s_loop: s[k],
            u_binnen: u[k],
            w_buiten: w,
            p_binnen: p_b,
            p_loop: p_l,
            p_buiten: p_u,
            verdreven: verdreven[k],
        });
    }
    // een voorkant mag niet langs de binnenzijde lopen (bv. de radiale voorkant
    // op de rand van de hoekzone valt samen met de binnenzijde van de volgende arm)
    for vk in voorkanten[1..aantal].iter().filter(|_| controle) {
        let d = eenheid(sub(vk.p_buiten, vk.p_binnen))?;
        for r in [c.binnen.richting_op(vk.u_binnen - 1.0), c.binnen.richting_op(vk.u_binnen + 1.0)] {
            if kruis(d, r).abs() < MIN_HOEK_VOORKANT.to_radians().sin() {
                return Err(Fout::Ontwerp(format!(
                    "voorkant {} loopt (bijna) langs de binnenzijde",
                    vk.nummer
                )));
            }
        }
    }
    for w in voorkanten.windows(2) {
        let (v0, v1) = (&w[0], &w[1]);
        if v1.u_binnen < v0.u_binnen - 1e-6 || v1.w_buiten < v0.w_buiten - 1e-6 {
            return Err(Fout::Ontwerp(format!(
                "voorkanten {} en {} kruisen elkaar (binnen {:.1} -> {:.1}, buiten {:.1} -> {:.1})",
                v0.nummer, v1.nummer, v0.u_binnen, v1.u_binnen, v0.w_buiten, v1.w_buiten
            )));
        }
    }
    Ok(Verdeling {
        aantrede_loop: a,
        voorkanten,
        vakken: vakken.to_vec(),
        waarschuwingen,
    })
}

/// Automatische vakken: per binnenhoek een tusschenkwart (de voorkant door de
/// hoek blijft onverdreven, beide armen worden verdreven), gekozen op: eerst
/// geen zichtbare overgang (eerste verdreven aantrede <= reeksverschil, Wattjes
/// deel VIII h2 par. 7-11 r. 851-856), dan de grootste kleinste aantrede aan de
/// binnenzijde.
pub fn kies_vakken(c: &Contour, aantal: usize) -> Result<Vec<Vak>, Fout> {
    let basis = verdeel_intern(c, aantal, &[], false)?;
    let hoeken = c.binnenhoeken();
    let u: Vec<f64> = basis.voorkanten.iter().map(|v| v.u_binnen).collect();
    // bereik per hoek: tussen de hoekvoorkanten van de buurhoeken
    let bij_hoek = |h: f64| -> Vec<usize> {
        (1..aantal).filter(|&k| (u[k] - h).abs() < 1e-6).collect()
    };
    let mut gekozen: Vec<Vak> = Vec::new();
    for (i, &h) in hoeken.iter().enumerate() {
        // na de vakken van de vorige hoek (een gedeelde grens mag)
        let lo = gekozen.last().map_or(0, |v| v.tot);
        // tot voor de hoekzone van de volgende hoek: die moet zijn eerste
        // voorkant nog kunnen verdrijven
        let hi = if i + 1 == hoeken.len() {
            aantal
        } else {
            bij_hoek(hoeken[i + 1]).first().map_or(aantal, |&k| k.saturating_sub(1))
        };
        // hoekvoorkanten: door de hoek en echt diagonaal (niet op de rand van de
        // hoekzone, daar valt hij samen met de binnenzijde van een arm)
        let p_h = c.binnen.punt_op(h);
        let (r_in, r_uit) = (c.binnen.richting_op(h - 1.0), c.binnen.richting_op(h + 1.0));
        let diagonaal = |k: usize| {
            let Ok(d) = eenheid(sub(basis.voorkanten[k].p_loop, p_h)) else { return false };
            let min = MIN_HOEK_HOEKVOORKANT.to_radians().sin();
            kruis(d, r_in).abs() >= min && kruis(d, r_uit).abs() >= min
        };
        let alle_in_zone: Vec<usize> = bij_hoek(h);
        let midden: Vec<usize> = alle_in_zone.iter().copied().filter(|&k| diagonaal(k)).collect();
        let mut opties: Vec<Vec<Vak>> = Vec::new();
        if midden.is_empty() {
            for van in lo..aantal {
                for tot in van + 2..=hi {
                    if u[van] < h && h < u[tot] {
                        opties.push(vec![Vak { van, tot }]);
                    }
                }
            }
        } else {
            for &m in &midden {
                for van in lo..m {
                    for tot in m + 1..=hi {
                        let mut v = Vec::new();
                        if m - van >= 2 {
                            v.push(Vak { van, tot: m });
                        }
                        if tot - m >= 2 {
                            v.push(Vak { van: m, tot });
                        }
                        // de andere voorkanten in de hoekzone moeten verdreven worden
                        let gedekt = alle_in_zone.iter().all(|&k| k == m || (van < k && k < tot));
                        if !v.is_empty() && gedekt {
                            opties.push(v);
                        }
                    }
                }
            }
        }
        // eerst opties zonder zichtbare overgang (Wattjes: eerste verdreven
        // aantrede <= reeksverschil), daarbinnen de grootste kleinste aantrede
        // aan de binnenzijde; alleen als er zo geen is, de beste met overgang
        // (O-2026-11-12-01: standaardtrap zonder Wattjes-afwijking)
        let mut beste: Option<(bool, f64, Vec<Vak>)> = None;
        for v in opties {
            let mut alle = gekozen.clone();
            alle.extend(v.iter().copied());
            let laatste = i + 1 == hoeken.len();
            let Ok(d) = verdeel_intern(c, aantal, &alle, laatste) else { continue };
            let vloeiend = d.waarschuwingen.is_empty();
            let b = d.aantreden(Waar::Binnen);
            let score = b[lo..hi].iter().copied().fold(f64::INFINITY, f64::min);
            let beter = match &beste {
                None => true,
                Some((vl, s, _)) => (vloeiend && !vl) || (vloeiend == *vl && score > *s + 1e-9),
            };
            if beter {
                beste = Some((vloeiend, score, v));
            }
        }
        let (_, _, v) = beste.ok_or_else(|| {
            Fout::Ontwerp(format!("geen geldige verdrijving gevonden bij binnenhoek {}", i + 1))
        })?;
        gekozen.extend(v);
    }
    Ok(gekozen)
}

/// Hoe 'aantrede ter plaatse van de klimlijn, gemeten loodrecht op de voorkant'
/// (Bbl tabel 4.26) gemeten wordt bij niet-evenwijdige voorkanten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Loodrecht {
    /// A: afstand van het klimlijnpunt van voorkant k+1 tot de lijn van
    /// voorkant k (besluit planner msg_693c11b88694).
    VanafVolgende,
    /// A': vanaf het klimlijnpunt van voorkant k, loodrecht op voorkant k, tot
    /// voorkant k+1 (vraag msg_3118912d50b4).
    VanafEigen,
}

/// Aantrede tussen twee voorkanten (binnenpunt, richting) met klimlijnpunten q0, q1.
fn aantrede_tussen(w: Loodrecht, b0: P2, d0: P2, q0: P2, b1: P2, d1: P2, q1: P2) -> f64 {
    match w {
        Loodrecht::VanafVolgende => kruis(d0, sub(q1, b0)).abs(),
        Loodrecht::VanafEigen => {
            let mut n = [-d0[1], d0[0]];
            if n[0] * (q1[0] - q0[0]) + n[1] * (q1[1] - q0[1]) < 0.0 {
                n = [-n[0], -n[1]];
            }
            let k = kruis(n, d1);
            if k.abs() < EPS {
                return f64::INFINITY;
            }
            kruis(sub(b1, q0), d1) / k
        }
    }
}

/// Aantreden op de ontwerplooplijn, loodrecht op de voorkant gemeten.
pub fn aantreden_loodrecht(v: &Verdeling, w: Loodrecht) -> Result<Vec<f64>, Fout> {
    v.voorkanten
        .windows(2)
        .map(|p| {
            let d0 = eenheid(sub(p[0].p_buiten, p[0].p_binnen))?;
            let d1 = eenheid(sub(p[1].p_buiten, p[1].p_binnen))?;
            Ok(aantrede_tussen(w, p[0].p_binnen, d0, p[0].p_loop, p[1].p_binnen, d1, p[1].p_loop))
        })
        .collect()
}

pub struct Klimlijnmeting {
    /// Loodrecht op de voorkant (meting `w`).
    pub loodrecht: Vec<f64>,
    /// Afstand tussen opvolgende snijpunten, ontwikkeld langs de lijn, per trede.
    pub langs: Vec<f64>,
    /// Kleinste daarvan.
    pub langs_min: f64,
    pub tot_binnenzijde: f64,
    pub tot_buitenzijde: f64,
}

/// Aantreden op een andere klimlijn dan de ontwerplooplijn, bij vaste
/// voorkanten: de klimlijn ligt `afstand` uit de buitenzijde (in de hoek een
/// boog uit de binnenhoek, Bbl bijlage I: 'vloeiend verlopend'). Per voorkant
/// het snijpunt met de klimlijn; aantrede = afstand van het snijpunt van
/// voorkant k+1 tot de lijn van voorkant k (loodrecht op de voorkant).
pub fn aantreden_op_klimlijn(
    c: &Contour,
    v: &Verdeling,
    afstand: f64,
    w: Loodrecht,
) -> Result<Klimlijnmeting, Fout> {
    let k = Contour::nieuw(
        c.binnen.clone(),
        c.buiten.clone(),
        c.kant_binnen,
        afstand,
        Looplijnhoek::Boog,
        None,
    )
    .or_else(|_| {
        Contour::nieuw(c.binnen.clone(), c.buiten.clone(), c.kant_binnen, afstand, Looplijnhoek::Scherp, None)
    })?;
    meet_op_lijn(&k, v, w)
}

/// Aantreden op klimlijn `k` (een eigen contour, bijvoorbeeld met een andere
/// afronding rho) bij de vaste voorkanten van `v`.
pub fn meet_op_lijn(k: &Contour, v: &Verdeling, w: Loodrecht) -> Result<Klimlijnmeting, Fout> {
    let afstand = k.afstand;
    let mut q = Vec::with_capacity(v.voorkanten.len());
    let mut st = Vec::with_capacity(v.voorkanten.len());
    for vk in &v.voorkanten {
        let d = eenheid(sub(vk.p_buiten, vk.p_binnen))?;
        let (s, p) = k
            .looplijn
            .snijd(plus(vk.p_binnen, d, -1.0), d)
            .ok_or_else(|| Fout::Ontwerp(format!("voorkant {} mist de klimlijn op {afstand:.0}", vk.nummer)))?;
        q.push((p, vk.p_binnen, d));
        st.push(s);
    }
    let loodrecht = q
        .windows(2)
        .map(|p| aantrede_tussen(w, p[0].1, p[0].2, p[0].0, p[1].1, p[1].2, p[1].0))
        .collect();
    let langs: Vec<f64> = st.windows(2).map(|p| p[1] - p[0]).collect();
    let langs_min = langs.iter().copied().fold(f64::INFINITY, f64::min);
    let (b, u) = looplijn_tot_zijkanten(k);
    Ok(Klimlijnmeting { loodrecht, langs, langs_min, tot_binnenzijde: b, tot_buitenzijde: u })
}

/// Kleinste breedte van een tredevlak loodrecht op zijn voorkant (plus neus).
pub fn smalste_tredevlak(v: &Verdeling, neus: f64) -> Result<f64, Fout> {
    let mut uit = f64::INFINITY;
    for w in v.voorkanten.windows(2) {
        let d = eenheid(sub(w[0].p_buiten, w[0].p_binnen))?;
        for p in [w[1].p_binnen, w[1].p_buiten] {
            uit = uit.min(kruis(d, sub(p, w[0].p_binnen)).abs());
        }
    }
    Ok(uit + neus)
}

/// Kleinste afstand van de looplijn tot binnen- en buitenzijde.
pub fn looplijn_tot_zijkanten(c: &Contour) -> (f64, f64) {
    let min_tot = |pl: &Polylijn| {
        c.looplijn
            .punten
            .iter()
            .map(|&p| pl.projecteer(p).1)
            .fold(f64::INFINITY, f64::min)
    };
    (min_tot(&c.binnen), min_tot(&c.buiten))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuik_reeks_eindigt_op_normale_aantrede() {
        let (pos, verschil, eerste) = fuik(600.0, 4, 200.0).unwrap();
        assert!((pos[3] - 600.0).abs() < 1e-9);
        // de volgende term (5e) is de normale aantrede
        assert!((eerste + 4.0 * verschil - 200.0).abs() < 1e-9);
    }

    #[test]
    fn fuik_te_klein_faalt() {
        assert!(fuik(100.0, 6, 250.0).is_err());
    }
}
