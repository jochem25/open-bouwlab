//! Application state shared across handlers.

use std::sync::Arc;
use std::time::Duration;

/// Default timeout voor de IFC-analyse-sidecar.
const DEFAULT_IFC_ANALYSE_TIMEOUT: Duration = Duration::from_secs(300);

use sqlx::SqlitePool;

use crate::organisatie::Organisaties;

/// Shared application state injected into handlers via Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub http_client: reqwest::Client,
    pub reports_api_url: Option<String>,
    /// Legacy shared-secret key (X-API-Key). Deprecated, vervangen door
    /// `reports_api_service_token`. Blijft ondersteund tot Caddy bypass weg is.
    pub reports_api_key: Option<String>,
    /// Authentik service-token voor backend-to-backend reports calls.
    pub reports_api_service_token: Option<String>,
    /// Map met standaard-coverfoto's per organisatie (env `REPORT_DEFAULT_COVER_DIR`).
    pub report_default_cover_dir: Option<std::path::PathBuf>,
    /// Basis-URL van de private IFC-analyse-sidecar (None = uitgeschakeld).
    pub ifc_analyse_url: Option<String>,
    /// Optioneel Bearer-token voor de IFC-analyse-sidecar. Nooit loggen.
    pub ifc_analyse_token: Option<String>,
    /// Timeout per request naar de IFC-analyse-sidecar.
    pub ifc_analyse_timeout: Duration,
    /// Entitlement-configuratie (module-gating via Authentik-groepen).
    pub entitlements: crate::entitlements::Config,
    /// Organisaties (gebruiker -> organisatie -> huisstijl), env `ORGANISATIES_CONFIG`.
    pub organisaties: Arc<Organisaties>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: SqlitePool,
        reports_api_url: Option<String>,
        reports_api_key: Option<String>,
        reports_api_service_token: Option<String>,
    ) -> Self {
        Self {
            db,
            http_client: reqwest::Client::new(),
            reports_api_url,
            reports_api_key,
            reports_api_service_token,
            report_default_cover_dir: None,
            ifc_analyse_url: None,
            ifc_analyse_token: None,
            ifc_analyse_timeout: DEFAULT_IFC_ANALYSE_TIMEOUT,
            entitlements: crate::entitlements::Config::default(),
            organisaties: Arc::new(Organisaties::default()),
        }
    }
}
