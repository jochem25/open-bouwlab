//! Minimale STEP-P21-schrijver (ISO 10303-21) voor IFC4.
//!
//! Bewust klein: alleen schrijven, geen schema-kennis. Elke entiteit is een naam
//! plus een al opgemaakte argumentlijst; de helpers hieronder maken de
//! argumenten op volgens P21 (reals altijd met punt, strings ge-escaped).

use std::fmt::Write as _;

/// Verwijzing naar een geschreven entiteit (`#n`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ref(pub usize);

impl std::fmt::Display for Ref {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Debug, Default)]
pub struct Bestand {
    regels: Vec<String>,
}

impl Bestand {
    pub fn new() -> Self {
        Self::default()
    }

    /// Voeg `IFCNAAM(args)` toe; `args` zonder buitenste haakjes.
    pub fn add(&mut self, entiteit: &str, args: impl AsRef<str>) -> Ref {
        let r = Ref(self.regels.len() + 1);
        self.regels.push(format!(
            "{r}={}({});",
            entiteit.to_ascii_uppercase(),
            args.as_ref()
        ));
        r
    }

    /// Vervang de argumenten van een eerder geschreven entiteit.
    pub fn vervang(&mut self, r: Ref, entiteit: &str, args: impl AsRef<str>) {
        self.regels[r.0 - 1] = format!("{r}={}({});", entiteit.to_ascii_uppercase(), args.as_ref());
    }

    pub fn aantal(&self) -> usize {
        self.regels.len()
    }

    /// Volledig bestand met header. `tijd` als ISO 8601 (de kern leest geen klok).
    pub fn tekst(&self, bestandsnaam: &str, tijd: &str, systeem: &str) -> String {
        let mut s =
            String::with_capacity(self.regels.iter().map(|r| r.len() + 1).sum::<usize>() + 512);
        s.push_str("ISO-10303-21;\nHEADER;\n");
        s.push_str("FILE_DESCRIPTION(('ViewDefinition[DesignTransferView]'),'2;1');\n");
        let _ = writeln!(
            s,
            "FILE_NAME({},{},(''),(''),{},{},'');",
            string(bestandsnaam),
            string(tijd),
            string(systeem),
            string(systeem)
        );
        s.push_str("FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n");
        for r in &self.regels {
            s.push_str(r);
            s.push('\n');
        }
        s.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        s
    }
}

/// P21-real: kortste exacte weergave, altijd met punt (`0.`, `1.5`, `1.E-05`).
pub fn real(x: f64) -> String {
    // 15 significante cijfers (zoals ifcopenshell): met 17 cijfers liep een
    // boolean in web-ifc 0.0.77 stuk op een verschil van 1e-13 (gemeten 30-10-2026)
    let x: f64 = format!("{x:.14e}").parse().unwrap_or(x);
    let x = if x == 0.0 { 0.0 } else { x }; // geen -0.
    let s = format!("{x:?}");
    if let Some((m, e)) = s.split_once('e') {
        let m = if m.contains('.') {
            m.to_string()
        } else {
            format!("{m}.")
        };
        return format!("{m}E{e}");
    }
    match s.strip_suffix(".0") {
        Some(kop) => format!("{kop}."),
        None if s.contains('.') => s,
        None => format!("{s}."),
    }
}

/// P21-string: `'` verdubbeld, `\` verdubbeld, niet-ASCII als `\X2\hhhh\X0\`.
pub fn string(s: &str) -> String {
    let mut uit = String::with_capacity(s.len() + 2);
    uit.push('\'');
    let mut tekens = s.chars().peekable();
    while let Some(c) = tekens.next() {
        match c {
            '\'' => uit.push_str("''"),
            '\\' => uit.push_str("\\\\"),
            ' '..='~' => uit.push(c),
            _ => {
                uit.push_str("\\X2\\");
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    let _ = write!(uit, "{u:04X}");
                }
                while let Some(&d) = tekens.peek() {
                    if d.is_ascii() {
                        break;
                    }
                    for u in d.encode_utf16(&mut buf) {
                        let _ = write!(uit, "{u:04X}");
                    }
                    tekens.next();
                }
                uit.push_str("\\X0\\");
            }
        }
    }
    uit.push('\'');
    uit
}

pub fn opt_string(s: Option<&str>) -> String {
    s.map_or_else(|| "$".to_string(), string)
}

pub fn enumeratie(s: &str) -> String {
    format!(".{}.", s.to_ascii_uppercase())
}

pub fn boolean(b: bool) -> String {
    if b { ".T." } else { ".F." }.to_string()
}

pub fn lijst<I, T>(items: I) -> String
where
    I: IntoIterator<Item = T>,
    T: std::fmt::Display,
{
    let delen: Vec<String> = items.into_iter().map(|t| t.to_string()).collect();
    format!("({})", delen.join(","))
}

pub fn reals(xs: &[f64]) -> String {
    lijst(xs.iter().map(|&x| real(x)))
}

/// IfcGloballyUniqueId (22 tekens, IFC-base64) van 128 bits.
pub fn guid_uit_bits(bits: u128) -> String {
    const TEKENS: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    // eerste teken 2 bits, daarna 21 x 6 bits
    let mut uit = String::with_capacity(22);
    uit.push(TEKENS[((bits >> 126) & 0x3) as usize] as char);
    for i in (0..21).rev() {
        uit.push(TEKENS[((bits >> (i * 6)) & 0x3f) as usize] as char);
    }
    uit
}

/// Deterministische GUID uit een sleutel (FNV-1a 128): reproduceerbaar bestand.
pub fn guid(sleutel: &str) -> String {
    const OFFSET: u128 = 0x6c62272e07bb014262b821756295c58d;
    const PRIME: u128 = 0x0000000001000000000000000000013B;
    let mut h = OFFSET;
    for b in sleutel.bytes() {
        h ^= u128::from(b);
        h = h.wrapping_mul(PRIME);
    }
    guid_uit_bits(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reals_volgens_p21() {
        assert_eq!(real(0.0), "0.");
        assert_eq!(real(-0.0), "0.");
        assert_eq!(real(1.0), "1.");
        assert_eq!(real(-130.0), "-130.");
        assert_eq!(real(191.764705882353), "191.764705882353");
        assert_eq!(real(1e-5), "1.E-5");
        assert_eq!(real(1.5e20), "1.5E20");
        assert_eq!(real(190.12159308557239), "190.121593085572");
        assert_eq!(real(-2.2737367544323206e-13), "-2.27373675443232E-13");
    }

    #[test]
    fn strings_escaped() {
        assert_eq!(string("a'b"), "'a''b'");
        assert_eq!(string("tr\u{e9}\u{e9}"), "'tr\\X2\\00E900E9\\X0\\'");
        assert_eq!(string("c:\\x"), "'c:\\\\x'");
    }

    #[test]
    fn guid_22_tekens_en_stabiel() {
        let g = guid("trede 01");
        assert_eq!(g.len(), 22);
        assert_eq!(g, guid("trede 01"));
        assert_ne!(g, guid("trede 02"));
        assert!(matches!(g.as_bytes()[0], b'0'..=b'3'));
    }
}
