//! Server configuration loaded from environment variables.

use std::env;
use std::time::Duration;

/// Default timeout (seconden) voor de IFC-analyse-sidecar.
const DEFAULT_IFC_ANALYSE_TIMEOUT_S: u64 = 300;

/// API route prefix.
pub const API_PREFIX: &str = "/api/v1";

/// Server configuration.
pub struct Config {
    pub port: u16,
    pub database_url: String,
    /// Directory containing static frontend files (SPA). When set, the server
    /// serves these files as a fallback for non-API routes.
    pub static_dir: Option<String>,
    /// Base URL for the OpenAEC Reports API (e.g. `https://reports.openaec.org`).
    pub reports_api_url: Option<String>,
    /// Legacy shared secret for the OpenAEC Reports API (X-API-Key).
    ///
    /// Deprecated per 2026-04-20 — vervangen door `reports_api_service_token`
    /// (Authentik Bearer). Blijft ondersteund tot Caddy bypass volledig weg is.
    pub reports_api_key: Option<String>,
    /// Authentik service-token (`svc-warmteverlies`) voor backend-to-backend
    /// calls naar de Reports API. Wordt als `Authorization: Bearer <token>`
    /// gestuurd. Vervangt de legacy shared-secret X-API-Key bypass.
    pub reports_api_service_token: Option<String>,
    /// Map met standaard-coverfoto's per organisatie (`<id>.jpg` of `<id>.png`).
    ///
    /// Env `REPORT_DEFAULT_COVER_DIR`. Bevat een rapport geen `cover.image`, dan
    /// vult de rapport-proxy die aan uit dit bestand. Staat bewust buiten de
    /// (publieke) repo; zie `crate::handlers::report`.
    pub report_default_cover_dir: Option<String>,
    /// Basis-URL van de private IFC-analyse-sidecar (env `IFC_ANALYSE_URL`,
    /// leeg = niet beschikbaar).
    pub ifc_analyse_url: Option<String>,
    /// Optioneel Bearer-token voor de sidecar (env `IFC_ANALYSE_TOKEN`).
    /// Nooit loggen.
    pub ifc_analyse_token: Option<String>,
    /// Timeout per analyse-request (env `IFC_ANALYSE_TIMEOUT_S`, default 300).
    pub ifc_analyse_timeout: Duration,
    /// Entitlement-gating voor de constructiemodule.
    ///
    /// Env `CONSTRUCTIE_ENTITLEMENT_GROUP`: Authentik-groep die toegang geeft
    /// (default `openbouwlab-constructie`, exacte match).
    /// Env `CONSTRUCTIE_ENABLED`: `false`/`0`/`no`/`off` zet de module voor
    /// iedereen uit (default aan). Zie `crate::entitlements`.
    pub entitlements: crate::entitlements::Config,
    /// Pad naar het organisatiebestand (env `ORGANISATIES_CONFIG`). Niet gezet
    /// = geen organisaties (rapporten geven dan 403 `geen_organisatie`).
    pub organisaties_config: Option<String>,
}

impl Config {
    /// Load configuration from environment variables with sensible defaults.
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3001),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://isso51.db?mode=rwc".to_string()),
            static_dir: env::var("STATIC_DIR").ok().filter(|s| !s.is_empty()),
            reports_api_url: env::var("REPORTS_API_URL")
                .ok()
                .filter(|s| !s.is_empty()),
            reports_api_key: env::var("REPORTS_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
            reports_api_service_token: env::var("REPORTS_API_SERVICE_TOKEN")
                .ok()
                .filter(|s| !s.is_empty()),
            report_default_cover_dir: env::var("REPORT_DEFAULT_COVER_DIR")
                .ok()
                .filter(|s| !s.is_empty()),
            ifc_analyse_url: env::var("IFC_ANALYSE_URL")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            ifc_analyse_token: env::var("IFC_ANALYSE_TOKEN")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            ifc_analyse_timeout: Duration::from_secs(
                env::var("IFC_ANALYSE_TIMEOUT_S")
                    .ok()
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(DEFAULT_IFC_ANALYSE_TIMEOUT_S),
            ),
            entitlements: crate::entitlements::Config::from_env(),
            organisaties_config: env::var(crate::organisatie::ENV_ORGANISATIES_CONFIG)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        }
    }
}
