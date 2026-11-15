//! Geannoteerde plattegrond als PDF (A3 liggend, vector), zonder externe crate.
//!
//! Inhoud: trapgat/sparing (gestreept), treden met nummers, bomen en palen,
//! looplijn en klimlijn, buitenmaten, en rechts een kader met de invoer, het
//! resultaat en het oordeel per toetsregel. Puur: geeft bytes terug.
//!
//! Tekst in Helvetica (WinAnsi); tekens buiten Latin-1 worden '?'.

use std::fmt::Write as _;

use crate::bouw::{Rapport, Tekening};
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
}

fn esc(t: &str) -> String {
    let mut s = String::new();
    for c in t.chars() {
        match c {
            '(' | ')' | '\\' => {
                s.push('\\');
                s.push(c);
            }
            c if (c as u32) < 32 => s.push(' '),
            c if (c as u32) < 128 => s.push(c),
            // Latin-1 als octale escape (WinAnsiEncoding valt daar samen)
            c if (c as u32) < 256 => {
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

/// Bouw de PDF. `sparing` = contour van het sparingsvolume (trapgat).
pub fn tekening_pdf(t: &Tekening, r: &Rapport, sparing: Option<&[P2]>, kader: &Kader) -> Vec<u8> {
    let mut alles: Vec<&[P2]> = vec![&t.binnenzijde, &t.buitenzijde, &t.looplijn];
    alles.extend(t.treden.iter().map(Vec::as_slice));
    alles.extend(t.bomen.iter().map(Vec::as_slice));
    alles.extend(t.palen.iter().map(Vec::as_slice));
    if let Some(s) = sparing {
        alles.push(s);
    }
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
        let z = tp(&zwaartepunt(s));
        d.tekst(z.0 - 30.0, z.1 + 4.0, 7.0, "trapgat / sparing");
        d.kleur_lijn(0.0, 0.0, 0.0);
    }
    // looplijn en klimlijn
    d.stijl(0.5, 0.0, 1.0, Some((2.0, 2.0)));
    d.kleur_lijn(0.1, 0.3, 0.8);
    d.veelhoek(&pts(&t.looplijn), false, false);
    d.kleur_lijn(0.1, 0.6, 0.2);
    d.veelhoek(&pts(&t.klimlijn), false, false);
    if let Some(p) = t.looplijn.first() {
        let q = tp(p);
        d.tekst(q.0 + 4.0, q.1 - 10.0, 6.5, "looplijn (blauw), klimlijn (groen)");
    }
    d.kleur_lijn(0.0, 0.0, 0.0);
    // buitenmaten
    d.stijl(0.4, 0.0, 1.0, None);
    let (x0, y0) = tp(&[b[0], b[1]]);
    let (x1, y1) = tp(&[b[2], b[3]]);
    let ym = y0 - 25.0;
    d.lijn((x0, ym), (x1, ym));
    d.lijn((x0, ym - 4.0), (x0, ym + 4.0));
    d.lijn((x1, ym - 4.0), (x1, ym + 4.0));
    d.tekst((x0 + x1) / 2.0 - 12.0, ym + 3.0, 7.0, &format!("{:.0}", b[2] - b[0]));
    let xm = x0 - 25.0;
    d.lijn((xm, y0), (xm, y1));
    d.lijn((xm - 4.0, y0), (xm + 4.0, y0));
    d.lijn((xm - 4.0, y1), (xm + 4.0, y1));
    d.tekst(xm - 22.0, (y0 + y1) / 2.0, 7.0, &format!("{:.0}", b[3] - b[1]));
    d.tekst(
        MARGE + 8.0,
        MARGE + 8.0,
        7.0,
        &format!(
            "plattegrond, maten in mm, schaal 1:{schaal:.0} op A3; projectcoordinaten x {:.0}..{:.0}, y {:.0}..{:.0}",
            b[0], b[2], b[1], b[3]
        ),
    );

    // kader rechts
    let mut y = HOOG - MARGE - 22.0;
    d.tekst(kx + 10.0, y, 12.0, &kader.titel);
    y -= 18.0;
    let res = &r.resultaat;
    let mut regels = kader.regels.clone();
    regels.push(String::new());
    regels.push(format!("hoogte {:.0} mm, {} optreden x {:.1} mm", res.hoogte, res.optreden, res.optrede));
    regels.push(format!("{} aantreden, {:.1} mm op de looplijn", res.aantreden, res.aantrede_looplijn));
    regels.push(format!("loopbreedte {:.0} mm, vrije hoogte {:.0} mm", res.loopbreedte, res.vrije_hoogte));
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
        assert_eq!(esc("\u{2014}"), "?");
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
