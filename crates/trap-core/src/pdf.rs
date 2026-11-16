//! Geannoteerde plattegrond als PDF (A3 liggend, vector), zonder externe crate.
//!
//! Inhoud: trapgat/sparing (gestreept), treden met nummers, bomen en palen,
//! looplijn en klimlijn, buitenmaten, en rechts een kader met de invoer, het
//! resultaat en het oordeel per toetsregel. Puur: geeft bytes terug.
//!
//! Tekst in Helvetica (WinAnsi); tekens buiten Latin-1 worden '?'.

use std::fmt::Write as _;

use crate::bouw::{Rapport, Tekening};
use crate::contract::Referentie;
use crate::model::P2;

const BREED: f64 = 1190.55; // A3 liggend, pt
const HOOG: f64 = 841.89;
const MM_PT: f64 = 72.0 / 25.4;
const MARGE: f64 = 28.0;
const KADER_B: f64 = 330.0;
/// Standaardschalen; de grootste die past wordt gekozen.
const SCHALEN: [f64; 7] = [10.0, 20.0, 25.0, 50.0, 100.0, 200.0, 500.0];
/// Scheidingsteken voor de tweede kolom in het kader (oordeel).
const KOLOM: char = '\u{1f}';

/// Regels tekst voor het kader (titel eerst).
pub struct Kader {
    pub titel: String,
    pub regels: Vec<String>,
}

struct Blad {
    s: String,
}

impl Blad {
    fn lijn(&mut self, a: (f64, f64), b: (f64, f64)) {
        let _ = writeln!(self.s, "{:.2} {:.2} m {:.2} {:.2} l S", a.0, a.1, b.0, b.1);
    }
    fn veelhoek(&mut self, p: &[(f64, f64)], vul: bool, sluit: bool) {
        if p.len() < 2 {
            return;
        }
        let _ = write!(self.s, "{:.2} {:.2} m ", p[0].0, p[0].1);
        for q in &p[1..] {
            let _ = write!(self.s, "{:.2} {:.2} l ", q.0, q.1);
        }
        self.s.push_str(match (vul, sluit) {
            (true, _) => "h B\n",
            (false, true) => "h S\n",
            (false, false) => "S\n",
        });
    }
    fn tekst(&mut self, x: f64, y: f64, grootte: f64, t: &str) {
        // tekst vult met de vulkleur: tijdelijk zwart
        let _ = writeln!(self.s, "q 0 g BT /F1 {grootte:.1} Tf {x:.2} {y:.2} Td ({}) Tj ET Q", esc(t));
    }
    fn stijl(&mut self, dikte: f64, grijs_lijn: f64, grijs_vul: f64, streep: Option<(f64, f64)>) {
        let _ = writeln!(self.s, "{dikte:.2} w {grijs_lijn:.2} G {grijs_vul:.2} g");
        match streep {
            Some((a, b)) => {
                let _ = writeln!(self.s, "[{a:.1} {b:.1}] 0 d");
            }
            None => self.s.push_str("[] 0 d\n"),
        }
    }
    fn kleur_lijn(&mut self, r: f64, g: f64, b: f64) {
        let _ = writeln!(self.s, "{r:.2} {g:.2} {b:.2} RG");
    }
    /// Tekst gedraaid over `hoek` (rad), linksonder op (x, y).
    fn tekst_rot(&mut self, x: f64, y: f64, grootte: f64, hoek: f64, t: &str) {
        let (c, s) = (hoek.cos(), hoek.sin());
        let _ = writeln!(
            self.s,
            "q 0 g BT /F1 {grootte:.1} Tf {c:.4} {s:.4} {:.4} {c:.4} {x:.2} {y:.2} Tm ({}) Tj ET Q",
            -s,
            esc(t)
        );
    }
    /// Maatlijn van a naar b (papier, pt), evenwijdig verschoven over `af` pt
    /// langs de eenheidsnormaal `n`; hulplijnen, diagonale eindmarkering
    /// (tekenconventies maatlijn_eindmarkering) en het getal boven de lijn.
    fn maat(&mut self, a: (f64, f64), b: (f64, f64), n: (f64, f64), af: f64, tekst: &str) {
        let a2 = (a.0 + n.0 * af, a.1 + n.1 * af);
        let b2 = (b.0 + n.0 * af, b.1 + n.1 * af);
        // hulplijnen met 2 pt overstek
        self.lijn((a.0 + n.0 * 2.0, a.1 + n.1 * 2.0), (a2.0 + n.0 * 2.0, a2.1 + n.1 * 2.0));
        self.lijn((b.0 + n.0 * 2.0, b.1 + n.1 * 2.0), (b2.0 + n.0 * 2.0, b2.1 + n.1 * 2.0));
        self.lijn(a2, b2);
        let (dx, dy) = (b2.0 - a2.0, b2.1 - a2.1);
        let l = (dx * dx + dy * dy).sqrt().max(1e-9);
        let (ux, uy) = (dx / l, dy / l);
        // diagonaal: 45 graden tussen lijnrichting en normaal, 3 pt
        let (gx, gy) = ((ux + n.0) * 1.5 / 2f64.sqrt(), (uy + n.1) * 1.5 / 2f64.sqrt());
        for p in [a2, b2] {
            self.lijn((p.0 - gx, p.1 - gy), (p.0 + gx, p.1 + gy));
        }
        // getal leesbaar: van links naar rechts of van onder naar boven
        let mut hoek = uy.atan2(ux);
        if !(-std::f64::consts::FRAC_PI_2 + 1e-6..=std::f64::consts::FRAC_PI_2 + 1e-6).contains(&hoek) {
            hoek += std::f64::consts::PI;
        }
        let (c, s) = (hoek.cos(), hoek.sin());
        let breed = tekst.chars().count() as f64 * 7.0 * 0.5;
        let m = ((a2.0 + b2.0) / 2.0, (a2.1 + b2.1) / 2.0);
        // boven de lijn = loodrecht op de leesrichting, 1,5 pt
        let x = m.0 - c * breed / 2.0 - s * 1.5;
        let y = m.1 - s * breed / 2.0 + c * 1.5;
        self.tekst_rot(x, y, 7.0, hoek, tekst);
    }
}

/// Getal met komma en hoogstens een decimaal (tekenconventies
/// decimalen_maatgetal, K6): 187,5 en 260 (geen ",0").
pub fn komma(x: f64) -> String {
    let t = (x * 10.0).round() / 10.0;
    if (t - t.round()).abs() < 1e-9 {
        format!("{t:.0}")
    } else {
        format!("{t:.1}").replace('.', ",")
    }
}

fn esc(t: &str) -> String {
    let mut s = String::new();
    for c in t.chars() {
        // WinAnsi (cp1252) heeft deze tekens op 0x80..0x9F
        let win = match c {
            '\u{20ac}' => Some(0x80),
            '\u{201a}' => Some(0x82),
            '\u{201e}' => Some(0x84),
            '\u{2026}' => Some(0x85),
            '\u{2018}' => Some(0x91),
            '\u{2019}' => Some(0x92),
            '\u{201c}' => Some(0x93),
            '\u{201d}' => Some(0x94),
            '\u{2013}' => Some(0x96),
            '\u{2014}' => Some(0x97),
            _ => None,
        };
        match (c, win) {
            (_, Some(w)) => {
                let _ = write!(s, "\\{w:03o}");
            }
            ('(' | ')' | '\\', _) => {
                s.push('\\');
                s.push(c);
            }
            (c, _) if (c as u32) < 32 => s.push(' '),
            (c, _) if (c as u32) < 128 => s.push(c),
            // 0x80..0x9F zijn in Unicode stuurtekens, in WinAnsi iets anders
            (c, _) if (c as u32) < 0xa0 => s.push('?'),
            // Latin-1 vanaf 0xA0 als octale escape (WinAnsi valt daar samen)
            (c, _) if (c as u32) < 256 => {
                let _ = write!(s, "\\{:03o}", c as u32);
            }
            _ => s.push('?'),
        }
    }
    s
}

fn bbox(sets: &[&[P2]]) -> [f64; 4] {
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for set in sets {
        for p in *set {
            b[0] = b[0].min(p[0]);
            b[1] = b[1].min(p[1]);
            b[2] = b[2].max(p[0]);
            b[3] = b[3].max(p[1]);
        }
    }
    b
}

fn zwaartepunt(p: &[P2]) -> P2 {
    let n = p.len().max(1) as f64;
    [p.iter().map(|q| q[0]).sum::<f64>() / n, p.iter().map(|q| q[1]).sum::<f64>() / n]
}

/// Een maatketen langs een as: waarden (mm, projectcoordinaten) en label.
struct Keten {
    waarden: Vec<f64>,
    label: &'static str,
}

/// Ketens van binnen naar buiten langs een as: trapgat, ruwe wanden,
/// stramien; zonder referentie de maat naar de projectoorsprong (0).
fn ketens(g0: f64, g1: f64, wanden: &[f64], stramien: &[f64]) -> Vec<Keten> {
    let mut uit = vec![Keten { waarden: vec![g0, g1], label: "trapgat" }];
    if !wanden.is_empty() {
        let mut w = Vec::new();
        if let Some(l) = wanden.iter().copied().filter(|x| *x <= g0 + 1e-6).reduce(f64::max) {
            w.push(l);
        }
        w.extend([g0, g1]);
        if let Some(r) = wanden.iter().copied().filter(|x| *x >= g1 - 1e-6).reduce(f64::min) {
            w.push(r);
        }
        w.dedup_by(|a, b| (*a - *b).abs() < 0.5);
        if w.len() > 2 {
            uit.push(Keten { waarden: w, label: "ruwe wand" });
        }
    }
    if let Some(s) = stramien.iter().copied().min_by(|a, b| (a - g0).abs().total_cmp(&(b - g0).abs())) {
        let mut v = vec![s, g0, g1];
        v.sort_by(f64::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 0.5);
        uit.push(Keten { waarden: v, label: "stramien" });
    }
    if wanden.is_empty() && stramien.is_empty() {
        let mut v = vec![0.0, g0, g1];
        v.sort_by(f64::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 0.5);
        uit.push(Keten { waarden: v, label: "projectoorsprong" });
    }
    uit
}

/// Bouw de PDF. `sparing` = contour van het sparingsvolume (trapgat);
/// `referentie` = stramien- en ruwe-wandlijnen voor de positiematen.
pub fn tekening_pdf(
    t: &Tekening,
    r: &Rapport,
    sparing: Option<&[P2]>,
    kader: &Kader,
    referentie: Option<&Referentie>,
) -> Vec<u8> {
    let mut alles: Vec<&[P2]> = vec![&t.binnenzijde, &t.buitenzijde, &t.looplijn];
    alles.extend(t.treden.iter().map(Vec::as_slice));
    alles.extend(t.bomen.iter().map(Vec::as_slice));
    alles.extend(t.palen.iter().map(Vec::as_slice));
    if let Some(s) = sparing {
        alles.push(s);
    }
    // trapgat = sparing, anders de omhullende van de trap
    let g = match sparing {
        Some(s) => bbox(&[s]),
        None => bbox(&alles),
    };
    let leeg = Referentie::default();
    let rf = referentie.unwrap_or(&leeg);
    let ketens_x = ketens(g[0], g[2], &rf.wand_x, &rf.stramien_x);
    let ketens_y = ketens(g[1], g[3], &rf.wand_y, &rf.stramien_y);
    // referentiewaarden in beeld houden
    let mut extra: Vec<P2> = Vec::new();
    for k in &ketens_x {
        extra.extend(k.waarden.iter().map(|x| [*x, g[1]]));
    }
    for k in &ketens_y {
        extra.extend(k.waarden.iter().map(|y| [g[0], *y]));
    }
    alles.push(&extra);
    let b = bbox(&alles);
    let (w_mm, h_mm) = ((b[2] - b[0]).max(1.0), (b[3] - b[1]).max(1.0));
    // tekenvlak links van het kader, met ruimte voor maatlijnen
    let vlak_b = BREED - KADER_B - 3.0 * MARGE - 60.0;
    let vlak_h = HOOG - 2.0 * MARGE - 90.0;
    let schaal = SCHALEN
        .iter()
        .copied()
        .find(|n| w_mm / n * MM_PT <= vlak_b && h_mm / n * MM_PT <= vlak_h)
        .unwrap_or(1000.0);
    let k = MM_PT / schaal;
    let ox = MARGE + 60.0 + (vlak_b - w_mm * k) / 2.0;
    let oy = MARGE + 60.0 + (vlak_h - h_mm * k) / 2.0;
    let tp = |p: &P2| (ox + (p[0] - b[0]) * k, oy + (p[1] - b[1]) * k);
    let pts = |v: &[P2]| v.iter().map(tp).collect::<Vec<_>>();

    let mut d = Blad { s: String::new() };
    // kaders
    d.stijl(1.0, 0.0, 1.0, None);
    d.veelhoek(
        &[(MARGE, MARGE), (BREED - MARGE, MARGE), (BREED - MARGE, HOOG - MARGE), (MARGE, HOOG - MARGE)],
        false,
        true,
    );
    let kx = BREED - MARGE - KADER_B;
    d.lijn((kx, MARGE), (kx, HOOG - MARGE));

    // treden en welstuk
    d.stijl(0.6, 0.0, 0.97, None);
    let n = t.treden.len();
    for (i, tr) in t.treden.iter().enumerate() {
        d.veelhoek(&pts(tr), true, true);
        let z = tp(&zwaartepunt(tr));
        let label = if i + 1 == n { "W".to_string() } else { format!("{}", i + 1) };
        d.tekst(z.0 - 3.0, z.1 - 3.0, 7.0, &label);
    }
    // bomen en palen
    d.stijl(0.6, 0.0, 0.75, None);
    for v in t.bomen.iter().chain(&t.palen) {
        d.veelhoek(&pts(v), true, true);
    }
    // sparing / trapgat (bovenop de treden)
    if let Some(s) = sparing {
        d.stijl(1.2, 0.0, 1.0, Some((6.0, 3.0)));
        d.kleur_lijn(0.8, 0.1, 0.1);
        d.veelhoek(&pts(s), false, true);
        // label boven de linkerbovenhoek: vrij van de tredenummers
        let z = tp(&[g[0], g[3]]);
        d.tekst(z.0 + 2.0, z.1 + 4.0, 7.0, "trapgat / sparing");
        d.kleur_lijn(0.0, 0.0, 0.0);
    }
    // looplijn en klimlijn: vallen ze samen (looplijn op klimlijnafstand),
    // dan een lijn met een legenda die dat zegt (review Jochem 15-11: de
    // groene lijn bedekte de blauwe)
    let samen = t.looplijn.len() == t.klimlijn.len()
        && t.looplijn.iter().zip(&t.klimlijn).all(|(a, k)| (a[0] - k[0]).hypot(a[1] - k[1]) < 1.0);
    d.stijl(0.6, 0.0, 1.0, Some((5.0, 2.0)));
    d.kleur_lijn(0.1, 0.3, 0.8);
    d.veelhoek(&pts(&t.looplijn), false, false);
    if !samen {
        d.stijl(0.6, 0.0, 1.0, Some((1.0, 2.0)));
        d.kleur_lijn(0.1, 0.6, 0.2);
        d.veelhoek(&pts(&t.klimlijn), false, false);
    }
    d.kleur_lijn(0.0, 0.0, 0.0);
    let legenda = if samen {
        format!(
            "looplijn = klimlijn (blauw, streep): {} mm uit de buitenkant van de boom",
            komma(t.klimlijn_afstand)
        )
    } else {
        format!(
            "looplijn (blauw, streep); klimlijn (groen, stip) {} mm uit de buitenkant van de boom",
            komma(t.klimlijn_afstand)
        )
    };
    let res = &r.resultaat;
    // aantrede op de looplijn, langs de looplijn bij de eerste treden
    let vk = &res.voorkanten;
    if vk.len() >= 2 {
        // gecentreerd boven de looplijn, tussen voorkant 1 en 4 (binnen de
        // trap, vrij van de loopbreedtemaat aan het begin)
        let k = 1.min(vk.len() - 2);
        let e = (k + 3).min(vk.len() - 1);
        let (a, c) = (tp(&vk[k].p_loop), tp(&vk[e].p_loop));
        let (dx, dy) = (c.0 - a.0, c.1 - a.1);
        let mut hoek = dy.atan2(dx);
        if hoek.abs() > std::f64::consts::FRAC_PI_2 + 1e-6 {
            hoek += std::f64::consts::PI;
        }
        let tekst = format!("aantrede {} x {} op de looplijn", res.aantreden, komma(res.aantrede_looplijn));
        let breed = tekst.chars().count() as f64 * 6.5 * 0.5;
        let (cs, sn) = (hoek.cos(), hoek.sin());
        let m = ((a.0 + c.0) / 2.0, (a.1 + c.1) / 2.0);
        d.tekst_rot(m.0 - cs * breed / 2.0 - sn * 3.0, m.1 - sn * breed / 2.0 + cs * 3.0, 6.5, hoek, &tekst);
        // loopbreedte: tussen de randen van de loopbreedte bij de eerste
        // voorkant, buiten de trap (tegen de looprichting in)
        let p0 = vk[0].p_loop;
        let p1 = vk[1].p_loop;
        let (ux, uy) = (p0[0] - p1[0], p0[1] - p1[1]);
        let lu = ux.hypot(uy).max(1e-9);
        let lb = (vk[0].p_binnen[0] - vk[0].p_buiten[0]).hypot(vk[0].p_binnen[1] - vk[0].p_buiten[1]);
        d.stijl(0.4, 0.0, 1.0, None);
        d.maat(
            tp(&vk[0].p_buiten),
            tp(&vk[0].p_binnen),
            (ux / lu, uy / lu),
            14.0,
            &format!("loopbreedte {lb:.0}"),
        );
    }
    // maatketens: trapgat, ruwe wand / stramien / projectoorsprong
    // (papiermaten: 10 mm tot het object, 5 mm tussen de ketens)
    d.stijl(0.4, 0.0, 1.0, None);
    let eerste = 10.0 * MM_PT;
    let tussen = 5.0 * MM_PT;
    let gy = tp(&[g[0], g[1]]).1;
    let gx = tp(&[g[0], g[1]]).0;
    for (i, kt) in ketens_x.iter().enumerate() {
        let af = eerste + tussen * i as f64;
        for w in kt.waarden.windows(2) {
            let (a, c) = (tp(&[w[0], g[1]]), tp(&[w[1], g[1]]));
            d.maat((a.0, gy), (c.0, gy), (0.0, -1.0), af, &format!("{:.0}", w[1] - w[0]));
        }
        let xe = tp(&[kt.waarden[kt.waarden.len() - 1], g[1]]).0;
        d.tekst(xe + 6.0, gy - af - 2.5, 6.0, kt.label);
    }
    for (i, kt) in ketens_y.iter().enumerate() {
        let af = eerste + tussen * i as f64;
        for w in kt.waarden.windows(2) {
            let (a, c) = (tp(&[g[0], w[0]]), tp(&[g[0], w[1]]));
            d.maat((gx, a.1), (gx, c.1), (-1.0, 0.0), af, &format!("{:.0}", w[1] - w[0]));
        }
        let ye = tp(&[g[0], kt.waarden[kt.waarden.len() - 1]]).1;
        d.tekst_rot(gx - af + 2.5, ye + 6.0, 6.0, std::f64::consts::FRAC_PI_2, kt.label);
    }
    // referentielijnen (streep-punt, grijs) over het tekenvlak
    d.stijl(0.3, 0.5, 1.0, Some((8.0, 2.0)));
    let (bx0, by0) = tp(&[b[0], b[1]]);
    let (bx1, by1) = tp(&[b[2], b[3]]);
    for kt in ketens_x.iter().skip(1) {
        for x in &kt.waarden {
            if (x - g[0]).abs() > 0.5 && (x - g[2]).abs() > 0.5 {
                let px = tp(&[*x, 0.0]).0;
                d.lijn((px, by0 - 10.0), (px, by1 + 10.0));
            }
        }
    }
    for kt in ketens_y.iter().skip(1) {
        for y in &kt.waarden {
            if (y - g[1]).abs() > 0.5 && (y - g[3]).abs() > 0.5 {
                let py = tp(&[0.0, *y]).1;
                d.lijn((bx0 - 10.0, py), (bx1 + 10.0, py));
            }
        }
    }
    d.stijl(0.4, 0.0, 1.0, None);
    d.tekst(MARGE + 8.0, MARGE + 17.0, 7.0, &legenda);
    d.tekst(
        MARGE + 8.0,
        MARGE + 8.0,
        7.0,
        &format!(
            "plattegrond, maten in mm, schaal 1:{schaal:.0} op A3; trapgat {:.0} x {:.0}; projectcoordinaten x {:.0}..{:.0}, y {:.0}..{:.0}",
            g[2] - g[0],
            g[3] - g[1],
            g[0],
            g[2],
            g[1],
            g[3]
        ),
    );

    // kader rechts
    let mut y = HOOG - MARGE - 22.0;
    d.tekst(kx + 10.0, y, 12.0, &kader.titel);
    y -= 18.0;
    let mut regels = kader.regels.clone();
    regels.push(String::new());
    regels.push(format!("hoogte {} mm, {} optreden x {} mm", komma(res.hoogte), res.optreden, komma(res.optrede)));
    regels.push(format!("{} aantreden, {} mm op de looplijn", res.aantreden, komma(res.aantrede_looplijn)));
    regels.push(format!("loopbreedte {} mm, vrije hoogte {} mm", komma(res.loopbreedte), komma(res.vrije_hoogte)));
    regels.push(format!("trapgat {} x {} mm", komma(g[2] - g[0]), komma(g[3] - g[1])));
    regels.push(String::new());
    regels.push("toetsing (Bbl bindend; Wattjes/praktijk advies):".into());
    for g in &r.regels {
        let o = serde_json::to_value(g.oordeel)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();
        regels.push(format!("  {}{KOLOM}{o}", g.id));
    }
    regels.push(String::new());
    for w in r.disclaimer.split_inclusive(". ") {
        regels.push(w.trim().to_string());
    }
    for regel in regels {
        for stuk in breek(&regel, 62) {
            if y < MARGE + 10.0 {
                break;
            }
            // tab = tweede kolom (oordeel)
            match stuk.split_once(KOLOM) {
                Some((a, b)) => {
                    d.tekst(kx + 10.0, y, 7.0, a);
                    d.tekst(kx + 175.0, y, 7.0, b);
                }
                None => d.tekst(kx + 10.0, y, 7.0, &stuk),
            }
            y -= 9.0;
        }
    }
    pdf_bestand(&d.s)
}

fn breek(t: &str, n: usize) -> Vec<String> {
    if t.chars().count() <= n {
        return vec![t.to_string()];
    }
    let mut uit = Vec::new();
    let mut huidig = String::new();
    for w in t.split(' ') {
        if !huidig.is_empty() && huidig.chars().count() + 1 + w.chars().count() > n {
            uit.push(std::mem::take(&mut huidig));
            huidig.push_str("    ");
        }
        if !huidig.is_empty() && !huidig.ends_with(' ') {
            huidig.push(' ');
        }
        huidig.push_str(w);
    }
    uit.push(huidig);
    uit
}

/// Minimale PDF 1.4: een pagina, een inhoudsstroom, Helvetica.
fn pdf_bestand(inhoud: &str) -> Vec<u8> {
    let objecten = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {BREED:.2} {HOOG:.2}] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
        ),
        format!("<< /Length {} >>\nstream\n{inhoud}endstream", inhoud.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
    ];
    let mut uit: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut posities = Vec::new();
    for (i, o) in objecten.iter().enumerate() {
        posities.push(uit.len());
        uit.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).bytes());
    }
    let xref = uit.len();
    let mut x = format!("xref\n0 {}\n0000000000 65535 f \n", objecten.len() + 1);
    for p in posities {
        let _ = writeln!(x, "{p:010} 00000 n ");
    }
    let _ = write!(
        x,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objecten.len() + 1
    );
    uit.extend(x.bytes());
    uit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_en_breken() {
        assert_eq!(esc("a(b)\\"), "a\\(b\\)\\\\");
        assert_eq!(esc("\u{e9}"), "\\351");
        assert_eq!(esc("\u{2014}"), "\\227");
        assert_eq!(esc("\u{2013}\u{20ac}"), "\\226\\200");
        assert_eq!(esc("\u{85}"), "?");
        assert_eq!(esc("\u{2192}"), "?");
        assert_eq!(komma(187.5), "187,5");
        assert_eq!(komma(208.46), "208,5");
        assert_eq!(komma(260.0), "260");
        assert_eq!(komma(259.96), "260");
        assert_eq!(breek("aa bb cc", 5), vec!["aa bb".to_string(), "    cc".to_string()]);
    }

    #[test]
    fn xref_klopt() {
        let b = pdf_bestand("0 0 m 1 1 l S\n");
        // offsets zijn bytes: niet via een lossy string (de binaire kopregel)
        let staart = String::from_utf8(b[b.len() - 200..].to_vec()).unwrap();
        let start: usize = staart.rsplit("startxref\n").next().unwrap().lines().next().unwrap().parse().unwrap();
        assert!(b[start..].starts_with(b"xref"));
        let xref = String::from_utf8(b[start..].to_vec()).unwrap();
        for (i, regel) in xref.lines().skip(3).take(5).enumerate() {
            let off: usize = regel[..10].parse().unwrap();
            assert!(b[off..].starts_with(format!("{} 0 obj", i + 1).as_bytes()), "obj {}", i + 1);
        }
    }
}
