//! Toetsverslag Bbl (markdown, plain ASCII) bij de headless trapontwerper.
//!
//! Per toetsregel: oordeel, waarde/grens, het Bbl-artikel en de
//! `artikel_status` + `bron_uri` uit het normcorpus (kennis-v2, skill
//! normcorpus-raadplegen). Het corpus lezen is I/O: de aanroeper (CLI) geeft
//! een opzoekfunctie mee. Ontbreekt het corpus, dan staat er "niet
//! geverifieerd" - nooit een aangenomen status.
//!
//! 'Gerekend met' = de effectieve invoer (`ContractUitvoer::gerekend_met`).

use std::fmt::Write as _;

use crate::ContractUitvoer;

/// Opzoeken in het corpus: (artikel, lid, bron_id) -> wat het corpus zegt.
pub type Opzoek<'a> = &'a dyn Fn(&str, Option<&str>, &str) -> Option<Artikel>;

/// Wat het corpus over een artikel zegt.
#[derive(Debug, Clone, PartialEq)]
pub struct Artikel {
    pub artikel_status: String,
    pub bron_uri: String,
    /// Corpusbestand (relatief aan kennis-v2), voor de vindplaats.
    pub bestand: String,
    /// Toestand van de bron (frontmatter `geldig_van`).
    pub toestand: String,
}

/// Bbl-verwijzingen in een bron-tekst: ("4.26", Some("1")), ("4.26a", None), ...
pub fn bbl_artikelen(bron: &str) -> Vec<(String, Option<String>)> {
    let mut uit = Vec::new();
    let mut rest = bron;
    while let Some(i) = rest.find("Bbl art. ") {
        rest = &rest[i + 9..];
        let art: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '.').collect();
        let art = art.trim_end_matches('.').to_string();
        let na = &rest[art.len()..];
        let lid = na
            .strip_prefix(" lid ")
            .map(|l| l.chars().take_while(char::is_ascii_digit).collect::<String>())
            .filter(|l| !l.is_empty());
        if !art.is_empty() && !uit.contains(&(art.clone(), lid.clone())) {
            uit.push((art, lid));
        }
    }
    uit
}

/// Zoek in de tekst van een corpusbestand de bronregel van artikel/lid.
pub fn zoek_in_corpus(tekst: &str, bestand: &str, artikel: &str, lid: Option<&str>) -> Option<Artikel> {
    let toestand = tekst
        .lines()
        .find_map(|l| l.strip_prefix("geldig_van:"))
        .map(|s| s.trim().trim_matches('"').to_string())
        .unwrap_or_default();
    let veld = |regel: &str, k: &str| -> Option<String> {
        regel
            .split(" | ")
            .find_map(|d| d.trim_matches('`').trim().strip_prefix(&format!("{k}: ")).map(str::to_string))
    };
    tekst
        .lines()
        .filter(|l| l.starts_with("`type:"))
        .filter(|l| veld(l, "artikel").as_deref() == Some(artikel))
        .find(|l| lid.is_none() || veld(l, "lid").as_deref() == lid)
        .map(|l| Artikel {
            artikel_status: veld(l, "artikel_status").unwrap_or_else(|| "onbekend".into()),
            bron_uri: veld(l, "bron_uri").unwrap_or_default(),
            bestand: bestand.into(),
            toestand: toestand.clone(),
        })
}

/// Laatste wijzigingsmoment van het Bbl (1 januari of 1 juli) op of voor `datum` (JJJJ-MM-DD).
pub fn laatste_wijzigingsmoment(datum: &str) -> String {
    let jaar = datum.get(0..4).unwrap_or("1970");
    let maand: u32 = datum.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(1);
    if maand >= 7 {
        format!("{jaar}-07-01")
    } else {
        format!("{jaar}-01-01")
    }
}

fn r1(x: f64) -> String {
    format!("{x:.1}")
}

/// Het verslag. `opzoek(artikel, lid, bron_id)` geeft wat het corpus zegt.
pub fn toetsverslag(
    u: &ContractUitvoer,
    tijd: &str,
    opzoek: Opzoek,
) -> String {
    let r = &u.rapport;
    let c = &u.contract;
    let i = &u.gerekend_met;
    let res = &r.resultaat;
    let mut s = String::new();
    let _ = writeln!(s, "# Toetsverslag Bbl - {} ({})\n", i.naam, c.onderdeel);
    let _ = writeln!(s, "Datum uitvoer: {tijd}. Kern: trap-core {} (open-bouwlab), headless volgens assemblagecontract kba-assemblage/1.", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(s, "Invoerbestand: {} (sha256 {}).\n", c.bron_bestand, c.bron_sha256);
    let _ = writeln!(s, "> {}\n", r.disclaimer);

    let _ = writeln!(s, "## Gerekend met\n");
    let _ = writeln!(s, "| invoer | waarde |\n|---|---|");
    let _ = writeln!(s, "| project | {} ({}) |", c.project.project.naam, c.project.project.id);
    let _ = writeln!(s, "| bouwlaag | {} '{}', peil {} mm |", c.laag.code, c.laag.naam, r1(c.laag.peil));
    let _ = writeln!(
        s,
        "| doorbreekt | {} '{}', peil {} mm, vloerpakket {} mm |",
        c.doorbreekt.code,
        c.doorbreekt.naam,
        r1(c.doorbreekt.peil),
        c.doorbreekt.vloerpakket.map_or("-".into(), r1)
    );
    let _ = writeln!(s, "| verdiepingshoogte | {} mm (niveaus.boven - niveaus.onder) |", r1(i.niveaus.boven - i.niveaus.onder));
    let _ = writeln!(s, "| gebruiksfunctie / bouwsoort | {:?} / {:?} |", i.gebruiksfunctie, i.bouwsoort);
    let _ = writeln!(s, "| materiaal (NAA.K.T.) | {} |", c.materiaal);
    let _ = writeln!(s, "| NL-SfB | {} {} |", c.nlsfb.code, c.nlsfb.omschrijving);
    let _ = writeln!(s, "| looplijn | {} mm uit de buitenkant van de buitenboom, hoek {:?} |", r1(i.looplijn.afstand_buitenkant_boom), i.looplijn.hoek);
    let _ = writeln!(
        s,
        "| trapgat | {} |",
        if i.trapgat.is_some() { "uit de invoer" } else { "niet opgegeven: minimaal trapgat uit de trap" }
    );
    let _ = writeln!(s, "| sparingsvolume | {} ({} punten) |\n", c.sparing_bron, c.sparing.len());
    let _ = writeln!(s, "Volledige effectieve invoer: zie bijlage A (JSON).\n");

    let _ = writeln!(s, "## Resultaat\n");
    let _ = writeln!(s, "| grootheid | waarde |\n|---|---|");
    let _ = writeln!(s, "| hoogte | {} mm |", r1(res.hoogte));
    let _ = writeln!(s, "| optreden x optrede | {} x {:.2} mm |", res.optreden, res.optrede);
    let _ = writeln!(s, "| aantreden, aantrede op de looplijn | {} x {:.2} mm |", res.aantreden, res.aantrede_looplijn);
    let _ = writeln!(s, "| loopbreedte | {} mm |", r1(res.loopbreedte));
    let _ = writeln!(s, "| vrije hoogte | {} mm |\n", r1(res.vrije_hoogte));

    let _ = writeln!(s, "## Toetsing\n");
    let _ = writeln!(s, "| regel | oordeel | waarde | grens | bron | artikel_status (corpus) |\n|---|---|---|---|---|---|");
    let mut corpus: Vec<(String, Option<Artikel>)> = Vec::new();
    for g in &r.regels {
        let o = serde_json::to_value(g.oordeel).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
        let bron_id = g.bron_id.split(';').next().unwrap_or("").trim().to_string();
        let mut status = Vec::new();
        for (art, lid) in bbl_artikelen(&g.bron) {
            let gevonden = opzoek(&art, lid.as_deref(), &bron_id);
            let label = match (&lid, &gevonden) {
                (Some(l), Some(a)) => format!("art. {art} lid {l}: {}", a.artikel_status),
                (None, Some(a)) => format!("art. {art}: {}", a.artikel_status),
                (Some(l), None) => format!("art. {art} lid {l}: niet geverifieerd"),
                (None, None) => format!("art. {art}: niet geverifieerd"),
            };
            status.push(label);
            let sleutel = format!("{art}{}", lid.as_deref().map(|l| format!(" lid {l}")).unwrap_or_default());
            if !corpus.iter().any(|(k, _)| *k == sleutel) {
                corpus.push((sleutel, gevonden));
            }
        }
        let status = if status.is_empty() { "geen Bbl-artikel (advies)".to_string() } else { status.join("; ") };
        let _ = writeln!(
            s,
            "| {} | {o} | {} | {} | {} | {status} |",
            g.id,
            g.waarde.map_or("-".into(), |w| format!("{w:.1} {}", g.eenheid)),
            g.grenswaarde.map_or("-".into(), |w| format!("{w:.1} {}", g.eenheid)),
            g.bron.replace('|', "/"),
        );
    }

    let _ = writeln!(s, "\n## Vindplaatsen Bbl (normcorpus kennis-v2)\n");
    let _ = writeln!(s, "| artikel | artikel_status | corpusbestand | toestand | bron_uri |\n|---|---|---|---|---|");
    let mut toestanden = Vec::new();
    for (k, a) in &corpus {
        match a {
            Some(a) => {
                let _ = writeln!(s, "| {k} | {} | {} | {} | {} |", a.artikel_status, a.bestand, a.toestand, a.bron_uri);
                if !toestanden.contains(&a.toestand) {
                    toestanden.push(a.toestand.clone());
                }
            }
            None => {
                let _ = writeln!(s, "| {k} | niet geverifieerd (corpus niet gevonden) | - | - | - |");
            }
        }
    }
    let datum = tijd.get(0..10).unwrap_or(tijd);
    let moment = laatste_wijzigingsmoment(datum);
    for t in &toestanden {
        let oordeel = if t.as_str() < moment.as_str() {
            format!("ouder dan het laatste wijzigingsmoment {moment}: toestand MOGELIJK VEROUDERD, controleer wetten.overheid.nl")
        } else {
            format!("na het laatste wijzigingsmoment {moment} (Bbl wijzigt 1 januari en 1 juli)")
        };
        let _ = writeln!(s, "\nToestand corpus {t}: {oordeel}.");
    }

    let _ = writeln!(s, "\n## Waarschuwingen\n");
    if r.waarschuwingen.is_empty() {
        let _ = writeln!(s, "geen");
    }
    for w in &r.waarschuwingen {
        let _ = writeln!(s, "- {w}");
    }
    let _ = writeln!(s, "\n## Bijlage A - effectieve invoer (gerekend met)\n");
    let _ = writeln!(s, "```json\n{}\n```", serde_json::to_string_pretty(i).unwrap_or_default());
    // plain ASCII (afspraak documenten): niet-ASCII als '?'
    s.chars().map(|c| if c.is_ascii() { c } else { '?' }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artikelen_uit_bron() {
        assert_eq!(
            bbl_artikelen("Bbl art. 4.26 lid 1, tabel 4.26 (geldend); bijlage I"),
            vec![("4.26".to_string(), Some("1".to_string()))]
        );
        assert_eq!(bbl_artikelen("Bbl art. 4.26a en tabel 4.24"), vec![("4.26a".to_string(), None)]);
        assert_eq!(bbl_artikelen("Bbl art. 4.27 (geldend)"), vec![("4.27".to_string(), None)]);
        assert!(bbl_artikelen("Wattjes deel VIII").is_empty());
    }

    #[test]
    fn corpusregel() {
        let t = "---\ngeldig_van: \"2026-09-24\"\n---\n`type: normatief | artikel: 4.26 | lid: 1 | artikel_status: geldend | bron_uri: http://x/4.26`\n`type: normatief | artikel: 4.26 | lid: 2 | artikel_status: vervallen | bron_uri: http://x/4.26b`\n";
        let a = zoek_in_corpus(t, "wet/b.md", "4.26", Some("2")).unwrap();
        assert_eq!(a.artikel_status, "vervallen");
        assert_eq!(a.toestand, "2026-09-24");
        assert_eq!(zoek_in_corpus(t, "f", "4.26", None).unwrap().artikel_status, "geldend");
        assert!(zoek_in_corpus(t, "f", "4.2", None).is_none());
    }

    #[test]
    fn wijzigingsmoment() {
        assert_eq!(laatste_wijzigingsmoment("2026-11-15"), "2026-07-01");
        assert_eq!(laatste_wijzigingsmoment("2027-03-01"), "2027-01-01");
    }
}
