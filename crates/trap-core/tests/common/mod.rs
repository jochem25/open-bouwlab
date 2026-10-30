//! Gedeelde hulp voor de golden tests: referenties van echte projecten staan
//! niet in de publieke repo maar lokaal in `tests/fixtures.local/trap/` (in de
//! root van de repo, gitignored). Ontbreekt een bestand, dan slaat de test over.

use std::path::PathBuf;

/// Inhoud van een lokale fixture, of `None` (met melding) als hij ontbreekt.
pub fn lokaal(naam: &str) -> Option<String> {
    let pad = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures.local/trap").join(naam);
    match std::fs::read_to_string(&pad) {
        Ok(s) => Some(s),
        Err(_) => {
            eprintln!("overgeslagen: lokale fixture {} ontbreekt", pad.display());
            None
        }
    }
}
