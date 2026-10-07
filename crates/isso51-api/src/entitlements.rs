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

use std::env;

use crate::auth::OidcClaims;

/// Default groepsnaam voor de constructiemodule.
pub const DEFAULT_CONSTRUCTIE_GROUP: &str = "openbouwlab-constructie";

/// Naam van de constructiemodule in `/me` -> `entitlements`.
pub const ENTITLEMENT_CONSTRUCTIE: &str = "constructie";

/// Entitlement-configuratie.
#[derive(Clone, Debug)]
pub struct Config {
    /// `false` = constructiemodule voor iedereen uit.
    pub constructie_enabled: bool,
    /// Authentik-groep die toegang geeft (exacte match).
    pub constructie_group: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            constructie_enabled: true,
            constructie_group: DEFAULT_CONSTRUCTIE_GROUP.to_string(),
        }
    }
}

impl Config {
    /// Lees `CONSTRUCTIE_ENABLED` en `CONSTRUCTIE_ENTITLEMENT_GROUP` uit de omgeving.
    pub fn from_env() -> Self {
        Self::from_values(
            env::var("CONSTRUCTIE_ENABLED").ok().as_deref(),
            env::var("CONSTRUCTIE_ENTITLEMENT_GROUP").ok().as_deref(),
        )
    }

    /// Bouw de configuratie uit ruwe waarden (leeg of afwezig = default).
    pub fn from_values(enabled: Option<&str>, group: Option<&str>) -> Self {
        let constructie_enabled = !matches!(
            enabled.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
            Some("false" | "0" | "no" | "off")
        );
        let constructie_group = group
            .map(str::trim)
            .filter(|g| !g.is_empty())
            .unwrap_or(DEFAULT_CONSTRUCTIE_GROUP)
            .to_string();
        Self {
            constructie_enabled,
            constructie_group,
        }
    }
}

/// Mag deze gebruiker de constructiemodule gebruiken?
///
/// Waar als de module aan staat en `claims.groups` de groepsnaam exact bevat.
pub fn constructie_toegestaan(claims: &OidcClaims, config: &Config) -> bool {
    config.constructie_enabled && claims.groups.iter().any(|g| g == &config.constructie_group)
}

/// Lijst met actieve entitlements van deze gebruiker (voor `/me`).
pub fn entitlements_van(claims: &OidcClaims, config: &Config) -> Vec<String> {
    let mut lijst = Vec::new();
    if constructie_toegestaan(claims, config) {
        lijst.push(ENTITLEMENT_CONSTRUCTIE.to_string());
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
