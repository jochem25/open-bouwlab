//! Report generation proxy handler.
//!
//! Forwards report JSON to the OpenAEC Reports API via Authentik service-token
//! (``svc-warmteverlies``) met ``X-Original-Tenant`` header voor on-behalf-of
//! user-tenant context.
//!
//! Auth vereisten:
//! - Caller moet via forward_auth (AuthClaims) authenticated zijn
//! - Upstream-call gebruikt service-token Bearer als geconfigureerd
//! - Fallback: legacy X-API-Key als service-token niet beschikbaar is
//!
//! Standaard-coverfoto: bevat het rapport geen `cover.image`, dan vult de
//! proxy die aan uit `REPORT_DEFAULT_COVER_DIR/<tenant>.{jpg,png}`. Zo werkt
//! een sjabloon met een verplichte foto (KBA) ook zonder upload, zonder dat
//! het bedrijfsbeeld in deze publieke repo staat. Geen bestand voor de
//! tenant: rapport gaat ongewijzigd door en een afkeuring van de Reports API
//! komt met de oorspronkelijke tekst bij de gebruiker.

use std::path::Path;
use std::time::Duration;

use base64::Engine;
use serde_json::Value;

use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};

use crate::auth::{AuthClaims, OidcClaims};
use crate::error::ApiError;
use crate::state::AppState;

/// POST /report/generate — proxy report generation to OpenAEC Reports API.
///
/// Auth chain:
/// 1. `AuthClaims` extractor (forward_auth) valideert de caller en levert user-tenant.
/// 2. Upstream call gebruikt `REPORTS_API_SERVICE_TOKEN` (Authentik ak-*) als
///    primary auth methode. De user-tenant wordt doorgegeven als
///    `X-Original-Tenant` header zodat reports de juiste tenant-templates kiest.
/// 3. Fallback voor transitie: als service-token niet geconfigureerd is maar
///    `REPORTS_API_KEY` wel, stuur die als `X-API-Key` (legacy Caddy bypass).
pub async fn generate_report(
    State(state): State<AppState>,
    AuthClaims(claims): AuthClaims,
    body: String,
) -> Result<Response, ApiError> {
    proxy_report(&state, &claims, body).await
}

/// Stuur rapport-JSON door naar de Reports API en geef de PDF terug.
///
/// Gedeeld door `/report/generate` en de constructiemodule; het gedrag
/// (service-token, tenant-header, foutmapping) is identiek.
pub(crate) async fn proxy_report(
    state: &AppState,
    claims: &OidcClaims,
    body: String,
) -> Result<Response, ApiError> {
    let base_url = state.reports_api_url.as_deref().ok_or_else(|| {
        ApiError::ServiceUnavailable(
            "Rapportgeneratie is niet geconfigureerd (REPORTS_API_URL ontbreekt)".to_string(),
        )
    })?;

    let url = format!("{}/api/generate/v2", base_url.trim_end_matches('/'));

    let mut req = state
        .http_client
        .post(&url)
        .header(header::CONTENT_TYPE.as_str(), "application/json")
        .timeout(Duration::from_secs(30));

    // Primair: Authentik service-token (Bearer ak-*) + X-Original-Tenant
    if let Some(token) = state.reports_api_service_token.as_deref() {
        req = req.header(
            header::AUTHORIZATION.as_str(),
            format!("Bearer {token}"),
        );
        if let Some(tenant) = claims.tenant.as_deref() {
            req = req.header("X-Original-Tenant", tenant);
        } else {
            tracing::warn!(
                user = %claims.sub,
                "geen tenant-claim: rapport gebruikt de tenant van het service-account"
            );
        }
    } else if let Some(api_key) = state.reports_api_key.as_deref() {
        // Legacy fallback: X-API-Key (Caddy bypass) — wordt verwijderd
        // zodra service-token overal werkt.
        req = req.header("X-API-Key", api_key);
    }

    let tenant = claims.tenant.clone();
    let dir = state.report_default_cover_dir.clone();
    let body = tokio::task::spawn_blocking(move || {
        vul_standaard_cover(body, tenant.as_deref(), dir.as_deref())
    })
    .await
    .map_err(|e| ApiError::Internal(format!("standaard-coverfoto: {e}")))?;

    let upstream = req.body(body).send().await.map_err(|e| {
        tracing::error!("Reports API request failed: {e}");
        ApiError::ReportService(format!("Rapport service niet bereikbaar: {e}"))
    })?;

    if !upstream.status().is_success() {
        let status = upstream.status();
        let detail = upstream.text().await.unwrap_or_default();
        tracing::error!("Reports API returned {status}: {detail}");
        if is_inhoudelijke_afkeuring(status) {
            return Err(ApiError::ReportRejected {
                status,
                detail: format!(
                    "Rapport afgekeurd door de rapportengine: {}",
                    upstream_detail(&detail)
                ),
            });
        }
        return Err(ApiError::ReportService(format!(
            "Rapport generatie mislukt ({status}): {detail}"
        )));
    }

    let pdf_bytes = upstream.bytes().await.map_err(|e| {
        tracing::error!("Failed to read report PDF: {e}");
        ApiError::ReportService("Fout bij ophalen van rapport PDF".to_string())
    })?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"rapport.pdf\"",
            ),
        ],
        pdf_bytes,
    )
        .into_response())
}

/// Maximale grootte van een standaard-coverfoto (bytes, ongecodeerd).
const MAX_COVER_BYTES: u64 = 5 * 1024 * 1024;

/// Vul `cover.image` aan met de standaardfoto van de tenant als die ontbreekt.
///
/// Laat de body ongewijzigd als er al een afbeelding is, als er geen tenant of
/// map is, als het bestand ontbreekt of als de body geen JSON-object is (dan
/// keurt de Reports API hem zelf af).
pub(crate) fn vul_standaard_cover(body: String, tenant: Option<&str>, dir: Option<&Path>) -> String {
    let (Some(tenant), Some(dir)) = (tenant, dir) else {
        return body;
    };
    if !is_veilige_slug(tenant) {
        tracing::warn!(tenant, "tenant-naam ongeschikt als bestandsnaam: geen standaard-coverfoto");
        return body;
    }
    let Ok(mut json) = serde_json::from_str::<Value>(&body) else {
        return body;
    };
    let Some(root) = json.as_object_mut() else {
        return body;
    };
    let heeft_foto = root
        .get("cover")
        .and_then(|c| c.get("image"))
        .is_some_and(|i| !i.is_null());
    if heeft_foto {
        return body;
    }
    let Some(afbeelding) = lees_standaard_cover(dir, tenant) else {
        tracing::info!(tenant, "geen standaard-coverfoto voor tenant");
        return body;
    };
    let cover = root
        .entry("cover")
        .or_insert_with(|| Value::Object(Default::default()));
    if !cover.is_object() {
        // `cover: null` of een andere vorm: vervangen, anders zou de foto stil wegvallen.
        *cover = Value::Object(Default::default());
    }
    if let Some(c) = cover.as_object_mut() {
        c.insert("image".to_string(), afbeelding);
    }
    json.to_string()
}

/// Lees `<dir>/<tenant>.jpg` of `.png` als base64-afbeelding voor de Reports API.
fn lees_standaard_cover(dir: &Path, tenant: &str) -> Option<Value> {
    for (ext, media_type) in [("jpg", "image/jpeg"), ("jpeg", "image/jpeg"), ("png", "image/png")] {
        let pad = dir.join(format!("{tenant}.{ext}"));
        let Ok(meta) = std::fs::metadata(&pad) else {
            continue;
        };
        if meta.len() > MAX_COVER_BYTES {
            tracing::error!(pad = %pad.display(), "standaard-coverfoto te groot; overgeslagen");
            return None;
        }
        match std::fs::read(&pad) {
            Ok(bytes) => {
                return Some(serde_json::json!({
                    "data": base64::engine::general_purpose::STANDARD.encode(bytes),
                    "media_type": media_type,
                    "filename": format!("standaard-{tenant}.{ext}"),
                }));
            }
            Err(e) => {
                tracing::error!(pad = %pad.display(), "standaard-coverfoto onleesbaar: {e}");
                return None;
            }
        }
    }
    None
}

/// Alleen `[a-z0-9_-]`: de tenant wordt een bestandsnaam.
fn is_veilige_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// Haal de leesbare tekst uit een foutbody van de Reports API.
///
/// FastAPI geeft `{"detail": "..."}` of, bij schemafouten, `{"detail": [{"msg": ...}]}`.
/// Anders de ruwe tekst, ingekort.
pub(crate) fn upstream_detail(body: &str) -> String {
    const GEEN_TOELICHTING: &str = "geen toelichting ontvangen";
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        // Geen JSON (bijv. een HTML-foutpagina van een proxy): niet tonen.
        return GEEN_TOELICHTING.to_string();
    };
    let tekst = match json.get("detail") {
        Some(Value::String(s)) => s.clone(),
        // Alleen `msg`: een schemafout bevat ook `input`, mogelijk de hele afbeelding.
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|i| i.get("msg").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("; "),
        _ => String::new(),
    };
    let tekst = tekst.trim();
    if tekst.is_empty() {
        return GEEN_TOELICHTING.to_string();
    }
    kap_af(tekst, MAX_DETAIL_TEKENS)
}

/// Maximale lengte van een doorgegeven fouttekst (tekens).
const MAX_DETAIL_TEKENS: usize = 500;

/// Kap `tekst` af op `max` tekens (op een tekengrens).
fn kap_af(tekst: &str, max: usize) -> String {
    match tekst.char_indices().nth(max) {
        Some((i, _)) => format!("{}...", &tekst[..i]),
        None => tekst.to_string(),
    }
}

/// 400, 413 en 422 zeggen iets over het rapport zelf en gaan door naar de
/// gebruiker. Andere 4xx (401/403 service-token, 404 verkeerde URL, 429) zijn
/// configuratie- of serverfouten: die blijven 502, anders leest de frontend
/// een 401/403 als een verlopen sessie van de gebruiker.
fn is_inhoudelijke_afkeuring(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 400 | 413 | 422)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use axum::http::StatusCode;

    /// Verse map met een standaardfoto `kba.jpg` (inhoud is willekeurig).
    fn cover_map(naam: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("isso51-cover-{naam}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("map");
        std::fs::write(dir.join("kba.jpg"), b"jpeg-bytes").expect("foto");
        dir
    }

    fn cover_image(body: &str) -> Value {
        serde_json::from_str::<Value>(body).expect("json")["cover"]["image"].clone()
    }

    #[test]
    fn zonder_upload_vult_standaardfoto_van_tenant_aan() {
        let dir = cover_map("zonder");
        let body = r#"{"template":"standaard_rapport","cover":{"subtitle":"x"}}"#.to_string();
        let uit = vul_standaard_cover(body, Some("kba"), Some(&dir));
        let image = cover_image(&uit);
        assert_eq!(image["media_type"], "image/jpeg");
        assert_eq!(image["data"], base64::engine::general_purpose::STANDARD.encode(b"jpeg-bytes"));
        assert_eq!(serde_json::from_str::<Value>(&uit).expect("json")["cover"]["subtitle"], "x");

        // Ook zonder cover-object, met `cover: null` en met `image: null`.
        for body in [
            r#"{"template":"t"}"#,
            r#"{"template":"t","cover":null}"#,
            r#"{"template":"t","cover":{"image":null}}"#,
        ] {
            let uit = vul_standaard_cover(body.to_string(), Some("kba"), Some(&dir));
            assert_eq!(cover_image(&uit)["media_type"], "image/jpeg", "{body}");
        }
    }

    #[test]
    fn met_upload_blijft_body_ongewijzigd() {
        let dir = cover_map("met");
        let body = r#"{"cover":{"image":{"data":"QUJD","media_type":"image/png"}}}"#.to_string();
        assert_eq!(vul_standaard_cover(body.clone(), Some("kba"), Some(&dir)), body);
    }

    #[test]
    fn geen_standaard_voor_tenant_of_geen_map_laat_body_ongewijzigd() {
        let dir = cover_map("geen");
        let body = r#"{"cover":{}}"#.to_string();
        assert_eq!(vul_standaard_cover(body.clone(), Some("ander"), Some(&dir)), body);
        assert_eq!(vul_standaard_cover(body.clone(), None, Some(&dir)), body);
        assert_eq!(vul_standaard_cover(body.clone(), Some("kba"), None), body);
        // Tenant wordt een bestandsnaam: padtekens worden geweigerd.
        assert_eq!(vul_standaard_cover(body.clone(), Some("../kba"), Some(&dir)), body);
        // Geen JSON: ongewijzigd, de Reports API keurt hem af.
        assert_eq!(vul_standaard_cover("geen json".to_string(), Some("kba"), Some(&dir)), "geen json");
    }

    #[test]
    fn upstream_detail_leest_fastapi_fouten() {
        assert_eq!(upstream_detail(r#"{"detail":"foto ontbreekt"}"#), "foto ontbreekt");
        assert_eq!(
            upstream_detail(r#"{"detail":[{"msg":"a"},{"msg":"b"}]}"#),
            "a; b"
        );
        // Zonder `msg` geen ruwe items (die kunnen de afbeelding als `input` bevatten).
        assert_eq!(
            upstream_detail(r#"{"detail":[{"msg":"a","input":"QUJD"},{"loc":["x"]}]}"#),
            "a"
        );
        // Geen JSON (HTML-foutpagina van een proxy): generieke tekst, niets doorlekken.
        assert_eq!(upstream_detail("<html>intern</html>"), "geen toelichting ontvangen");
        assert_eq!(upstream_detail(""), "geen toelichting ontvangen");
        let lang = upstream_detail(&format!(r#"{{"detail":"{}"}}"#, "x".repeat(2000)));
        assert!(lang.ends_with("...") && lang.chars().count() == MAX_DETAIL_TEKENS + 3);
    }

    /// Nep-Reports API: bewaart de ontvangen body en antwoordt met `status` + `antwoord`.
    async fn nep_reports_api(status: StatusCode, antwoord: &'static str) -> (String, Arc<Mutex<String>>) {
        let ontvangen = Arc::new(Mutex::new(String::new()));
        let opslag = ontvangen.clone();
        let app = axum::Router::new().route(
            "/api/generate/v2",
            axum::routing::post(move |body: String| {
                let opslag = opslag.clone();
                async move {
                    *opslag.lock().expect("lock") = body;
                    (status, [(header::CONTENT_TYPE, "application/json")], antwoord)
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let adres = listener.local_addr().expect("adres");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        (format!("http://{adres}"), ontvangen)
    }

    async fn state(url: String, dir: Option<PathBuf>) -> AppState {
        let db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        let mut state = AppState::new(
            db,
            Some(url),
            None,
            Some("ak-test".to_string()),
            openaec_cloud::TenantsRegistry::default(),
            None,
        );
        state.report_default_cover_dir = dir;
        state
    }

    fn kba() -> OidcClaims {
        OidcClaims {
            sub: "tester".to_string(),
            tenant: Some("kba".to_string()),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn afkeuring_4xx_komt_met_detailtekst_bij_gebruiker() {
        let (url, _) = nep_reports_api(
            StatusCode::UNPROCESSABLE_ENTITY,
            r#"{"detail":"static_elements: verplichte afbeelding ontbreekt"}"#,
        )
        .await;
        let fout = proxy_report(&state(url, None).await, &kba(), r#"{"cover":{}}"#.to_string())
            .await
            .expect_err("422");
        let resp = fout.into_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.expect("body");
        let json: Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(json["error"], "report_rejected");
        assert_eq!(
            json["detail"],
            "Rapport afgekeurd door de rapportengine: static_elements: verplichte afbeelding ontbreekt"
        );
    }

    #[tokio::test]
    async fn auth_fout_van_reports_api_blijft_serverfout() {
        // Een verlopen service-token mag bij de gebruiker geen 401 worden.
        for status in [StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN, StatusCode::NOT_FOUND] {
            let (url, _) = nep_reports_api(status, r#"{"detail":"x"}"#).await;
            let fout = proxy_report(&state(url, None).await, &kba(), "{}".to_string())
                .await
                .expect_err("fout");
            assert_eq!(fout.into_response().status(), StatusCode::BAD_GATEWAY, "{status}");
        }
    }

    #[tokio::test]
    async fn fout_5xx_blijft_report_service_error() {
        let (url, _) = nep_reports_api(StatusCode::INTERNAL_SERVER_ERROR, "boem").await;
        let fout = proxy_report(&state(url, None).await, &kba(), "{}".to_string())
            .await
            .expect_err("500");
        assert_eq!(fout.into_response().status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn proxy_stuurt_standaardfoto_mee_zonder_upload() {
        let dir = cover_map("proxy");
        let (url, ontvangen) = nep_reports_api(StatusCode::OK, "%PDF").await;
        proxy_report(&state(url, Some(dir)).await, &kba(), r#"{"cover":{}}"#.to_string())
            .await
            .expect("pdf");
        let body = ontvangen.lock().expect("lock").clone();
        assert_eq!(cover_image(&body)["media_type"], "image/jpeg");
    }
}
