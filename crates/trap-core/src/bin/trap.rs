//! CLI van de trapontwerper: invoer-JSON naar IFC4 en toets-JSON.
//!
//! ```text
//! trap ontwerp <invoer.json> [--ifc <uit.ifc>] [--toets <uit.json>] [--tijd <ISO>]
//! trap schema               JSON-schema van de invoer naar stdout
//! trap uitvoer <invoer.json> [--tijd <ISO>]   scherm-uitvoer (toetsing, tekening, IFC's) als JSON
//! trap contract <invoer.json> [--uit <map>] [--tijd <ISO>] [--corpus <kennis-v2>]
//!                           headless volgens het assemblagecontract: <onderdeel>.ifc,
//!                           .toets.json, .toetsverslag.md en .pdf
//! trap contract-schema      JSON-schema van de headless invoer
//! ```
//!
//! Zonder `--ifc`/`--toets` komen de bestanden naast de invoer
//! (`<naam>.ifc`, `<naam>.toets.json`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use trap_core::{ontwerp_from_json, IfcOpties, DISCLAIMER};

fn gebruik() -> ExitCode {
    eprintln!(
        "gebruik:\n  trap ontwerp <invoer.json> [--ifc <uit.ifc>] [--toets <uit.json>] [--tijd <ISO 8601>]\n  trap schema\n  trap uitvoer <invoer.json> [--tijd <ISO 8601>]\n  trap contract <invoer.json> [--uit <map>] [--tijd <ISO 8601>] [--corpus <kennis-v2>]\n  trap contract-schema"
    );
    ExitCode::from(2)
}

/// UTC-tijd als ISO 8601 zonder externe crate (civiele datum uit dagen).
fn nu() -> String {
    let s = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (dag, sec) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    let z = dag + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}", sec / 3600, sec / 60 % 60, sec % 60)
}

fn naast(invoer: &Path, achtervoegsel: &str) -> PathBuf {
    let stam = invoer.file_stem().and_then(|s| s.to_str()).unwrap_or("trap");
    invoer.with_file_name(format!("{stam}{achtervoegsel}"))
}

/// Normcorpus kennis-v2: `--corpus`, anders `%KBA_SHARED%\kennis-v2`.
fn corpus_map(expliciet: Option<PathBuf>) -> Option<PathBuf> {
    expliciet
        .or_else(|| std::env::var_os("KBA_SHARED").map(|s| PathBuf::from(s).join("kennis-v2")))
        .filter(|p| p.is_dir())
}

/// `trap contract`: headless volgens het assemblagecontract.
fn contract(args: &[String]) -> Result<Vec<PathBuf>, String> {
    let invoer = args.first().map(PathBuf::from).ok_or("invoerbestand ontbreekt")?;
    let (mut uit, mut tijd, mut corpus) = (None, None, None);
    let mut i = 1;
    while i < args.len() {
        let w = args.get(i + 1).cloned().ok_or_else(|| format!("waarde ontbreekt bij {}", args[i]))?;
        match args[i].as_str() {
            "--uit" => uit = Some(PathBuf::from(w)),
            "--tijd" => tijd = Some(w),
            "--corpus" => corpus = Some(PathBuf::from(w)),
            o => return Err(format!("onbekende optie {o}")),
        }
        i += 2;
    }
    let tijd = tijd.unwrap_or_else(nu);
    let bron = std::fs::read(&invoer).map_err(|e| format!("kan {} niet lezen: {e}", invoer.display()))?;
    // eerst het schema-veld: een verkeerd bestand (bv. project.json) krijgt dan
    // een duidelijke melding in plaats van een fout in een binnenveld
    let ruw: serde_json::Value =
        serde_json::from_slice(&bron).map_err(|e| format!("{}: geen geldige JSON: {e}", invoer.display()))?;
    let schema = ruw.get("schema").and_then(|s| s.as_str()).unwrap_or("(geen)");
    if schema != trap_core::contract::INVOER_SCHEMA {
        return Err(format!(
            "{}: schema '{schema}', verwacht '{}' (headless trapinvoer)",
            invoer.display(),
            trap_core::contract::INVOER_SCHEMA
        ));
    }
    let ci: trap_core::contract::ContractInvoer =
        serde_json::from_value(ruw).map_err(|e| format!("{}: {e}", invoer.display()))?;
    let project = match &ci.project {
        trap_core::contract::ProjectBron::Inline(p) => (**p).clone(),
        trap_core::contract::ProjectBron::Pad(p) => {
            let pad = invoer.parent().unwrap_or(Path::new(".")).join(p);
            let tekst = std::fs::read(&pad).map_err(|e| format!("kan project {} niet lezen: {e}", pad.display()))?;
            serde_json::from_slice(&tekst).map_err(|e| format!("{}: {e}", pad.display()))?
        }
    };
    let naam = invoer.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
    let u = trap_core::ontwerp_contract(&ci, &project, &naam, &bron, &tijd).map_err(|e| e.to_string())?;

    let corpus = corpus_map(corpus);
    let opzoek = |art: &str, lid: Option<&str>, bron_id: &str| {
        let map = corpus.as_ref()?;
        if bron_id.is_empty() || bron_id == "-" {
            return None;
        }
        // het corpus knipt grote bestanden op (bbl__afdeling-4-2.md ->
        // bbl__afdeling-4-2-paragraaf-*.md, 16-11-2026): zoek in het hele
        // bestand of anders in de delen
        let (dir, stam) = match bron_id.rsplit_once('/') {
            Some((d, s)) => (d, s),
            None => ("", bron_id),
        };
        let namen: Vec<String> = std::fs::read_dir(map.join(dir))
            .ok()?
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .collect();
        trap_core::verslag::corpus_bestanden(&namen, stam).into_iter().find_map(|n| {
            let rel = if dir.is_empty() { n } else { format!("{dir}/{n}") };
            let tekst = std::fs::read_to_string(map.join(&rel)).ok()?;
            trap_core::verslag::zoek_in_corpus(&tekst, &rel, art, lid)
        })
    };
    let verslag = trap_core::verslag::toetsverslag(&u, &tijd, &opzoek);
    let kader = trap_core::pdf::Kader {
        titel: format!("{} ({})", u.gerekend_met.naam, u.contract.onderdeel),
        regels: vec![
            format!("project {} ({})", u.contract.project.project.naam, u.contract.project.project.id),
            format!("bouwlaag {} '{}', peil {:.0}", u.contract.laag.code, u.contract.laag.naam, u.contract.laag.peil),
            format!("doorbreekt {} '{}', peil {:.0}", u.contract.doorbreekt.code, u.contract.doorbreekt.naam, u.contract.doorbreekt.peil),
            format!("materiaal {}, NL-SfB {}", u.contract.materiaal, u.contract.nlsfb.code),
            format!("sparing: {}", u.contract.sparing_bron),
            format!("uitvoer {tijd}, invoer {naam}"),
        ],
    };
    let pdf = trap_core::pdf::tekening_pdf(&u.tekening, &u.rapport, Some(&u.contract.sparing), &kader, ci.referentie.as_ref());

    let map = uit.unwrap_or_else(|| invoer.parent().unwrap_or(Path::new(".")).to_path_buf());
    std::fs::create_dir_all(&map).map_err(|e| format!("kan {} niet maken: {e}", map.display()))?;
    let stam = map.join(&u.bestandsnaam);
    let toets = serde_json::to_string_pretty(&u.rapport).map_err(|e| e.to_string())? + "\n";
    let set = [
        (".ifc", u.ifc.into_bytes()),
        (".toets.json", toets.into_bytes()),
        (".toetsverslag.md", verslag.into_bytes()),
        (".pdf", pdf),
    ];
    let paden: Vec<(PathBuf, PathBuf)> = set
        .iter()
        .map(|(ext, _)| {
            let pad = PathBuf::from(format!("{}{ext}", stam.display()));
            (PathBuf::from(format!("{}.tmp", pad.display())), pad)
        })
        .collect();
    schrijf_set(&paden, set.iter().map(|(_, b)| b.as_slice()))
}

/// Schrijf de set als geheel: eerst ALLE .tmp-bestanden, pas daarna hernoemen.
/// Faalt een schrijfactie, dan worden de .tmp's opgeruimd en blijft de vorige
/// revisie compleet staan (review 16-11: geen mengsel van twee revisies).
fn schrijf_set<'a>(paden: &[(PathBuf, PathBuf)], inhoud: impl Iterator<Item = &'a [u8]>) -> Result<Vec<PathBuf>, String> {
    let ruim = |tot: usize| {
        for (tmp, _) in &paden[..tot] {
            let _ = std::fs::remove_file(tmp);
        }
    };
    for (i, ((tmp, _), b)) in paden.iter().zip(inhoud).enumerate() {
        if let Err(e) = std::fs::write(tmp, b) {
            ruim(i + 1);
            return Err(format!("kan {} niet schrijven: {e}", tmp.display()));
        }
    }
    for (i, (tmp, pad)) in paden.iter().enumerate() {
        if let Err(e) = std::fs::rename(tmp, pad) {
            // de eerdere zijn al vervangen; de rest opruimen en dat melden
            for (t, _) in &paden[i..] {
                let _ = std::fs::remove_file(t);
            }
            return Err(format!(
                "kan {} niet vervangen ({e}); {} van {} bestanden zijn al de nieuwe revisie",
                pad.display(),
                i,
                paden.len()
            ));
        }
    }
    Ok(paden.iter().map(|(_, p)| p.clone()).collect())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("contract") => match contract(&args[1..]) {
            Ok(paden) => {
                for p in paden {
                    println!("{}", p.display());
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        Some("contract-schema") => {
            print!("{}", trap_core::contract::schema());
            ExitCode::SUCCESS
        }
        Some("schema") => {
            print!("{}", trap_core::invoer::schema());
            ExitCode::SUCCESS
        }
        Some("uitvoer") => {
            let Some(pad) = args.get(1) else {
                return gebruik();
            };
            let tijd = match (args.get(2).map(String::as_str), args.get(3)) {
                (Some("--tijd"), Some(t)) => t.clone(),
                (None, _) => nu(),
                _ => return gebruik(),
            };
            let uit = std::fs::read_to_string(pad)
                .map_err(|e| format!("kan {pad} niet lezen: {e}"))
                .and_then(|j| serde_json::from_str::<trap_core::Invoer>(&j).map_err(|e| e.to_string()))
                .and_then(|i| trap_core::ontwerp_uitvoer(&i, &tijd).map_err(|e| e.to_string()))
                .and_then(|u| serde_json::to_string(&u).map_err(|e| e.to_string()));
            match uit {
                Ok(j) => {
                    println!("{j}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("ontwerp") => {
            let Some(invoer) = args.get(1).map(PathBuf::from) else {
                return gebruik();
            };
            let (mut ifc, mut toets, mut tijd) = (None, None, None);
            let mut i = 2;
            while i < args.len() {
                let waarde = args.get(i + 1).cloned();
                match (args[i].as_str(), waarde) {
                    ("--ifc", Some(w)) => ifc = Some(PathBuf::from(w)),
                    ("--toets", Some(w)) => toets = Some(PathBuf::from(w)),
                    ("--tijd", Some(w)) => tijd = Some(w),
                    _ => return gebruik(),
                }
                i += 2;
            }
            let ifc = ifc.unwrap_or_else(|| naast(&invoer, ".ifc"));
            let toets = toets.unwrap_or_else(|| naast(&invoer, ".toets.json"));
            let json = match std::fs::read_to_string(&invoer) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("kan {} niet lezen: {e}", invoer.display());
                    return ExitCode::FAILURE;
                }
            };
            let opties = IfcOpties {
                tijd: tijd.unwrap_or_else(nu),
                bestandsnaam: ifc.file_name().and_then(|s| s.to_str()).unwrap_or("").into(),
                disclaimer: Some(DISCLAIMER.into()),
                ..Default::default()
            };
            match ontwerp_from_json(&json, &opties) {
                Ok((tekst_ifc, tekst_toets)) => {
                    for (pad, inhoud) in [(&ifc, tekst_ifc), (&toets, tekst_toets)] {
                        if let Err(e) = std::fs::write(pad, inhoud) {
                            eprintln!("kan {} niet schrijven: {e}", pad.display());
                            return ExitCode::FAILURE;
                        }
                    }
                    println!("{}\n{}", ifc.display(), toets.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => gebruik(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Faalt een bestand van de set, dan blijft de vorige revisie compleet
    /// staan en blijft er geen .tmp achter (review 16-11).
    #[test]
    fn set_is_alles_of_niets() {
        let d = std::env::temp_dir().join(format!("trap-set-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let p = |n: &str| (d.join(format!("{n}.tmp")), d.join(n));
        std::fs::write(d.join("a"), b"oud").unwrap();
        // derde doel in een map die niet bestaat: schrijven faalt
        let paden = vec![p("a"), p("b"), (d.join("nee").join("c.tmp"), d.join("nee").join("c"))];
        let r = schrijf_set(&paden, [b"nieuw".as_slice(), b"x", b"y"].into_iter());
        assert!(r.is_err());
        assert_eq!(std::fs::read(d.join("a")).unwrap(), b"oud");
        assert!(!d.join("b").exists());
        let rest: Vec<_> = std::fs::read_dir(&d).unwrap().filter_map(|e| e.ok()).map(|e| e.file_name()).collect();
        assert_eq!(rest, vec![std::ffi::OsString::from("a")], "geen .tmp over");
        let ok = schrijf_set(&paden[..2], [b"nieuw".as_slice(), b"x"].into_iter()).unwrap();
        assert_eq!(ok.len(), 2);
        assert_eq!(std::fs::read(d.join("a")).unwrap(), b"nieuw");
        let _ = std::fs::remove_dir_all(&d);
    }
}
