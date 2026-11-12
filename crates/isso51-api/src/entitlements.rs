//! Entitlement-gating voor optionele modules.
//!
//! Een module staat achter een Authentik-groep. De groepsnamen komen binnen via
//! `X-Authentik-Groups` (forward-auth) of via de Bearer-flow en staan in
//! [`OidcClaims::groups`]. Per module is er een aan/uit-schakelaar zodat de
//! module voor iedereen uit kan zonder groepen aan te passen.
//!
//! Omgevingsvariabelen:
//! - `CONSTRUCTIE_ENTITLEMENT_GROUP`: naam van de Authentik-groep (default
//!   [`DEFAULT_CONSTRUCTIE_GROUP`]).
//! - `CONSTRUCTIE_ENABLED`: `false`, `0`, `no` of `off` zet de module voor
//!   iedereen uit (default aan).
//! - `TRAP_ENTITLEMENT_GROUP` / `TRAP_ENABLED`: idem voor de trapontwerper
//!   (default [`DEFAULT_TRAP_GROUP`], aan; O-2026-11-12-01).

use std::env;

use crate::auth::OidcClaims;

/// Default groepsnaam voor de constructiemodule.
pub const DEFAULT_CONSTRUCTIE_GROUP: &str = "openbouwlab-constructie";

/// Naam van de constructiemodule in `/me` -> `entitlements`.
pub const ENTITLEMENT_CONSTRUCTIE: &str = "constructie";

/// Default groepsnaam voor de trapontwerper.
pub const DEFAULT_TRAP_GROUP: &str = "openbouwlab-trap";

/// Naam van de trapontwerper in `/me` -> `entitlements`.
pub const ENTITLEMENT_TRAP: &str = "trap";

/// Entitlement-configuratie.
#[derive(Clone, Debug)]
pub struct Config {
    /// `false` = constructiemodule voor iedereen uit.
    pub constructie_enabled: bool,
    /// Authentik-groep die toegang geeft (exacte match).
    pub constructie_group: String,
    /// `false` = trapontwerper voor iedereen uit.
    pub trap_enabled: bool,
    /// Authentik-groep die toegang geeft tot de trapontwerper (exacte match).
    pub trap_group: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            constructie_enabled: true,
            constructie_group: DEFAULT_CONSTRUCTIE_GROUP.to_string(),
            trap_enabled: true,
            trap_group: DEFAULT_TRAP_GROUP.to_string(),
        }
    }
}

/// Aan/uit en groep van een module uit ruwe waarden (leeg of afwezig = default).
fn module(enabled: Option<&str>, group: Option<&str>, standaard: &str) -> (bool, String) {
    let aan = !matches!(
        enabled.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("false" | "0" | "no" | "off")
    );
    let groep = group
        .map(str::trim)
        .filter(|g| !g.is_empty())
        .unwrap_or(standaard)
        .to_string();
    (aan, groep)
}

impl Config {
    /// Lees `CONSTRUCTIE_ENABLED`, `CONSTRUCTIE_ENTITLEMENT_GROUP`, `TRAP_ENABLED`
    /// en `TRAP_ENTITLEMENT_GROUP` uit de omgeving.
    pub fn from_env() -> Self {
        Self::from_values(
            env::var("CONSTRUCTIE_ENABLED").ok().as_deref(),
            env::var("CONSTRUCTIE_ENTITLEMENT_GROUP").ok().as_deref(),
        )
        .met_trap(
            env::var("TRAP_ENABLED").ok().as_deref(),
            env::var("TRAP_ENTITLEMENT_GROUP").ok().as_deref(),
        )
    }

    /// Bouw de configuratie uit ruwe waarden voor de constructiemodule (leeg of
    /// afwezig = default); de trapontwerper krijgt zijn default.
    pub fn from_values(enabled: Option<&str>, group: Option<&str>) -> Self {
        let (constructie_enabled, constructie_group) =
            module(enabled, group, DEFAULT_CONSTRUCTIE_GROUP);
        Self {
            constructie_enabled,
            constructie_group,
            ..Self::default()
        }
    }

    /// Zet de trapontwerper uit ruwe waarden (leeg of afwezig = default).
    pub fn met_trap(self, enabled: Option<&str>, group: Option<&str>) -> Self {
        let (trap_enabled, trap_group) = module(enabled, group, DEFAULT_TRAP_GROUP);
        Self {
            trap_enabled,
            trap_group,
            ..self
        }
    }
}

/// Mag deze gebruiker de constructiemodule gebruiken?
///
/// Waar als de module aan staat en `claims.groups` de groepsnaam exact bevat.
pub fn constructie_toegestaan(claims: &OidcClaims, config: &Config) -> bool {
    config.constructie_enabled && claims.groups.iter().any(|g| g == &config.constructie_group)
}

/// Mag deze gebruiker de trapontwerper gebruiken?
///
/// Waar als de module aan staat en `claims.groups` de groepsnaam exact bevat.
pub fn trap_toegestaan(claims: &OidcClaims, config: &Config) -> bool {
    config.trap_enabled && claims.groups.iter().any(|g| g == &config.trap_group)
}

/// Lijst met actieve entitlements van deze gebruiker (voor `/me`).
pub fn entitlements_van(claims: &OidcClaims, config: &Config) -> Vec<String> {
    let mut lijst = Vec::new();
    if constructie_toegestaan(claims, config) {
        lijst.push(ENTITLEMENT_CONSTRUCTIE.to_string());
    }
    if trap_toegestaan(claims, config) {
        lijst.push(ENTITLEMENT_TRAP.to_string());
    }
    lijst
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(groepen: &[&str]) -> OidcClaims {
        OidcClaims {
            sub: "tester".to_string(),
            groups: groepen.iter().map(|g| (*g).to_string()).collect(),
            ..OidcClaims::default()
        }
    }

    #[test]
    fn groep_aanwezig_is_toegestaan() {
        let cfg = Config::default();
        assert!(constructie_toegestaan(
            &claims(&["andere", DEFAULT_CONSTRUCTIE_GROUP]),
            &cfg
        ));
        assert_eq!(
            entitlements_van(&claims(&[DEFAULT_CONSTRUCTIE_GROUP]), &cfg),
            vec!["constructie".to_string()]
        );
    }

    #[test]
    fn groep_afwezig_of_deelmatch_is_geweigerd() {
        let cfg = Config::default();
        assert!(!constructie_toegestaan(&claims(&["andere"]), &cfg));
        assert!(!constructie_toegestaan(&claims(&[]), &cfg));
        // Exacte match: geen prefix of hoofdletterverschil.
        assert!(!constructie_toegestaan(
            &claims(&["openbouwlab-constructie-extra", "OpenBouwlab-Constructie"]),
            &cfg
        ));
        assert!(entitlements_van(&claims(&[]), &cfg).is_empty());
    }

    #[test]
    fn uitgeschakeld_weigert_iedereen() {
        let cfg = Config::from_values(Some("false"), None);
        assert!(!cfg.constructie_enabled);
        assert!(!constructie_toegestaan(
            &claims(&[DEFAULT_CONSTRUCTIE_GROUP]),
            &cfg
        ));
    }

    #[test]
    fn trap_eigen_groep_los_van_constructie() {
        let cfg = Config::default();
        assert!(trap_toegestaan(&claims(&[DEFAULT_TRAP_GROUP]), &cfg));
        assert!(!trap_toegestaan(&claims(&[DEFAULT_CONSTRUCTIE_GROUP]), &cfg));
        assert!(!constructie_toegestaan(&claims(&[DEFAULT_TRAP_GROUP]), &cfg));
        assert!(!trap_toegestaan(&claims(&["openbouwlab-trap-extra"]), &cfg));
        assert_eq!(
            entitlements_van(&claims(&[DEFAULT_CONSTRUCTIE_GROUP, DEFAULT_TRAP_GROUP]), &cfg),
            vec!["constructie".to_string(), "trap".to_string()]
        );
        let uit = Config::default().met_trap(Some("off"), Some(" eigen-trap "));
        assert!(!uit.trap_enabled);
        assert_eq!(uit.trap_group, "eigen-trap");
        assert!(!trap_toegestaan(&claims(&["eigen-trap"]), &uit));
        // constructie-instellingen laten de trap ongemoeid
        let c = Config::from_values(Some("false"), None);
        assert!(c.trap_enabled && c.trap_group == DEFAULT_TRAP_GROUP);
    }

    #[test]
    fn env_waarden_worden_geparsed() {
        let cfg = Config::from_values(None, Some("  eigen-groep "));
        assert!(cfg.constructie_enabled);
        assert_eq!(cfg.constructie_group, "eigen-groep");
        assert!(constructie_toegestaan(&claims(&["eigen-groep"]), &cfg));
        assert!(!constructie_toegestaan(
            &claims(&[DEFAULT_CONSTRUCTIE_GROUP]),
            &cfg
        ));
        assert_eq!(
            Config::from_values(Some("OFF"), Some("")).constructie_group,
            DEFAULT_CONSTRUCTIE_GROUP
        );
        assert!(!Config::from_values(Some("0"), None).constructie_enabled);
        assert!(Config::from_values(Some("true"), None).constructie_enabled);
    }
}
