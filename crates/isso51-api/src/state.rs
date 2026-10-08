//! Application state shared across handlers.

use std::sync::Arc;
use std::time::Duration;

/// Default timeout voor de IFC-analyse-sidecar.
const DEFAULT_IFC_ANALYSE_TIMEOUT: Duration = Duration::from_secs(300);

use openaec_cloud::TenantsRegistry;
use sqlx::SqlitePool;

/// Default path to the `ifc-tool` executable inside the Docker container.
const DEFAULT_IFC_TOOL_PATH: &str = "/opt/ifc-tool-venv/bin/ifc-tool";

/// Tool slug used for cloud storage directory mapping.
/// Maps to `calculations/` via `openaec_cloud::container::output_dir_for_tool`.
pub const TOOL_SLUG: &str = "warmteverlies";

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
    /// Map met standaard-coverfoto's per tenant (env `REPORT_DEFAULT_COVER_DIR`).
    pub report_default_cover_dir: Option<std::path::PathBuf>,
    /// Path to the `ifc-tool` CLI for server-side IFC import.
    pub ifc_tool_path: String,
    /// Basis-URL van de private IFC-analyse-sidecar (None = uitgeschakeld).
    pub ifc_analyse_url: Option<String>,
    /// Timeout per request naar de IFC-analyse-sidecar.
    pub ifc_analyse_timeout: Duration,
    /// Multi-tenant cloud storage registry.
    pub tenants: Arc<TenantsRegistry>,
    /// Default tenant slug (fallback when token has no tenant claim).
    pub default_tenant: Option<String>,
    /// Entitlement-configuratie (module-gating via Authentik-groepen).
    pub entitlements: crate::entitlements::Config,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: SqlitePool,
        reports_api_url: Option<String>,
        reports_api_key: Option<String>,
        reports_api_service_token: Option<String>,
        ifc_tool_path: Option<String>,
        tenants: TenantsRegistry,
        default_tenant: Option<String>,
    ) -> Self {
        Self {
            db,
            http_client: reqwest::Client::new(),
            reports_api_url,
            reports_api_key,
            reports_api_service_token,
            report_default_cover_dir: None,
            ifc_tool_path: ifc_tool_path.unwrap_or_else(|| DEFAULT_IFC_TOOL_PATH.to_string()),
            ifc_analyse_url: None,
            ifc_analyse_timeout: DEFAULT_IFC_ANALYSE_TIMEOUT,
            tenants: Arc::new(tenants),
            default_tenant,
            entitlements: crate::entitlements::Config::default(),
        }
    }

    /// Get a [`CloudClient`] for the given tenant slug, or the default tenant.
    ///
    /// Returns `None` if no tenant is configured or the slug is unknown.
    pub fn cloud_client(
        &self,
        tenant_slug: Option<&str>,
    ) -> Option<openaec_cloud::CloudClient> {
        let slug = tenant_slug
            .or(self.default_tenant.as_deref())?;
        let tenant = self.tenants.get(slug)?;
        Some(openaec_cloud::CloudClient::new(tenant, TOOL_SLUG))
    }
}
