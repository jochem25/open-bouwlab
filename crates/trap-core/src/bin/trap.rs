//! CLI van de trapontwerper: invoer-JSON naar IFC4 en toets-JSON.
//!
//! ```text
//! trap ontwerp <invoer.json> [--ifc <uit.ifc>] [--toets <uit.json>] [--tijd <ISO>]
//! trap schema               JSON-schema van de invoer naar stdout
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
        "gebruik:\n  trap ontwerp <invoer.json> [--ifc <uit.ifc>] [--toets <uit.json>] [--tijd <ISO 8601>]\n  trap schema"
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("schema") => {
            print!("{}", trap_core::invoer::schema());
            ExitCode::SUCCESS
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
