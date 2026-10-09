//! CORS layer builder.
//!
//! De toegestane origins komen uit een bron: de `cors_origins` van alle
//! organisaties in het organisatiebestand (zie `crate::organisatie`), plus de
//! lokale dev-origins ([`DEV_ORIGINS`]) zolang `OPENAEC_ENV` niet `production`
//! is. Er is geen tweede lijst (de vroegere `CORS_ORIGINS`-env en de
//! `tenant.yaml`-lezer zijn vervallen per 09-10-2026).
//!
//! ## Semantiek
//!
//! - **Credentials-safe:** `allow_credentials(true)` in combinatie met
//!   `AllowOrigin::predicate` echo't exact het Origin-header i.p.v. `*`.
//! - **Lege set:** permissive `Any` zonder credentials (zelfde gedrag als
//!   voorheen zonder configuratie). De webapp draait same-origin en heeft
//!   CORS niet nodig.
//! - **Reload:** alleen bij backend-startup.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use axum::http::{header, HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Lokale origins van de Vite-devserver en Tauri-dev.
pub const DEV_ORIGINS: [&str; 2] = ["http://localhost:5173", "http://localhost:1420"];

/// Toegestane origins: organisatie-origins plus, buiten productie, de dev-origins.
pub fn origins(organisatie_origins: HashSet<String>, include_dev: bool) -> HashSet<String> {
    let mut set = organisatie_origins;
    if include_dev {
        set.extend(DEV_ORIGINS.iter().map(|o| (*o).to_string()));
    }
    set
}

// ---------------------------------------------------------------------------
// Origin validation
// ---------------------------------------------------------------------------
/// Valideer een enkele origin-string .
///
/// Regels:
/// - Protocol `http://` of `https://`.
/// - Geen trailing slash.
/// - Geen wildcards.
/// - Lowercase.
///
/// Returns `true` als de origin geldig is. Bij `false` wordt een warning
/// gelogd door de caller met tenant-context.
pub fn is_valid_origin(origin: &str) -> bool {
    if origin.is_empty() {
        return false;
    }
    if !(origin.starts_with("http://") || origin.starts_with("https://")) {
        return false;
    }
    if origin.ends_with('/') {
        return false;
    }
    if origin.contains('*') {
        return false;
    }
    if origin != origin.to_lowercase() {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// CorsLayer builder
// ---------------------------------------------------------------------------

/// Custom forward_auth headers die door de frontend gestuurd mogen
/// worden. Blijft in sync met de lijst in `auth.rs` en met de Caddy-config.
fn custom_auth_headers() -> Vec<HeaderName> {
    vec![
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        HeaderName::from_static("x-authentik-username"),
        HeaderName::from_static("x-authentik-email"),
        HeaderName::from_static("x-authentik-name"),
        HeaderName::from_static("x-authentik-uid"),
        HeaderName::from_static("x-authentik-groups"),
        HeaderName::from_static("x-authentik-meta-company"),
        HeaderName::from_static("x-authentik-meta-jobtitle"),
        HeaderName::from_static("x-authentik-meta-phone"),
        HeaderName::from_static("x-authentik-meta-regnumber"),
    ]
}

/// Bouw een `CorsLayer` rondom de gegeven origin-set.
///
/// - `origins` niet-leeg -> `AllowOrigin::predicate` met exact-match tegen
///   de set; `allow_credentials(true)` echo't het Origin-header bij match.
/// - `origins` leeg -> warning + `AllowOrigin::any()` zonder credentials.
pub fn build_cors_layer(origins: HashSet<String>) -> CorsLayer {
    let effective: HashSet<String> = origins
        .into_iter()
        .filter(|o| {
            let ok = is_valid_origin(o);
            if !ok {
                tracing::warn!(origin = %o, "CORS-origin ongeldig - overgeslagen");
            }
            ok
        })
        .collect();

    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
        Method::PATCH,
    ];
    let allow_headers = custom_auth_headers();

    if effective.is_empty() {
        tracing::warn!("CORS: geen origins - gebruik permissive Any (credentials uit)");
        return CorsLayer::new()
            .allow_origin(AllowOrigin::any())
            .allow_methods(methods)
            .allow_headers(allow_headers)
            .max_age(Duration::from_secs(600));
    }

    tracing::info!(count = effective.len(), "CORS layer: {} origins", effective.len());

    let allowed = Arc::new(effective);
    let allowed_for_closure = Arc::clone(&allowed);
    let predicate = AllowOrigin::predicate(move |origin: &HeaderValue, _parts| {
        // HeaderValue → &str; invalide UTF-8 origin = geen match.
        match origin.to_str() {
            Ok(s) => allowed_for_closure.contains(s),
            Err(_) => false,
        }
    });

    CorsLayer::new()
        .allow_origin(predicate)
        .allow_methods(methods)
        .allow_headers(allow_headers)
        .allow_credentials(true)
        .max_age(Duration::from_secs(600))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_uit_organisaties_plus_dev() {
        let org: HashSet<String> = ["https://app.example.org".to_string()].into();
        let prod = origins(org.clone(), false);
        assert_eq!(prod, org);
        let dev = origins(org, true);
        assert!(dev.contains("https://app.example.org"));
        assert!(dev.contains("http://localhost:5173"));
        assert!(dev.contains("http://localhost:1420"));
    }

    #[test]
    fn build_cors_layer_paden_panicen_niet() {
        let _leeg = build_cors_layer(HashSet::new());
        let _met = build_cors_layer(origins(HashSet::new(), true));
        let _ongeldig = build_cors_layer(["geen-origin".to_string()].into());
    }

    #[test]
    fn test_is_valid_origin_rules() {
        assert!(is_valid_origin("https://report.open-aec.com"));
        assert!(is_valid_origin("http://localhost:5173"));
        assert!(is_valid_origin("http://127.0.0.1:5173"));

        assert!(!is_valid_origin(""));
        assert!(!is_valid_origin("report.open-aec.com"));
        assert!(!is_valid_origin("ftp://report.open-aec.com"));
        assert!(!is_valid_origin("https://report.open-aec.com/"));
        assert!(!is_valid_origin("https://*.open-aec.com"));
        assert!(!is_valid_origin("HTTPS://Report.Open-AEC.COM"));
    }
}
